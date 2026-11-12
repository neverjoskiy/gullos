use std::ffi::CString;
use std::os::fd::{AsFd, FromRawFd, OwnedFd};
use std::ptr;
use std::time::{Duration, Instant};

use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_keyboard, wl_output, wl_registry, wl_seat, wl_shm, wl_shm_pool,
    wl_surface,
};
use wayland_client::{Connection, Dispatch, EventQueue, QueueHandle, WEnum};
use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1, ext_session_lock_surface_v1, ext_session_lock_v1,
};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

use crate::auth::LockConfig;
use crate::render::{LockRenderer, RingState};

struct ShmBuffer {
    buffer: wl_buffer::WlBuffer,
    ptr: *mut u32,
    byte_len: usize,
    width: u32,
    height: u32,
}

impl Drop for ShmBuffer {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() && self.byte_len > 0 {
                libc::munmap(self.ptr as *mut libc::c_void, self.byte_len);
            }
        }
        self.buffer.destroy();
    }
}

struct OutputSurface {
    output_global_name: u32,
    output: wl_output::WlOutput,
    wl_surface: Option<wl_surface::WlSurface>,
    lock_surface: Option<ext_session_lock_surface_v1::ExtSessionLockSurfaceV1>,
    layer_surface: Option<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    shm_buf: Option<ShmBuffer>,
    width: u32,
    height: u32,
    configured: bool,
}

pub struct WaylandLockState {
    config: LockConfig,
    renderer: LockRenderer,
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    seat: Option<wl_seat::WlSeat>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    lock_manager: Option<ext_session_lock_manager_v1::ExtSessionLockManagerV1>,
    session_lock: Option<ext_session_lock_v1::ExtSessionLockV1>,
    layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    outputs: Vec<OutputSurface>,
    using_ext_lock: bool,
    locked_confirmed: bool,
    unlocked: bool,
    aborted_error: Option<String>,
    password_buf: String,
    ring_state: RingState,
    failed_at: Option<Instant>,
    shift_down: bool,
    ctrl_down: bool,
    caps_locked: bool,
    last_drawn_minute: i32,
}

impl WaylandLockState {
    fn new(config: LockConfig) -> Self {
        Self {
            config,
            renderer: LockRenderer::new(),
            compositor: None,
            shm: None,
            seat: None,
            keyboard: None,
            lock_manager: None,
            session_lock: None,
            layer_shell: None,
            outputs: Vec::new(),
            using_ext_lock: false,
            locked_confirmed: false,
            unlocked: false,
            aborted_error: None,
            password_buf: String::with_capacity(64),
            ring_state: RingState::Idle,
            failed_at: None,
            shift_down: false,
            ctrl_down: false,
            caps_locked: false,
            last_drawn_minute: current_minute_of_day(),
        }
    }

    fn create_surfaces_for_outputs(&mut self, qh: &QueueHandle<Self>) {
        let Some(ref compositor) = self.compositor else {
            return;
        };

        for idx in 0..self.outputs.len() {
            if self.outputs[idx].wl_surface.is_some() {
                continue;
            }
            let surface = compositor.create_surface(qh, ());
            if self.using_ext_lock {
                if let Some(ref lock) = self.session_lock {
                    let lock_surf =
                        lock.get_lock_surface(&surface, &self.outputs[idx].output, qh, idx);
                    self.outputs[idx].lock_surface = Some(lock_surf);
                }
            } else if let Some(ref layer_shell) = self.layer_shell {
                let layer_surf = layer_shell.get_layer_surface(
                    &surface,
                    Some(&self.outputs[idx].output),
                    zwlr_layer_shell_v1::Layer::Overlay,
                    "gull-lock".to_string(),
                    qh,
                    idx,
                );
                layer_surf.set_size(0, 0);
                layer_surf.set_anchor(
                    zwlr_layer_surface_v1::Anchor::Top
                        | zwlr_layer_surface_v1::Anchor::Bottom
                        | zwlr_layer_surface_v1::Anchor::Left
                        | zwlr_layer_surface_v1::Anchor::Right,
                );
                layer_surf.set_exclusive_zone(-1);
                layer_surf.set_keyboard_interactivity(
                    zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive,
                );
                surface.commit();
                self.outputs[idx].layer_surface = Some(layer_surf);
            }
            self.outputs[idx].wl_surface = Some(surface);
        }
    }

