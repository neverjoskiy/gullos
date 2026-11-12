mod app;
mod font;
mod grid;
mod pty;
mod renderer;
mod theme;

use std::env;
use std::process::Command;

use app::GullTermApp;
use pty::{resolve_default_shell, TermEvent};
use winit::event_loop::EventLoop;

fn main() {
    let cli = parse_cli_args();
    if cli.print_version {
        println!("gull-term 0.1.0 (GullOS Wayland + TTY Fallback / Catppuccin Mocha)");
        return;
    }

    // Probe Wayland compositor connectivity BEFORE initializing winit.
    // If launched outside Wayland (e.g., on /dev/tty2 raw Linux console), gracefully
    // transition into native TTY shell mode without panicking or crashing.
    if cli.force_tty || !wayland_compositor_reachable() {
        exec_native_tty_fallback(cli.override_command);
    }

    let event_loop = match EventLoop::<TermEvent>::with_user_event().build() {
        Ok(el) => el,
        Err(err) => {
            eprintln!(
                "\x1b[38;2;249;226;175m[gull-term]\x1b[0m Wayland display unavailable ({}); switching to native TTY shell.",
                err
            );
            exec_native_tty_fallback(cli.override_command);
        }
    };

    let proxy = event_loop.create_proxy();
    let mut app = GullTermApp::new(proxy, cli.override_command);
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("gull-term: Wayland event loop exited with error: {}", err);
        std::process::exit(1);
    }
}

struct CliArgs {
    print_version: bool,
    force_tty: bool,
    override_command: Option<(String, Vec<String>)>,
}

fn parse_cli_args() -> CliArgs {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut print_version = false;
    let mut force_tty = false;
    let mut override_command = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--version" | "-v" => {
                print_version = true;
                i += 1;
            }
            "--tty" => {
                force_tty = true;
                i += 1;
            }
            "-e" | "--command" => {
                if i + 1 < args.len() {
                    let prog = args[i + 1].clone();
                    let rest = args[i + 2..].to_vec();
                    override_command = Some((prog, rest));
                }
                break;
            }
            _ => {
                i += 1;
            }
        }
    }

    CliArgs {
        print_version,
        force_tty,
        override_command,
    }
}

fn wayland_compositor_reachable() -> bool {
    let has_display_var = env::var_os("WAYLAND_DISPLAY")
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    let has_socket_fd = env::var_os("WAYLAND_SOCKET")
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    if !has_display_var && !has_socket_fd {
        return false;
    }

    wayland_client::Connection::connect_to_env().is_ok()
}

fn exec_native_tty_fallback(override_command: Option<(String, Vec<String>)>) -> ! {
    let (prog, args) = override_command.unwrap_or_else(|| (resolve_default_shell(), Vec::new()));

    eprintln!(
        "\x1b[38;2;203;166;247m[gull-term]\x1b[0m \x1b[38;2;166;173;200mNo Wayland compositor detected — running \x1b[38;2;166;227;161m{}\x1b[38;2;166;173;200m natively on TTY.\x1b[0m",
        prog
    );

    if env::var_os("TERM").is_none() {
        env::set_var("TERM", "linux");
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = Command::new(&prog).args(&args).exec();
        eprintln!("gull-term: failed to exec `{}`: {}", prog, err);
        let fallback_err = Command::new("/bin/sh").exec();
        eprintln!("gull-term: failed to exec `/bin/sh`: {}", fallback_err);
        std::process::exit(127);
    }

    #[cfg(not(unix))]
    {
        let status = Command::new(&prog)
            .args(&args)
            .status()
            .map(|s| s.code().unwrap_or(1))
            .unwrap_or(127);
        std::process::exit(status);
    }
}
