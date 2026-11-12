use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;

use crossterm::style::Color;

use crate::builtins::is_builtin;
use crate::lexer::{is_valid_assignment, scan_spans, OpKind, SpanKind};

/// Canonical Catppuccin Mocha 24-bit palette for GullSH
pub mod mocha {
    use crossterm::style::Color;

    pub const TEXT: Color = Color::Rgb { r: 205, g: 214, b: 244 };     // #cdd6f4
    pub const SUBTEXT0: Color = Color::Rgb { r: 166, g: 173, b: 200 }; // #a6adc8
    pub const OVERLAY0: Color = Color::Rgb { r: 108, g: 112, b: 134 }; // #6c7086
    pub const MAUVE: Color = Color::Rgb { r: 203, g: 166, b: 247 };    // #cba6f7
    pub const LAVENDER: Color = Color::Rgb { r: 180, g: 190, b: 254 }; // #b4befe
    pub const BLUE: Color = Color::Rgb { r: 137, g: 180, b: 250 };     // #89b4fa
    pub const SKY: Color = Color::Rgb { r: 137, g: 220, b: 235 };      // #89dceb
    pub const TEAL: Color = Color::Rgb { r: 148, g: 226, b: 213 };     // #94e2d5
    pub const GREEN: Color = Color::Rgb { r: 166, g: 227, b: 161 };    // #a6e3a1
    pub const YELLOW: Color = Color::Rgb { r: 249, g: 226, b: 175 };   // #f9e2af
    pub const PEACH: Color = Color::Rgb { r: 250, g: 179, b: 135 };    // #fab387
    pub const RED: Color = Color::Rgb { r: 243, g: 139, b: 168 };      // #f38ba8
}

#[derive(Debug, Clone)]
pub struct StyledSegment {
    pub text: String,
    pub color: Color,
}

pub struct PathIndex {
    executables: HashSet<String>,
    cached_path_env: String,
}

impl PathIndex {
    pub fn new() -> Self {
        let mut idx = Self {
            executables: HashSet::new(),
            cached_path_env: String::new(),
        };
        idx.refresh_if_needed();
        idx
    }

    pub fn refresh_if_needed(&mut self) {
        let current_path = env::var("PATH").unwrap_or_default();
        if current_path == self.cached_path_env && !self.executables.is_empty() {
            return;
        }
        self.cached_path_env = current_path.clone();
        self.executables.clear();

        for dir in env::split_paths(&current_path) {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Ok(ft) = entry.file_type() {
                        if !ft.is_dir() && is_executable_path(&entry.path()) {
                            if let Some(name) = entry.file_name().to_str() {
                                self.executables.insert(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn contains_command(&self, cmd: &str, aliases: &HashSet<String>) -> bool {
        if is_builtin(cmd) || aliases.contains(cmd) {
            return true;
        }
        if cmd.contains('/') {
            let expanded = if let Some(stripped) = cmd.strip_prefix("~/") {
                if let Ok(home) = env::var("HOME") {
                    format!("{}/{}", home, stripped)
                } else {
                    cmd.to_string()
                }
            } else {
                cmd.to_string()
            };
            let p = Path::new(&expanded);
            return p.is_file() && is_executable_path(p);
        }
        self.executables.contains(cmd)
    }

    pub fn all_commands<'a>(&'a self, aliases: &'a HashSet<String>) -> Vec<String> {
        let mut set: HashSet<String> = self.executables.clone();
        for b in crate::builtins::BUILTIN_NAMES {
            set.insert((*b).to_string());
        }
        for a in aliases {
            set.insert(a.clone());
        }
        let mut list: Vec<String> = set.into_iter().collect();
        list.sort();
        list
    }
}

#[cfg(unix)]
fn is_executable_path(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|m| m.is_file() && (m.permissions().mode() & 0o111 != 0))
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable_path(path: &Path) -> bool {
    path.is_file()
}

/// Highlights an interactive input line into Catppuccin Mocha colored segments.
pub fn highlight_line(
    input: &str,
    path_index: &PathIndex,
    aliases: &HashSet<String>,
) -> Vec<StyledSegment> {
    if input.is_empty() {
        return Vec::new();
    }

    let spans = scan_spans(input, 0);
    let mut segments = Vec::new();
    let mut cursor = 0usize;
    let mut expecting_command = true;
    let mut expecting_redir_target = false;

    for span in &spans {
        if span.start > cursor {
            segments.push(StyledSegment {
                text: input[cursor..span.start].to_string(),
                color: mocha::TEXT,
            });
        }

        let color = match span.kind {
            SpanKind::Comment => mocha::OVERLAY0,
            SpanKind::SingleQuoted | SpanKind::DoubleQuoted => {
                expecting_redir_target = false;
                expecting_command = false;
                mocha::YELLOW
            }
            SpanKind::Operator(op) => match op {
                OpKind::RedirIn | OpKind::RedirOut | OpKind::RedirApp => {
                    expecting_redir_target = true;
                    mocha::SKY
                }
                OpKind::Pipe | OpKind::AndIf | OpKind::OrIf | OpKind::Semi | OpKind::Background => {
                    expecting_command = true;
                    expecting_redir_target = false;
                    mocha::SKY
                }
            },
            SpanKind::Word => {
                if expecting_redir_target {
                    expecting_redir_target = false;
                    mocha::TEAL
                } else if expecting_command && is_valid_assignment(&span.raw) {
                    mocha::MAUVE
                } else if expecting_command {
                    // Wrapper commands like `sudo` or `env` keep highlighting the next word as a command
                    if matches!(span.value.as_str(), "sudo" | "time" | "exec" | "nohup") {
                        expecting_command = true;
                        mocha::GREEN
                    } else {
                        expecting_command = false;
                        if path_index.contains_command(&span.value, aliases) {
                            mocha::GREEN
                        } else {
                            mocha::RED
                        }
                    }
                } else if span.raw.starts_with('$') {
                    mocha::MAUVE
                } else if span.raw.starts_with('-') {
                    mocha::PEACH
                } else if looks_like_existing_path(&span.value) {
                    mocha::LAVENDER
                } else {
                    mocha::TEXT
                }
            }
        };

        segments.push(StyledSegment {
            text: input[span.start..span.end].to_string(),
            color,
        });
        cursor = span.end;
    }

    if cursor < input.len() {
        segments.push(StyledSegment {
            text: input[cursor..].to_string(),
            color: mocha::TEXT,
        });
    }

    segments
}

fn looks_like_existing_path(token: &str) -> bool {
    if token.contains('/') || token.starts_with('.') || token.starts_with('~') {
        return Path::new(token).exists();
    }
    false
}