    fn redraw_all(&mut self, qh: &QueueHandle<Self>) {
        for idx in 0..self.outputs.len() {
            if self.outputs[idx].configured {
                self.redraw_output(idx, qh);
            }
        }
        self.last_drawn_minute = current_minute_of_day();
    }

    fn redraw_output(&mut self, idx: usize, qh: &QueueHandle<Self>) {
        let Some(ref shm) = self.shm else {
            return;
        };
        if idx >= self.outputs.len() {
            return;
        }

        let w = self.outputs[idx].width.max(320);
        let h = self.outputs[idx].height.max(240);

        let needs_alloc = match self.outputs[idx].shm_buf {
            Some(ref b) => b.width != w || b.height != h,
            None => true,
        };

        if needs_alloc {
            match allocate_shm_buffer(shm, w, h, qh) {
                Ok(buf) => {
                    self.outputs[idx].shm_buf = Some(buf);
                }
                Err(err) => {
                    eprintln!("gull-lock: wl_shm allocation error: {}", err);
                    return;
                }
            }
        }

        let user_host = format!("{}@{}", self.config.user, self.config.hostname);
        let ring = self.ring_state;

        let wl_surface = self.outputs[idx].wl_surface.clone();

        if let Some(ref mut shm_buf) = self.outputs[idx].shm_buf {
            let total_px = (w as usize) * (h as usize);
            let slice = unsafe { std::slice::from_raw_parts_mut(shm_buf.ptr, total_px) };
            self.renderer
                .draw_frame(slice, w as usize, h as usize, &user_host, ring);

            if let Some(ref wl_surface) = wl_surface {
                wl_surface.attach(Some(&shm_buf.buffer), 0, 0);
                wl_surface.damage_buffer(0, 0, w as i32, h as i32);
                wl_surface.commit();
            }
        }
    }

    fn handle_key_press(&mut self, evdev_key: u32, qh: &QueueHandle<Self>) {
        match evdev_key {
            // Escape
            1 => {
                self.password_buf.clear();
                self.ring_state = RingState::Idle;
                self.failed_at = None;
                self.redraw_all(qh);
            }
            // Backspace
            14 => {
                self.password_buf.pop();
                self.failed_at = None;
                self.ring_state = if self.password_buf.is_empty() {
                    RingState::Idle
                } else {
                    RingState::Typing(self.password_buf.chars().count())
                };
                self.redraw_all(qh);
            }
            // Enter / Keypad Enter
            28 | 96 => {
                self.ring_state = RingState::Verifying;
                self.redraw_all(qh);

                if self.config.verify(&self.password_buf) {
                    self.password_buf.clear();
                    if self.using_ext_lock {
                        if let Some(lock) = self.session_lock.take() {
                            lock.unlock_and_destroy();
                        }
                    } else {
                        for out in &mut self.outputs {
                            if let Some(ls) = out.layer_surface.take() {
                                ls.destroy();
                            }
                            if let Some(ws) = out.wl_surface.take() {
                                ws.destroy();
                            }
                        }
                    }
                    self.unlocked = true;
                } else {
                    self.password_buf.clear();
                    self.ring_state = RingState::Failed;
                    self.failed_at = Some(Instant::now());
                    self.redraw_all(qh);
                }
            }
            other => {
                if self.ctrl_down && other == 22 {
                    // Ctrl+U clears input buffer
                    self.password_buf.clear();
                    self.ring_state = RingState::Idle;
                    self.failed_at = None;
                    self.redraw_all(qh);
                    return;
                }

                if !self.ctrl_down {
                    if let Some(ch) = evdev_to_ascii(other, self.shift_down, self.caps_locked) {
                        if self.password_buf.len() < 128 {
                            self.password_buf.push(ch);
                            self.failed_at = None;
                            self.ring_state = RingState::Typing(self.password_buf.chars().count());
                            self.redraw_all(qh);
                        }
                    }
                }
            }
        }
    }
}

