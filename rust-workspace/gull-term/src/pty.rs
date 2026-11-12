use std::env;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::mpsc::Sender;
use std::thread;

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use winit::event_loop::EventLoopProxy;

#[derive(Debug)]
pub enum TermEvent {
    PtyOutput(Vec<u8>),
    PtyExited,
}

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    _child: Box<dyn Child + Send + Sync>,
}

impl PtySession {
    pub fn spawn(
        cols: u16,
        rows: u16,
        override_command: Option<(String, Vec<String>)>,
        proxy: EventLoopProxy<TermEvent>,
        fallback_tx: Sender<Vec<u8>>,
    ) -> Result<Self, String> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: rows.max(4),
                cols: cols.max(10),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| format!("failed to allocate PTY: {}", e))?;

        let mut cmd = if let Some((prog, args)) = override_command {
            let mut cb = CommandBuilder::new(prog);
            cb.args(args);
            cb
        } else {
            let shell = resolve_default_shell();
            CommandBuilder::new(shell)
        };

        if let Ok(cwd) = env::current_dir() {
            cmd.cwd(cwd);
        } else if let Ok(home) = env::var("HOME") {
            cmd.cwd(home);
        }

        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("TERM_PROGRAM", "gull-term");

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| format!("failed to spawn child shell in PTY: {}", e))?;
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("failed to clone PTY reader: {}", e))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| format!("failed to take PTY writer: {}", e))?;

        thread::Builder::new()
            .name("gull-term-pty-reader".to_string())
            .spawn(move || {
                let mut buf = [0u8; 8192];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => {
                            let _ = proxy.send_event(TermEvent::PtyExited);
                            break;
                        }
                        Ok(n) => {
                            let payload = buf[..n].to_vec();
                            if proxy.send_event(TermEvent::PtyOutput(payload.clone())).is_err() {
                                let _ = fallback_tx.send(payload);
                                break;
                            }
                        }
                        Err(_) => {
                            let _ = proxy.send_event(TermEvent::PtyExited);
                            break;
                        }
                    }
                }
            })
            .map_err(|e| format!("failed to start PTY reader thread: {}", e))?;

        Ok(Self {
            master: pair.master,
            writer,
            _child: child,
        })
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        if !bytes.is_empty() {
            let _ = self.writer.write_all(bytes);
            let _ = self.writer.flush();
        }
    }

    pub fn resize(&self, cols: u16, rows: u16, pixel_w: u16, pixel_h: u16) {
        let _ = self.master.resize(PtySize {
            rows: rows.max(4),
            cols: cols.max(10),
            pixel_width: pixel_w,
            pixel_height: pixel_h,
        });
    }
}

pub fn resolve_default_shell() -> String {
    if Path::new("/usr/bin/gull-sh").is_file() {
        return "/usr/bin/gull-sh".to_string();
    }
    if let Ok(shell_env) = env::var("SHELL") {
        if !shell_env.is_empty() && Path::new(&shell_env).is_file() {
            return shell_env;
        }
    }
    "/bin/sh".to_string()
}
