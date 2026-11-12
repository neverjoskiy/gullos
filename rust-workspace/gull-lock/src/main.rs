mod auth;
mod render;
mod wayland;

use std::env;
use std::io::{self, Write};

use auth::{interactive_set_password, LockConfig};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Some(first) = args.first().map(|s| s.as_str()) {
        match first {
            "--version" | "-v" => {
                println!("gull-lock 0.1.0 (GullOS Wayland Lockscreen / Catppuccin Mocha)");
                return;
            }
            "--help" | "-h" => {
                println!("Usage: gull-lock [--set-password | --version]");
                println!("Locks the active Wayland session using ext-session-lock-v1.");
                return;
            }
            "--set-password" => {
                if let Err(err) = interactive_set_password() {
                    eprintln!("gull-lock: failed to update password: {}", err);
                    std::process::exit(1);
                }
                return;
            }
            _ => {}
        }
    }

    let config = LockConfig::load();

    // Ignore SIGINT / SIGQUIT / SIGTSTP so Ctrl+C / Ctrl+Z cannot bypass lock
    unsafe {
        libc::signal(libc::SIGINT, libc::SIG_IGN);
        libc::signal(libc::SIGQUIT, libc::SIG_IGN);
        libc::signal(libc::SIGTSTP, libc::SIG_IGN);
    }

    let has_wayland = env::var_os("WAYLAND_DISPLAY")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
        || env::var_os("WAYLAND_SOCKET")
            .map(|v| !v.is_empty())
            .unwrap_or(false);

    if !has_wayland {
        // Fallback console lock if invoked outside a Wayland compositor
        run_tty_lock(&config);
        return;
    }

    if let Err(err) = wayland::run_wayland_lock(config) {
        eprintln!("gull-lock: {}", err);
        std::process::exit(1);
    }
}

fn run_tty_lock(config: &LockConfig) {
    let mut stdout = io::stdout();
    loop {
        print!(
            "\x1b[2J\x1b[H\x1b[38;2;203;166;247mGullOS TTY Lock\x1b[0m ({}@{})\nPassword: ",
            config.user, config.hostname
        );
        let _ = stdout.flush();

        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_ok() {
            let candidate = line.trim_end_matches(&['\r', '\n'][..]);
            if config.verify(candidate) {
                println!("\x1b[38;2;166;227;161mUnlocked.\x1b[0m");
                break;
            } else {
                println!("\x1b[38;2;243;139;168mIncorrect password.\x1b[0m");
                std::thread::sleep(std::time::Duration::from_millis(900));
            }
        }
    }
}