pub fn run_wayland_lock(config: LockConfig) -> Result<(), String> {
    let conn = Connection::connect_to_env()
        .map_err(|e| format!("failed to connect to Wayland compositor: {}", e))?;

    let mut event_queue: EventQueue<WaylandLockState> = conn.new_event_queue();
    let qh = event_queue.handle();

    let display = conn.display();
    let _registry = display.get_registry(&qh, ());

    let mut state = WaylandLockState::new(config);

    // Initial roundtrip to discover globals (compositor, shm, seat, outputs, lock manager / layer shell)
    event_queue
        .roundtrip(&mut state)
        .map_err(|e| format!("Wayland registry roundtrip failed: {}", e))?;

    if state.compositor.is_none() || state.shm.is_none() {
        return Err("Wayland compositor missing wl_compositor or wl_shm".to_string());
    }

    if let Some(ref manager) = state.lock_manager {
        let lock = manager.lock(&qh, ());
        state.session_lock = Some(lock);
        state.using_ext_lock = true;
    } else if state.layer_shell.is_some() {
        state.using_ext_lock = false;
        state.locked_confirmed = true;
    } else {
        return Err(
            "Wayland compositor supports neither ext_session_lock_manager_v1 nor zwlr_layer_shell_v1"
                .to_string(),
        );
    }

    state.create_surfaces_for_outputs(&qh);

    // Second roundtrip to receive initial Configure events and render initial locked buffers
    event_queue
        .roundtrip(&mut state)
        .map_err(|e| format!("Wayland surface configure roundtrip failed: {}", e))?;

    while !state.unlocked {
        if let Some(ref err) = state.aborted_error {
            return Err(err.clone());
        }

        // Reset Failed ring back to Idle after 1.4 seconds
        if let Some(failed_ts) = state.failed_at {
            if failed_ts.elapsed() >= Duration::from_millis(1400) {
                state.failed_at = None;
                if state.password_buf.is_empty() {
                    state.ring_state = RingState::Idle;
                    state.redraw_all(&qh);
                }
            }
        }

        // Refresh clock if the wall-clock minute rolled over
        if current_minute_of_day() != state.last_drawn_minute {
            state.redraw_all(&qh);
        }

        let _ = conn.flush();

        event_queue
            .dispatch_pending(&mut state)
            .map_err(|e| format!("Wayland dispatch error: {}", e))?;

        if state.unlocked {
            break;
        }

        let poll_timeout_ms: i32 = if state.failed_at.is_some() {
            250
        } else {
            10_000
        };

        if let Some(guard) = conn.prepare_read() {
            let raw_fd = guard.connection_fd();
            let mut pfd = libc::pollfd {
                fd: std::os::unix::io::AsRawFd::as_raw_fd(&raw_fd),
                events: libc::POLLIN,
                revents: 0,
            };
            let rc = unsafe { libc::poll(&mut pfd, 1, poll_timeout_ms) };
            if rc > 0 && (pfd.revents & (libc::POLLIN | libc::POLLERR | libc::POLLHUP)) != 0 {
                let _ = guard.read();
            } else {
                drop(guard);
            }
        }

        event_queue
            .dispatch_pending(&mut state)
            .map_err(|e| format!("Wayland dispatch error: {}", e))?;
    }

    // Flush final unlock_and_destroy request to compositor before exiting cleanly
    let _ = event_queue.roundtrip(&mut state);
    let _ = conn.flush();
    Ok(())
}

