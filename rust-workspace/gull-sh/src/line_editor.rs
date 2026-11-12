use std::collections::HashSet;
use std::env;
use std::io::{self, Write};

use crossterm::cursor::MoveToColumn;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::queue;
use crossterm::style::{Print, ResetColor, SetForegroundColor};
use crossterm::terminal::{self, Clear, ClearType};

use crate::completer::{complete_at_cursor, longest_common_prefix};
use crate::highlighter::{highlight_line, mocha, PathIndex};
use crate::history::History;

pub enum ReadOutcome {
    Line(String),
    Interrupted,
    Eof,
}

struct RawModeGuard;

impl RawModeGuard {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

pub fn read_interactive_line(
    last_status: i32,
    history: &History,
    path_index: &mut PathIndex,
    aliases: &HashSet<String>,
) -> io::Result<ReadOutcome> {
    path_index.refresh_if_needed();
    let (prompt_ansi, prompt_width) = build_prompt(last_status);

    let _raw_guard = RawModeGuard::enter()?;
    let mut stdout = io::stdout();

    let mut buffer = String::new();
    let mut cursor = 0usize; // byte offset in `buffer`
    let mut history_pos: Option<usize> = None;
    let mut saved_draft = String::new();
    let mut tab_press_count = 0u8;

    render_line(
        &mut stdout,
        &prompt_ansi,
        prompt_width,
        &buffer,
        cursor,
        history,
        path_index,
        aliases,
    )?;

    loop {
        let ev = event::read()?;
        let Event::Key(KeyEvent {
            code,
            modifiers,
            kind,
            ..
        }) = ev
        else {
            continue;
        };

        if kind == KeyEventKind::Release {
            continue;
        }

        if code != KeyCode::Tab {
            tab_press_count = 0;
        }

        match (code, modifiers) {
            (KeyCode::Enter, _) | (KeyCode::Char('j'), KeyModifiers::CONTROL) | (KeyCode::Char('m'), KeyModifiers::CONTROL) => {
                // Re-render without ghost autosuggestion before committing newline
                render_committed_line(
                    &mut stdout,
                    &prompt_ansi,
                    &buffer,
                    path_index,
                    aliases,
                )?;
                write!(stdout, "\r\n")?;
                stdout.flush()?;
                return Ok(ReadOutcome::Line(buffer));
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                write!(stdout, "^C\r\n")?;
                stdout.flush()?;
                return Ok(ReadOutcome::Interrupted);
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                if buffer.is_empty() {
                    write!(stdout, "\r\n")?;
                    stdout.flush()?;
                    return Ok(ReadOutcome::Eof);
                } else if cursor < buffer.len() {
                    let next_len = buffer[cursor..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                    buffer.drain(cursor..cursor + next_len);
                }
            }
            (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                queue!(stdout, Clear(ClearType::All), crossterm::cursor::MoveTo(0, 0))?;
            }
            (KeyCode::Char('a'), KeyModifiers::CONTROL) | (KeyCode::Home, _) => {
                cursor = 0;
            }
            (KeyCode::Char('e'), KeyModifiers::CONTROL) | (KeyCode::End, _) => {
                if cursor == buffer.len() {
                    if let Some(ghost) = history.suggest_suffix(&buffer) {
                        buffer.push_str(&ghost);
                    }
                }
                cursor = buffer.len();
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                buffer.drain(..cursor);
                cursor = 0;
            }
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                buffer.truncate(cursor);
            }
            (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                let new_cursor = prev_word_boundary(&buffer, cursor);
                buffer.drain(new_cursor..cursor);
                cursor = new_cursor;
            }
            (KeyCode::Backspace, _) | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                if cursor > 0 {
                    let prev_idx = prev_char_boundary(&buffer, cursor);
                    buffer.drain(prev_idx..cursor);
                    cursor = prev_idx;
                }
            }
            (KeyCode::Delete, _) => {
                if cursor < buffer.len() {
                    let next_len = buffer[cursor..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                    buffer.drain(cursor..cursor + next_len);
                }
            }
            (KeyCode::Left, _) | (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                if cursor > 0 {
                    cursor = prev_char_boundary(&buffer, cursor);
                }
            }
            (KeyCode::Right, _) | (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                if cursor < buffer.len() {
                    let next_len = buffer[cursor..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                    cursor += next_len;
                } else if let Some(ghost) = history.suggest_suffix(&buffer) {
                    buffer.push_str(&ghost);
                    cursor = buffer.len();
                }
            }
            (KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => {
                let entries = history.entries();
                if !entries.is_empty() {
                    match history_pos {
                        None => {
                            saved_draft = buffer.clone();
                            let idx = entries.len() - 1;
                            history_pos = Some(idx);
                            buffer = entries[idx].clone();
                            cursor = buffer.len();
                        }
                        Some(idx) if idx > 0 => {
                            let next_idx = idx - 1;
                            history_pos = Some(next_idx);
                            buffer = entries[next_idx].clone();
                            cursor = buffer.len();
                        }
                        _ => {}
                    }
                }
            }
            (KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                let entries = history.entries();
                if let Some(idx) = history_pos {
                    if idx + 1 < entries.len() {
                        let next_idx = idx + 1;
                        history_pos = Some(next_idx);
                        buffer = entries[next_idx].clone();
                        cursor = buffer.len();
                    } else {
                        history_pos = None;
                        buffer = saved_draft.clone();
                        cursor = buffer.len();
                    }
                }
            }
            (KeyCode::Tab, _) => {
                tab_press_count = tab_press_count.saturating_add(1);
                let comp = complete_at_cursor(&buffer, cursor, path_index, aliases);
                if comp.candidates.len() == 1 {
                    let mut replacement = comp.candidates[0].clone();
                    if !replacement.ends_with('/') {
                        replacement.push(' ');
                    }
                    buffer.replace_range(comp.replace_start..comp.replace_end, &replacement);
                    cursor = comp.replace_start + replacement.len();
                    tab_press_count = 0;
                } else if comp.candidates.len() > 1 {
                    let lcp = longest_common_prefix(&comp.candidates);
                    let current_slice = &buffer[comp.replace_start..comp.replace_end];
                    if lcp.len() > current_slice.len() {
                        buffer.replace_range(comp.replace_start..comp.replace_end, &lcp);
                        cursor = comp.replace_start + lcp.len();
                    } else if tab_press_count >= 1 {
                        print_completion_grid(&mut stdout, &comp.candidates)?;
                    }
                }
            }
            (KeyCode::Char(ch), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                buffer.insert(cursor, ch);
                cursor += ch.len_utf8();
            }
            _ => {}
        }

        render_line(
            &mut stdout,
            &prompt_ansi,
            prompt_width,
            &buffer,
            cursor,
            history,
            path_index,
            aliases,
        )?;
    }
}

#[allow(clippy::too_many_arguments)]
fn render_line(
    stdout: &mut io::Stdout,
    prompt_ansi: &str,
    prompt_width: usize,
    buffer: &str,
    cursor: usize,
    history: &History,
    path_index: &PathIndex,
    aliases: &HashSet<String>,
) -> io::Result<()> {
    queue!(
        stdout,
        MoveToColumn(0),
        Clear(ClearType::CurrentLine),
        Print(prompt_ansi)
    )?;

    for seg in highlight_line(buffer, path_index, aliases) {
        queue!(stdout, SetForegroundColor(seg.color), Print(seg.text))?;
    }

    if cursor == buffer.len() {
        if let Some(ghost) = history.suggest_suffix(buffer) {
            queue!(stdout, SetForegroundColor(mocha::OVERLAY0), Print(ghost))?;
        }
    }

    let cursor_chars = buffer[..cursor].chars().count();
    let target_col = (prompt_width + cursor_chars) as u16;
    queue!(stdout, ResetColor, MoveToColumn(target_col))?;
    stdout.flush()
}

fn render_committed_line(
    stdout: &mut io::Stdout,
    prompt_ansi: &str,
    buffer: &str,
    path_index: &PathIndex,
    aliases: &HashSet<String>,
) -> io::Result<()> {
    queue!(
        stdout,
        MoveToColumn(0),
        Clear(ClearType::CurrentLine),
        Print(prompt_ansi)
    )?;
    for seg in highlight_line(buffer, path_index, aliases) {
        queue!(stdout, SetForegroundColor(seg.color), Print(seg.text))?;
    }
    queue!(stdout, ResetColor)?;
    Ok(())
}

fn print_completion_grid(stdout: &mut io::Stdout, candidates: &[String]) -> io::Result<()> {
    let term_cols = terminal::size().map(|(w, _)| w as usize).unwrap_or(80).max(40);
    let max_display = candidates.len().min(48);
    let shown = &candidates[..max_display];

    let col_width = shown
        .iter()
        .map(|s| s.chars().count())
        .max()
        .unwrap_or(8)
        + 3;
    let per_row = (term_cols / col_width).max(1);

    write!(stdout, "\r\n")?;
    for (idx, item) in shown.iter().enumerate() {
        if item.ends_with('/') {
            queue!(stdout, SetForegroundColor(mocha::BLUE))?;
        } else {
            queue!(stdout, SetForegroundColor(mocha::SUBTEXT0))?;
        }
        let pad = col_width.saturating_sub(item.chars().count());
        queue!(stdout, Print(item), ResetColor, Print(" ".repeat(pad)))?;
        if (idx + 1) % per_row == 0 && idx + 1 < shown.len() {
            write!(stdout, "\r\n")?;
        }
    }
    if candidates.len() > max_display {
        write!(
            stdout,
            "\r\n\x1b[38;2;108;112;134m... and {} more matches\x1b[0m",
            candidates.len() - max_display
        )?;
    }
    write!(stdout, "\r\n")?;
    stdout.flush()
}

fn build_prompt(last_status: i32) -> (String, usize) {
    let user = env::var("USER").unwrap_or_else(|_| "gull".to_string());
    let host = read_hostname();
    let cwd = compact_cwd();

    let sigil_rgb = if last_status == 0 {
        "\x1b[38;2;203;166;247m" // Mauve
    } else {
        "\x1b[38;2;243;139;168m" // Red
    };

    let ansi = format!(
        "\x1b[38;2;166;227;161m{}\x1b[38;2;108;112;134m@\x1b[38;2;137;180;250m{} \x1b[38;2;180;190;254m{} {}❯\x1b[0m ",
        user, host, cwd, sigil_rgb
    );
    let visible_width = user.chars().count() + 1 + host.chars().count() + 1 + cwd.chars().count() + 3;
    (ansi, visible_width)
}

fn read_hostname() -> String {
    if let Ok(h) = std::fs::read_to_string("/etc/hostname") {
        let trimmed = h.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    "gull-pc".to_string()
}

fn compact_cwd() -> String {
    let cwd = env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "?".to_string());
    if let Ok(home) = env::var("HOME") {
        if cwd == home {
            return "~".to_string();
        }
        if let Some(rest) = cwd.strip_prefix(&format!("{}/", home)) {
            return format!("~/{}", rest);
        }
    }
    cwd
}

fn prev_char_boundary(s: &str, byte_idx: usize) -> usize {
    s[..byte_idx]
        .char_indices()
        .next_back()
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn prev_word_boundary(s: &str, cursor: usize) -> usize {
    let slice = &s[..cursor];
    let trimmed = slice.trim_end();
    if trimmed.is_empty() {
        return 0;
    }
    trimmed
        .rfind(|c: char| c.is_whitespace() || c == '/')
        .map(|i| i + 1)
        .unwrap_or(0)
}
