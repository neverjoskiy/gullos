use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::highlighter::PathIndex;

#[derive(Debug, Clone)]
pub struct CompletionResult {
    /// Byte index in line where replacement begins
    pub replace_start: usize,
    /// Byte index in line where replacement ends
    pub replace_end: usize,
    /// Sorted completion candidates
    pub candidates: Vec<String>,
}

/// Computes Tab-completion candidates at the given cursor byte position.
pub fn complete_at_cursor(
    line: &str,
    cursor: usize,
    path_index: &PathIndex,
    aliases: &HashSet<String>,
) -> CompletionResult {
    let prefix_slice = &line[..cursor.min(line.len())];

    // Find start of current word token
    let mut word_start = prefix_slice.len();
    for (idx, ch) in prefix_slice.char_indices().rev() {
        if ch.is_whitespace() || matches!(ch, '|' | '&' | ';' | '<' | '>') {
            break;
        }
        word_start = idx;
    }

    let word = &prefix_slice[word_start..];
    let before_word = prefix_slice[..word_start].trim_end();

    let in_command_position = before_word.is_empty()
        || before_word.ends_with('|')
        || before_word.ends_with("&&")
        || before_word.ends_with("||")
        || before_word.ends_with(';')
        || before_word.split_whitespace().last() == Some("sudo");

    let candidates = if in_command_position && !word.contains('/') && !word.starts_with('.') && !word.starts_with('~') {
        let mut matches: Vec<String> = path_index
            .all_commands(aliases)
            .into_iter()
            .filter(|cmd| cmd.starts_with(word))
            .collect();
        // Also include matching subdirectories in CWD so users can type `dir/` quickly
        let mut dir_matches = complete_filesystem_path(word, true);
        matches.append(&mut dir_matches);
        matches.sort();
        matches.dedup();
        matches
    } else {
        complete_filesystem_path(word, false)
    };

    CompletionResult {
        replace_start: word_start,
        replace_end: cursor.min(line.len()),
        candidates,
    }
}

fn complete_filesystem_path(partial: &str, dirs_only: bool) -> Vec<String> {
    let home = env::var("HOME").unwrap_or_else(|_| "/home/gull".to_string());

    let (display_dir_prefix, search_dir, file_prefix) = if let Some(rest) = partial.strip_prefix("~/") {
        if let Some(slash) = rest.rfind('/') {
            let sub = &rest[..=slash];
            let base = &rest[slash + 1..];
            (
                format!("~/{}", sub),
                PathBuf::from(format!("{}/{}", home, sub)),
                base.to_string(),
            )
        } else {
            ("~/".to_string(), PathBuf::from(&home), rest.to_string())
        }
    } else if partial == "~" {
        return vec!["~/".to_string()];
    } else if let Some(slash) = partial.rfind('/') {
        let dir_part = &partial[..=slash];
        let base_part = &partial[slash + 1..];
        let fs_dir = if dir_part == "/" {
            PathBuf::from("/")
        } else {
            PathBuf::from(dir_part)
        };
        (dir_part.to_string(), fs_dir, base_part.to_string())
    } else {
        (
            String::new(),
            env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            partial.to_string(),
        )
    };

    let show_hidden = file_prefix.starts_with('.');
    let mut results = Vec::new();

    if let Ok(entries) = fs::read_dir(&search_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !show_hidden && name.starts_with('.') {
                continue;
            }
            if !name.starts_with(&file_prefix) {
                continue;
            }
            let path: PathBuf = entry.path();
            let is_dir = Path::new(&path).is_dir();
            if dirs_only && !is_dir {
                continue;
            }
            let escaped_name = escape_shell_filename(&name);
            if is_dir {
                results.push(format!("{}{}/", display_dir_prefix, escaped_name));
            } else {
                results.push(format!("{}{}", display_dir_prefix, escaped_name));
            }
        }
    }

    results.sort();
    results
}

fn escape_shell_filename(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch == ' ' {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Computes the longest common prefix across all candidates.
pub fn longest_common_prefix(items: &[String]) -> String {
    if items.is_empty() {
        return String::new();
    }
    let first = &items[0];
    let mut prefix_len = first.len();

    for item in &items[1..] {
        let mut matched = 0;
        for (a, b) in first.chars().zip(item.chars()) {
            if a == b {
                matched += a.len_utf8();
            } else {
                break;
            }
        }
        prefix_len = prefix_len.min(matched);
    }
    first[..prefix_len].to_string()
}