fn allocate_shm_buffer(
    shm: &wl_shm::WlShm,
    width: u32,
    height: u32,
    qh: &QueueHandle<WaylandLockState>,
) -> Result<ShmBuffer, String> {
    let stride = width
        .checked_mul(4)
        .ok_or_else(|| "buffer width overflow".to_string())?;
    let byte_len = (stride as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| "buffer size overflow".to_string())?;

    let name = CString::new("gull-lock-shm").unwrap();
    let raw_fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
    if raw_fd < 0 {
        return Err("memfd_create failed".to_string());
    }
    let fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };

    if unsafe { libc::ftruncate(raw_fd, byte_len as libc::off_t) } < 0 {
        return Err("ftruncate on memfd failed".to_string());
    }

    let mapped = unsafe {
        libc::mmap(
            ptr::null_mut(),
            byte_len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            raw_fd,
            0,
        )
    };
    if mapped == libc::MAP_FAILED {
        return Err("mmap on memfd failed".to_string());
    }

    let pool = shm.create_pool(fd.as_fd(), byte_len as i32, qh, ());
    let buffer = pool.create_buffer(
        0,
        width as i32,
        height as i32,
        stride as i32,
        wl_shm::Format::Xrgb8888,
        qh,
        (),
    );
    pool.destroy();

    Ok(ShmBuffer {
        buffer,
        ptr: mapped as *mut u32,
        byte_len,
        width,
        height,
    })
}

fn current_minute_of_day() -> i32 {
    unsafe {
        let mut now: libc::time_t = 0;
        libc::time(&mut now);
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut tm);
        tm.tm_hour * 60 + tm.tm_min
    }
}

