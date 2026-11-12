mod builtins;
mod completer;
mod executor;
mod highlighter;
mod history;
mod lexer;
mod line_editor;

use std::env;
use std::io::{self, BufRead, IsTerminal};
use std::path::Path;
use std::process::Command;

use builtins::ShellState;
use executor::{execute_line, execute_script_file, ExecControl};
use highlighter::PathIndex;
use history::History;
use line_editor::{read_interactive_line, ReadOutcome};

fn main() {
    install_shell_signal_handlers();

    let raw_args: Vec<String> = env::args().collect();
    let is_login_shell = raw_args
        .first()
        .map(|arg0| arg0.starts_with('-'))
        .unwrap_or(false)
        || raw_args.iter().any(|a| a == "-l" || a == "--login");

    ensure_default_environment();

    // If invoked as a login shell directly on /dev/tty1 with no Wayland session running,
    // launch the GullOS Wayland desktop session (`gull-session`) automatically.
    if is_login_shell && raw_args.len() == 1 {
        maybe_autostart_wayland_on_tty1();
    }

    let mut state = ShellState::new();
    let mut history = History::load();

    // Handle `-c "<command>"` one-shot execution (standard POSIX shell contract)
    let mut idx = 1;
    while idx < raw_args.len() {
        match raw_args[idx].as_str() {
            "-l" | "--login" | "-i" => {
                idx += 1;
            }
            "-c" => {
                let Some(cmd_str) = raw_args.get(idx + 1) else {
                    eprintln!("gull-sh: -c: option requires an argument");
                    std::process::exit(2);
                };
                match execute_line(cmd_str, &mut state, &history) {
                    ExecControl::Continue(code) | ExecControl::Exit(code) => {
                        std::process::exit(code);
                    }
                }
            }
            "--version" | "-v" => {
                println!("gull-sh 0.1.0 (GullOS / Catppuccin Mocha)");
                std::process::exit(0);
            }
            script_arg if !script_arg.starts_with('-') => {
                let code = match execute_script_file(Path::new(script_arg), &mut state, &history) {
                    ExecControl::Continue(c) | ExecControl::Exit(c) => c,
                };
                std::process::exit(code);
            }
            _ => {
                idx += 1;
            }
        }
    }

    // Source user RC if present (~/.gullrc)
    if let Ok(home) = env::var("HOME") {
        let rc_path = Path::new(&home).join(".gullrc");
        if rc_path.is_file() {
            let _ = execute_script_file(&rc_path, &mut state, &history);
        }
    }

    // Non-TTY stdin pipe execution
    if !io::stdin().is_terminal() {
        let stdin = io::stdin();
        for line in stdin.lock().lines().map_while(Result::ok) {
            match execute_line(&line, &mut state, &history) {
                ExecControl::Continue(_) => {}
                ExecControl::Exit(code) => std::process::exit(code),
            }
        }
        std::process::exit(state.last_status);
    }

    print_welcome_banner();

    let mut path_index = PathIndex::new();
    loop {
        let aliases = state.alias_names();
        match read_interactive_line(state.last_status, &history, &mut path_index, &aliases) {
            Ok(ReadOutcome::Line(line)) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                history.add(trimmed);
                match execute_line(trimmed, &mut state, &history) {
                    ExecControl::Continue(_) => {
                        // Refresh $PATH cache if user ran pacman or export PATH=...
                        if trimmed.contains("pacman") || trimmed.contains("PATH") {
                            path_index = PathIndex::new();
                        }
                    }
                    ExecControl::Exit(code) => std::process::exit(code),
                }
            }
            Ok(ReadOutcome::Interrupted) => {
                state.last_status = 130;
            }
            Ok(ReadOutcome::Eof) => {
                println!("exit");
                std::process::exit(state.last_status);
            }
            Err(err) => {
                eprintln!("gull-sh: input error: {}", err);
                std::process::exit(1);
            }
        }
    }
}

fn ensure_default_environment() {
    if env::var_os("PATH").is_none() {
        env::set_var("PATH", "/usr/local/sbin:/usr/local/bin:/usr/bin:/bin:/sbin");
    }
    if env::var_os("SHELL").is_none() {
        env::set_var("SHELL", "/usr/bin/gull-sh");
    }
    if env::var_os("EDITOR").is_none() {
        env::set_var("EDITOR", "nano");
    }
    if env::var_os("COLORTERM").is_none() {
        env::set_var("COLORTERM", "truecolor");
    }
}

#[cfg(unix)]
fn install_shell_signal_handlers() {
    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_IGN);
        libc::signal(libc::SIGQUIT, libc::SIG_IGN);
    }
}

#[cfg(not(unix))]
fn install_shell_signal_handlers() {}

#[cfg(unix)]
fn maybe_autostart_wayland_on_tty1() {
    if env::var_os("WAYLAND_DISPLAY").is_some()
        || env::var_os("GULL_SESSION_ACTIVE").is_some()
    {
        return;
    }

    let current_tty = unsafe {
        let ptr = libc::ttyname(libc::STDIN_FILENO);
        if ptr.is_null() {
            String::new()
        } else {
            std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    };

    if current_tty == "/dev/tty1" && Path::new("/usr/bin/gull-session").is_file() {
        use std::os::unix::process::CommandExt;
        env::set_var("GULL_SESSION_ACTIVE", "1");
        let _ = Command::new("/usr/bin/gull-session").exec();
    }
}

#[cfg(not(unix))]
fn maybe_autostart_wayland_on_tty1() {}

fn print_welcome_banner() {
    println!(
        "\x1b[38;2;203;166;247m╭─ GullSH 0.1.0\x1b[0m \x1b[38;2;108;112;134m•\x1b[0m \x1b[38;2;137;180;250mGullOS x86_64\x1b[0m \x1b[38;2;108;112;134m(type \x1b[38;2;166;227;161mhelp\x1b[38;2;108;112;134m or \x1b[38;2;166;227;161msudo pacman -Sy <pkg>\x1b[38;2;108;112;134m)\x1b[0m"
    );
}
