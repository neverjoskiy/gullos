use std::env;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

const MAX_HISTORY_ENTRIES: usize = 5000;

pub struct History {
    entries: Vec<String>,
    file_path: PathBuf,
}

impl History {
    pub fn load() -> Self {
        let file_path = default_history_path();
        let mut entries = Vec::new();

        if let Ok(file) = fs::File::open(&file_path) {
            let reader = BufReader::new(file);
            for line in reader.lines().map_while(Result::ok) {
                let trimmed = line.trim().to_string();
                if !trimmed.is_empty() {
                    entries.push(trimmed);
                }
            }
        }

        if entries.len() > MAX_HISTORY_ENTRIES {
            let drain_count = entries.len() - MAX_HISTORY_ENTRIES;
            entries.drain(0..drain_count);
        }

        Self { entries, file_path }
    }

    pub fn add(&mut self, command: &str) {
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return;
        }
        if self.entries.last().map(|s| s.as_str()) == Some(trimmed) {
            return;
        }
        self.entries.push(trimmed.to_string());

        if let Some(parent) = self.file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file_path)
        {
            let _ = writeln!(file, "{}", trimmed);
        }
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    /// Fish-style inline autosuggestion: returns the suffix of the most recent
    /// command starting with `prefix`.
    pub fn suggest_suffix(&self, prefix: &str) -> Option<String> {
        if prefix.is_empty() {
            return None;
        }
        for entry in self.entries.iter().rev() {
            if entry.len() > prefix.len() && entry.starts_with(prefix) {
                return Some(entry[prefix.len()..].to_string());
            }
        }
        None
    }
}

fn default_history_path() -> PathBuf {
    if let Ok(custom) = env::var("GULL_HISTFILE") {
        if !custom.is_empty() {
            return PathBuf::from(custom);
        }
    }
    let home = env::var("HOME").unwrap_or_else(|_| "/home/gull".to_string());
    PathBuf::from(home).join(".gull_history")
}