fn evdev_to_ascii(evdev_code: u32, shift: bool, caps: bool) -> Option<char> {
    let alpha_upper = shift ^ caps;
    match evdev_code {
        2 => Some(if shift { '!' } else { '1' }),
        3 => Some(if shift { '@' } else { '2' }),
        4 => Some(if shift { '#' } else { '3' }),
        5 => Some(if shift { '$' } else { '4' }),
        6 => Some(if shift { '%' } else { '5' }),
        7 => Some(if shift { '^' } else { '6' }),
        8 => Some(if shift { '&' } else { '7' }),
        9 => Some(if shift { '*' } else { '8' }),
        10 => Some(if shift { '(' } else { '9' }),
        11 => Some(if shift { ')' } else { '0' }),
        12 => Some(if shift { '_' } else { '-' }),
        13 => Some(if shift { '+' } else { '=' }),
        16 => Some(if alpha_upper { 'Q' } else { 'q' }),
        17 => Some(if alpha_upper { 'W' } else { 'w' }),
        18 => Some(if alpha_upper { 'E' } else { 'e' }),
        19 => Some(if alpha_upper { 'R' } else { 'r' }),
        20 => Some(if alpha_upper { 'T' } else { 't' }),
        21 => Some(if alpha_upper { 'Y' } else { 'y' }),
        22 => Some(if alpha_upper { 'U' } else { 'u' }),
        23 => Some(if alpha_upper { 'I' } else { 'i' }),
        24 => Some(if alpha_upper { 'O' } else { 'o' }),
        25 => Some(if alpha_upper { 'P' } else { 'p' }),
        26 => Some(if shift { '{' } else { '[' }),
        27 => Some(if shift { '}' } else { ']' }),
        30 => Some(if alpha_upper { 'A' } else { 'a' }),
        31 => Some(if alpha_upper { 'S' } else { 's' }),
        32 => Some(if alpha_upper { 'D' } else { 'd' }),
        33 => Some(if alpha_upper { 'F' } else { 'f' }),
        34 => Some(if alpha_upper { 'G' } else { 'g' }),
        35 => Some(if alpha_upper { 'H' } else { 'h' }),
        36 => Some(if alpha_upper { 'J' } else { 'j' }),
        37 => Some(if alpha_upper { 'K' } else { 'k' }),
        38 => Some(if alpha_upper { 'L' } else { 'l' }),
        39 => Some(if shift { ':' } else { ';' }),
        40 => Some(if shift { '"' } else { '\'' }),
        41 => Some(if shift { '~' } else { '`' }),
        43 => Some(if shift { '|' } else { '\\' }),
        44 => Some(if alpha_upper { 'Z' } else { 'z' }),
        45 => Some(if alpha_upper { 'X' } else { 'x' }),
        46 => Some(if alpha_upper { 'C' } else { 'c' }),
        47 => Some(if alpha_upper { 'V' } else { 'v' }),
        48 => Some(if alpha_upper { 'B' } else { 'b' }),
        49 => Some(if alpha_upper { 'N' } else { 'n' }),
        50 => Some(if alpha_upper { 'M' } else { 'm' }),
        51 => Some(if shift { '<' } else { ',' }),
        52 => Some(if shift { '>' } else { '.' }),
        53 => Some(if shift { '?' } else { '/' }),
        57 => Some(' '),
        // Numpad digits
        71 => Some('7'),
        72 => Some('8'),
        73 => Some('9'),
        74 => Some('-'),
        75 => Some('4'),
        76 => Some('5'),
        77 => Some('6'),
        78 => Some('+'),
        79 => Some('1'),
        80 => Some('2'),
        81 => Some('3'),
        82 => Some('0'),
        83 => Some('.'),
        _ => None,
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandLockState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => match interface.as_str() {
                "wl_compositor" => {
                    state.compositor =
                        Some(registry.bind::<wl_compositor::WlCompositor, _, _>(
                            name,
                            version.min(5),
                            qh,
                            (),
                        ));
                }
                "wl_shm" => {
                    state.shm = Some(registry.bind::<wl_shm::WlShm, _, _>(
                        name,
                        version.min(1),
                        qh,
                        (),
                    ));
                }
                "wl_seat" => {
                    if state.seat.is_none() {
                        state.seat = Some(registry.bind::<wl_seat::WlSeat, _, _>(
                            name,
                            version.min(7),
                            qh,
                            (),
                        ));
                    }
                }
                "wl_output" => {
                    let output = registry.bind::<wl_output::WlOutput, _, _>(
                        name,
                        version.min(4),
                        qh,
                        (),
                    );
                    state.outputs.push(OutputSurface {
                        output_global_name: name,
                        output,
                        wl_surface: None,
                        lock_surface: None,
                        layer_surface: None,
                        shm_buf: None,
                        width: 1280,
                        height: 800,
                        configured: false,
                    });
                    if state.session_lock.is_some() || (!state.using_ext_lock && state.layer_shell.is_some()) {
                        state.create_surfaces_for_outputs(qh);
                    }
                }
                "ext_session_lock_manager_v1" => {
                    state.lock_manager = Some(
                        registry
                            .bind::<ext_session_lock_manager_v1::ExtSessionLockManagerV1, _, _>(
                                name,
                                version.min(1),
                                qh,
                                (),
                            ),
                    );
                }
                "zwlr_layer_shell_v1" => {
                    state.layer_shell =
                        Some(registry.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(
                            name,
                            version.min(4),
                            qh,
                            (),
                        ));
                }
                _ => {}
            },
            wl_registry::Event::GlobalRemove { name } => {
                state.outputs.retain(|o| o.output_global_name != name);
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for WaylandLockState {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(caps),
        } = event
        {
            if caps.contains(wl_seat::Capability::Keyboard) && state.keyboard.is_none() {
                state.keyboard = Some(seat.get_keyboard(qh, ()));
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for WaylandLockState {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_keyboard::Event::Modifiers {
                mods_depressed,
                mods_locked,
                ..
            } => {
                state.shift_down = (mods_depressed & 0x01) != 0;
                state.ctrl_down = (mods_depressed & 0x04) != 0;
                state.caps_locked = (mods_locked & 0x02) != 0;
            }
            wl_keyboard::Event::Key {
                key,
                state: WEnum::Value(key_state),
                ..
            } => {
                let pressed = key_state == wl_keyboard::KeyState::Pressed;
                match key {
                    42 | 54 => state.shift_down = pressed,
                    29 | 97 => state.ctrl_down = pressed,
                    58 if pressed => state.caps_locked = !state.caps_locked,
                    _ if pressed => state.handle_key_press(key, qh),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<ext_session_lock_v1::ExtSessionLockV1, ()> for WaylandLockState {
    fn event(
        state: &mut Self,
        _: &ext_session_lock_v1::ExtSessionLockV1,
        event: ext_session_lock_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_session_lock_v1::Event::Locked => {
                state.locked_confirmed = true;
            }
            ext_session_lock_v1::Event::Finished => {
                if !state.unlocked {
                    state.aborted_error =
                        Some("Wayland compositor denied or terminated session lock".to_string());
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<ext_session_lock_surface_v1::ExtSessionLockSurfaceV1, usize> for WaylandLockState {
    fn event(
        state: &mut Self,
        lock_surface: &ext_session_lock_surface_v1::ExtSessionLockSurfaceV1,
        event: ext_session_lock_surface_v1::Event,
        &output_idx: &usize,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let ext_session_lock_surface_v1::Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            lock_surface.ack_configure(serial);
            if let Some(out) = state.outputs.get_mut(output_idx) {
                if width > 0 {
                    out.width = width;
                }
                if height > 0 {
                    out.height = height;
                }
                out.configured = true;
            }
            state.redraw_output(output_idx, qh);
        }
    }
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, usize> for WaylandLockState {
    fn event(
        state: &mut Self,
        layer_surface: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
        event: zwlr_layer_surface_v1::Event,
        &output_idx: &usize,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_layer_surface_v1::Event::Configure {
                serial,
                width,
                height,
            } => {
                layer_surface.ack_configure(serial);
                if let Some(out) = state.outputs.get_mut(output_idx) {
                    if width > 0 {
                        out.width = width;
                    }
                    if height > 0 {
                        out.height = height;
                    }
                    out.configured = true;
                }
                state.redraw_output(output_idx, qh);
            }
            zwlr_layer_surface_v1::Event::Closed => {}
            _ => {}
        }
    }
}

// No-op Dispatch implementations for stateless Wayland protocol objects
impl Dispatch<wl_compositor::WlCompositor, ()> for WaylandLockState {
    fn event(
        _: &mut Self,
        _: &wl_compositor::WlCompositor,
        _: wl_compositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm::WlShm, ()> for WaylandLockState {
    fn event(
        _: &mut Self,
        _: &wl_shm::WlShm,
        _: wl_shm::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_shm_pool::WlShmPool, ()> for WaylandLockState {
    fn event(
        _: &mut Self,
        _: &wl_shm_pool::WlShmPool,
        _: wl_shm_pool::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_buffer::WlBuffer, ()> for WaylandLockState {
    fn event(
        _: &mut Self,
        _: &wl_buffer::WlBuffer,
        _: wl_buffer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for WaylandLockState {
    fn event(
        _: &mut Self,
        _: &wl_surface::WlSurface,
        _: wl_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_output::WlOutput, ()> for WaylandLockState {
    fn event(
        _: &mut Self,
        _: &wl_output::WlOutput,
        _: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ext_session_lock_manager_v1::ExtSessionLockManagerV1, ()> for WaylandLockState {
    fn event(
        _: &mut Self,
        _: &ext_session_lock_manager_v1::ExtSessionLockManagerV1,
        _: ext_session_lock_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, ()> for WaylandLockState {
    fn event(
        _: &mut Self,
        _: &zwlr_layer_shell_v1::ZwlrLayerShellV1,
        _: zwlr_layer_shell_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
