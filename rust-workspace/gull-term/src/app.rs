use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver};

use softbuffer::{Context, Surface};
use vte::Parser;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::font::MonoFont;
use crate::grid::TerminalGrid;
use crate::pty::{PtySession, TermEvent};
use crate::renderer::{compute_grid_dimensions, render_grid_to_buffer};

pub struct GullTermApp {
    proxy: EventLoopProxy<TermEvent>,
    override_command: Option<(String, Vec<String>)>,
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    font: MonoFont,
    grid: TerminalGrid,
    vte_parser: Parser,
    pty: Option<PtySession>,
    fallback_rx: Option<Receiver<Vec<u8>>>,
    modifiers: ModifiersState,
    focused: bool,
    last_title: String,
}

impl GullTermApp {
    pub fn new(
        proxy: EventLoopProxy<TermEvent>,
        override_command: Option<(String, Vec<String>)>,
    ) -> Self {
        let font = MonoFont::load(15.0);
        let grid = TerminalGrid::new(96, 28);
        Self {
            proxy,
            override_command,
            window: None,
            surface: None,
            font,
            grid,
            vte_parser: Parser::new(),
            pty: None,
            fallback_rx: None,
            modifiers: ModifiersState::empty(),
            focused: true,
            last_title: "GullTerm — GullOS".to_string(),
        }
    }

    fn process_pty_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.vte_parser.advance(&mut self.grid, byte);
        }
        if let Some(ref win) = self.window {
            if self.grid.title != self.last_title {
                self.last_title = self.grid.title.clone();
                win.set_title(&self.last_title);
            }
            win.request_redraw();
        }
    }

    fn redraw(&mut self) {
        let (Some(ref win), Some(ref mut surface)) = (&self.window, &mut self.surface) else {
            return;
        };

        let size = win.inner_size();
        let width = size.width.max(64);
        let height = size.height.max(64);

        let (Some(nz_w), Some(nz_h)) = (NonZeroU32::new(width), NonZeroU32::new(height)) else {
            return;
        };

        if surface.resize(nz_w, nz_h).is_err() {
            return;
        }

        if let Ok(mut buffer) = surface.buffer_mut() {
            render_grid_to_buffer(
                &mut buffer,
                width as usize,
                height as usize,
                &self.grid,
                &mut self.font,
                self.focused,
            );
            let _ = buffer.present();
            self.grid.dirty = false;
        }
    }

    fn handle_resize(&mut self, pixel_w: u32, pixel_h: u32) {
        if pixel_w == 0 || pixel_h == 0 {
            return;
        }
        let (cols, rows) = compute_grid_dimensions(pixel_w, pixel_h, &self.font);
        if cols != self.grid.cols || rows != self.grid.rows {
            self.grid.resize(cols, rows);
            if let Some(ref pty) = self.pty {
                pty.resize(
                    cols as u16,
                    rows as u16,
                    pixel_w.min(u16::MAX as u32) as u16,
                    pixel_h.min(u16::MAX as u32) as u16,
                );
            }
        }
        if let Some(ref win) = self.window {
            win.request_redraw();
        }
    }

    fn translate_key_to_pty(&mut self, logical_key: &Key, text: Option<&str>) {
        let ctrl = self.modifiers.control_key();
        let alt = self.modifiers.alt_key();
        let shift = self.modifiers.shift_key();

        // Viewport scrollback navigation with Shift+PageUp / Shift+PageDown
        if shift {
            match logical_key {
                Key::Named(NamedKey::PageUp) => {
                    self.grid.scroll_viewport((self.grid.rows / 2).max(1) as i32);
                    if let Some(ref win) = self.window {
                        win.request_redraw();
                    }
                    return;
                }
                Key::Named(NamedKey::PageDown) => {
                    self.grid.scroll_viewport(-((self.grid.rows / 2).max(1) as i32));
                    if let Some(ref win) = self.window {
                        win.request_redraw();
                    }
                    return;
                }
                _ => {}
            }
        }

        let app_cursor = self.grid.app_cursor_keys;
        let mut seq: Vec<u8> = Vec::new();

        match logical_key {
            Key::Named(NamedKey::Enter) => seq.push(b'\r'),
            Key::Named(NamedKey::Backspace) => seq.push(if ctrl { 0x08 } else { 0x7f }),
            Key::Named(NamedKey::Tab) => {
                if shift {
                    seq.extend_from_slice(b"\x1b[Z");
                } else {
                    seq.push(b'\t');
                }
            }
            Key::Named(NamedKey::Escape) => seq.push(0x1b),
            Key::Named(NamedKey::Space) => {
                if ctrl {
                    seq.push(0x00);
                } else {
                    seq.push(b' ');
                }
            }
            Key::Named(NamedKey::ArrowUp) => {
                seq.extend_from_slice(if app_cursor { b"\x1bOA" } else { b"\x1b[A" });
            }
            Key::Named(NamedKey::ArrowDown) => {
                seq.extend_from_slice(if app_cursor { b"\x1bOB" } else { b"\x1b[B" });
            }
            Key::Named(NamedKey::ArrowRight) => {
                seq.extend_from_slice(if app_cursor { b"\x1bOC" } else { b"\x1b[C" });
            }
            Key::Named(NamedKey::ArrowLeft) => {
                seq.extend_from_slice(if app_cursor { b"\x1bOD" } else { b"\x1b[D" });
            }
            Key::Named(NamedKey::Home) => {
                seq.extend_from_slice(if app_cursor { b"\x1bOH" } else { b"\x1b[H" });
            }
            Key::Named(NamedKey::End) => {
                seq.extend_from_slice(if app_cursor { b"\x1bOF" } else { b"\x1b[F" });
            }
            Key::Named(NamedKey::Insert) => seq.extend_from_slice(b"\x1b[2~"),
            Key::Named(NamedKey::Delete) => seq.extend_from_slice(b"\x1b[3~"),
            Key::Named(NamedKey::PageUp) => seq.extend_from_slice(b"\x1b[5~"),
            Key::Named(NamedKey::PageDown) => seq.extend_from_slice(b"\x1b[6~"),
            Key::Named(NamedKey::F1) => seq.extend_from_slice(b"\x1bOP"),
            Key::Named(NamedKey::F2) => seq.extend_from_slice(b"\x1bOQ"),
            Key::Named(NamedKey::F3) => seq.extend_from_slice(b"\x1bOR"),
            Key::Named(NamedKey::F4) => seq.extend_from_slice(b"\x1bOS"),
            Key::Named(NamedKey::F5) => seq.extend_from_slice(b"\x1b[15~"),
            Key::Named(NamedKey::F6) => seq.extend_from_slice(b"\x1b[17~"),
            Key::Named(NamedKey::F7) => seq.extend_from_slice(b"\x1b[18~"),
            Key::Named(NamedKey::F8) => seq.extend_from_slice(b"\x1b[19~"),
            Key::Named(NamedKey::F9) => seq.extend_from_slice(b"\x1b[20~"),
            Key::Named(NamedKey::F10) => seq.extend_from_slice(b"\x1b[21~"),
            Key::Named(NamedKey::F11) => seq.extend_from_slice(b"\x1b[23~"),
            Key::Named(NamedKey::F12) => seq.extend_from_slice(b"\x1b[24~"),
            Key::Character(ch_str) => {
                if ctrl {
                    if let Some(c) = ch_str.chars().next() {
                        let lower = c.to_ascii_lowercase();
                        if lower.is_ascii_lowercase() {
                            if alt {
                                seq.push(0x1b);
                            }
                            seq.push((lower as u8) - b'a' + 1);
                        }
                    }
                } else {
                    if alt {
                        seq.push(0x1b);
                    }
                    if let Some(txt) = text {
                        seq.extend_from_slice(txt.as_bytes());
                    } else {
                        seq.extend_from_slice(ch_str.as_bytes());
                    }
                }
            }
            _ => {
                if let Some(txt) = text {
                    if !ctrl {
                        if alt {
                            seq.push(0x1b);
                        }
                        seq.extend_from_slice(txt.as_bytes());
                    }
                }
            }
        }

        if !seq.is_empty() {
            self.grid.scroll_offset = 0;
            if let Some(ref mut pty) = self.pty {
                pty.write_bytes(&seq);
            }
        }
    }
}

