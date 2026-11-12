use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use crate::history::History;

pub const BUILTIN_NAMES: &[&str] = &[
    "cd", "pwd", "export", "unset", "alias", "unalias", "history", "clear", "help", "exit",
    "source", "type", "true", "false",
];

pub fn is_builtin(name: &str) -> bool {
    BUILTIN_NAMES.contains(&name)
}

pub enum BuiltinOutcome {
    Executed(i32),
    ExitShell(i32),
    SourceFile(PathBuf),
}

pub struct ShellState {
    pub last_status: i32,
    pub previous_dir: Option<PathBuf>,
    pub aliases: HashMap<String, String>,
}

impl ShellState {
    pub fn new() -> Self {
        let mut aliases = HashMap::new();
        aliases.insert("ll".to_string(), "ls -lh --color=auto".to_string());
        aliases.insert("la".to_string(), "ls -lah --color=auto".to_string());
        aliases.insert("ls".to_string(), "ls --color=auto".to_string());
        aliases.insert("grep".to_string(), "grep --color=auto".to_string());

        Self {
            last_status: 0,
            previous_dir: None,
            aliases,
        }
    }

    pub fn alias_names(&self) -> std::collections::HashSet<String> {
        self.aliases.keys().cloned().collect()
    }
}

pub fn run_builtin(
    argv: &[String],
    state: &mut ShellState,
    history: &History,
) -> Option<BuiltinOutcome> {
    let cmd = argv.first()?.as_str();
    match cmd {
        "true" => Some(BuiltinOutcome::Executed(0)),
        "false" => Some(BuiltinOutcome::Executed(1)),
        "pwd" => {
            match env::current_dir() {
                Ok(dir) => {
                    println!("{}", dir.display());
                    Some(BuiltinOutcome::Executed(0))
                }
                Err(err) => {
                    eprintln!("gull-sh: pwd: {}", err);
                    Some(BuiltinOutcome::Executed(1))
                }
            }
        }
        "cd" => {
            let current = env::current_dir().ok();
            let target = match argv.get(1).map(|s| s.as_str()) {
                None | Some("~") => PathBuf::from(env::var("HOME").unwrap_or_else(|_| "/".into())),
                Some("-") => match &state.previous_dir {
                    Some(prev) => {
                        println!("{}", prev.display());
                        prev.clone()
                    }
                    None => {
                        eprintln!("gull-sh: cd: OLDPWD not set");
                        return Some(BuiltinOutcome::Executed(1));
                    }
                },
                Some(path) => PathBuf::from(path),
            };

            match env::set_current_dir(&target) {
                Ok(()) => {
                    if let Some(prev) = current {
                        env::set_var("OLDPWD", &prev);
                        state.previous_dir = Some(prev);
                    }
                    if let Ok(new_cwd) = env::current_dir() {
                        env::set_var("PWD", new_cwd);
                    }
                    Some(BuiltinOutcome::Executed(0))
                }
                Err(err) => {
                    eprintln!("gull-sh: cd: {}: {}", target.display(), err);
                    Some(BuiltinOutcome::Executed(1))
                }
            }
        }
        "export" => {
            if argv.len() == 1 {
                let mut vars: Vec<(String, String)> = env::vars().collect();
                vars.sort_by(|a, b| a.0.cmp(&b.0));
                for (k, v) in vars {
                    println!("export {}=\"{}\"", k, v);
                }
                return Some(BuiltinOutcome::Executed(0));
            }
            for arg in &argv[1..] {
                if let Some(eq) = arg.find('=') {
                    let key = &arg[..eq];
                    let val = &arg[eq + 1..];
                    if key.is_empty() {
                        eprintln!("gull-sh: export: invalid identifier `{}`", arg);
                        return Some(BuiltinOutcome::Executed(1));
                    }
                    env::set_var(key, val);
                } else if let Ok(existing) = env::var(arg) {
                    env::set_var(arg, existing);
                } else {
                    env::set_var(arg, "");
                }
            }
            Some(BuiltinOutcome::Executed(0))
        }
        "unset" => {
            for arg in &argv[1..] {
                env::remove_var(arg);
            }
            Some(BuiltinOutcome::Executed(0))
        }
        "alias" => {
            if argv.len() == 1 {
                let mut entries: Vec<(&String, &String)> = state.aliases.iter().collect();
                entries.sort_by(|a, b| a.0.cmp(b.0));
                for (k, v) in entries {
                    println!("alias {}='{}'", k, v);
                }
                return Some(BuiltinOutcome::Executed(0));
            }
            for arg in &argv[1..] {
                if let Some(eq) = arg.find('=') {
                    let name = arg[..eq].trim().to_string();
                    let value = arg[eq + 1..].trim_matches('\'').trim_matches('"').to_string();
                    state.aliases.insert(name, value);
                } else if let Some(val) = state.aliases.get(arg) {
                    println!("alias {}='{}'", arg, val);
                } else {
                    eprintln!("gull-sh: alias: {}: not found", arg);
                    return Some(BuiltinOutcome::Executed(1));
                }
            }
            Some(BuiltinOutcome::Executed(0))
        }
        "unalias" => {
            for arg in &argv[1..] {
                state.aliases.remove(arg);
            }
            Some(BuiltinOutcome::Executed(0))
        }
        "history" => {
            for (idx, line) in history.entries().iter().enumerate() {
                println!("{:>5}  {}", idx + 1, line);
            }
            Some(BuiltinOutcome::Executed(0))
        }
        "clear" => {
            print!("\x1b[2J\x1b[H");
            let _ = std::io::Write::flush(&mut std::io::stdout());
            Some(BuiltinOutcome::Executed(0))
        }
        "type" => {
            let mut status = 0;
            for arg in &argv[1..] {
                if let Some(alias_target) = state.aliases.get(arg) {
                    println!("{} is aliased to `{}`", arg, alias_target);
                } else if is_builtin(arg) {
                    println!("{} is a gull-sh builtin", arg);
                } else if let Some(found) = resolve_in_path(arg) {
                    println!("{} is {}", arg, found.display());
                } else {
                    eprintln!("gull-sh: type: {}: not found", arg);
                    status = 1;
                }
            }
            Some(BuiltinOutcome::Executed(status))
        }
        "help" => {
            println!("\x1b[38;2;203;166;247mGullSH 0.1.0\x1b[0m — GullOS Interactive Shell (Catppuccin Mocha)");
            println!("  Live Highlighting : \x1b[38;2;166;227;161mvalid-cmd\x1b[0m | \x1b[38;2;243;139;168minvalid-cmd\x1b[0m | \x1b[38;2;250;179;135m--flags\x1b[0m | \x1b[38;2;249;226;175m\"strings\"\x1b[0m");
            println!("  Tab               : Complete commands in $PATH, builtins, and filesystem paths");
            println!("  Right / Ctrl+F    : Accept inline Fish-style history autosuggestion");
            println!("  Up / Down         : Navigate persistent ~/.gull_history");
            println!("  Builtins          : {}", BUILTIN_NAMES.join(", "));
            Some(BuiltinOutcome::Executed(0))
        }
        "source" | "." => {
            let Some(target) = argv.get(1) else {
                eprintln!("gull-sh: source: filename argument required");
                return Some(BuiltinOutcome::Executed(2));
            };
            Some(BuiltinOutcome::SourceFile(PathBuf::from(target)))
        }
        "exit" => {
            let code = argv
                .get(1)
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(state.last_status);
            Some(BuiltinOutcome::ExitShell(code))
        }
        _ => None,
    }
}

fn resolve_in_path(cmd: &str) -> Option<PathBuf> {
    let path_var = env::var("PATH").ok()?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