impl ApplicationHandler<TermEvent> for GullTermApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let initial_w = (self.grid.cols * self.font.cell_w + 24) as u32;
        let initial_h = (self.grid.rows * self.font.cell_h + 20) as u32;

        let attrs = WindowAttributes::default()
            .with_title(&self.last_title)
            .with_inner_size(LogicalSize::new(initial_w, initial_h));

        let win = match event_loop.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(err) => {
                eprintln!("gull-term: failed to create Wayland window: {}", err);
                event_loop.exit();
                return;
            }
        };

        let context = match Context::new(win.clone()) {
            Ok(ctx) => ctx,
            Err(err) => {
                eprintln!("gull-term: failed to initialize softbuffer context: {}", err);
                event_loop.exit();
                return;
            }
        };

        let surface = match Surface::new(&context, win.clone()) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("gull-term: failed to create wl_shm surface: {}", err);
                event_loop.exit();
                return;
            }
        };

        let phys = win.inner_size();
        let (cols, rows) = compute_grid_dimensions(phys.width, phys.height, &self.font);
        self.grid.resize(cols, rows);

        let (tx, rx) = mpsc::channel();
        match PtySession::spawn(
            cols as u16,
            rows as u16,
            self.override_command.take(),
            self.proxy.clone(),
            tx,
        ) {
            Ok(pty) => {
                self.pty = Some(pty);
                self.fallback_rx = Some(rx);
            }
            Err(err) => {
                eprintln!("gull-term: {}", err);
                event_loop.exit();
                return;
            }
        }

        win.request_redraw();
        self.window = Some(win);
        self.surface = Some(surface);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: TermEvent) {
        match event {
            TermEvent::PtyOutput(bytes) => {
                self.process_pty_bytes(&bytes);
            }
            TermEvent::PtyExited => {
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(ref rx) = self.fallback_rx {
            let drained: Vec<Vec<u8>> = rx.try_iter().collect();
            for chunk in drained {
                self.process_pty_bytes(&chunk);
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Focused(is_focused) => {
                self.focused = is_focused;
                if let Some(ref win) = self.window {
                    win.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(new_mods) => {
                self.modifiers = new_mods.state();
            }
            WindowEvent::Resized(new_size) => {
                self.handle_resize(new_size.width, new_size.height);
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(ref win) = self.window {
                    let s = win.inner_size();
                    self.handle_resize(s.width, s.height);
                }
            }
            WindowEvent::KeyboardInput { event: key_ev, .. } => {
                if key_ev.state == ElementState::Pressed {
                    let text_ref = key_ev.text.as_deref();
                    self.translate_key_to_pty(&key_ev.logical_key, text_ref);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => (y * 3.0).round() as i32,
                    MouseScrollDelta::PixelDelta(pos) => {
                        (pos.y / self.font.cell_h.max(1) as f64).round() as i32
                    }
                };
                if lines != 0 {
                    self.grid.scroll_viewport(lines);
                    if let Some(ref win) = self.window {
                        win.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
            }
            _ => {}
        }
    }
}
