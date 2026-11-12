use std::fs;
use std::path::Path;

use ab_glyph::{Font, FontVec, PxScale, ScaleFont};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RingState {
    Idle,
    Typing(usize),
    Verifying,
    Failed,
}

#[derive(Clone, Copy)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const fn to_xrgb(self) -> u32 {
        0xFF00_0000 | ((self.0 as u32) << 16) | ((self.1 as u32) << 8) | (self.2 as u32)
    }

    pub fn blend(self, fg: Rgb, alpha: f32) -> Rgb {
        let a = alpha.clamp(0.0, 1.0);
        let inv = 1.0 - a;
        Rgb(
            (self.0 as f32 * inv + fg.0 as f32 * a).round() as u8,
            (self.1 as f32 * inv + fg.1 as f32 * a).round() as u8,
            (self.2 as f32 * inv + fg.2 as f32 * a).round() as u8,
        )
    }
}

// Catppuccin Mocha palette
pub const BASE: Rgb = Rgb(0x1e, 0x1e, 0x2e);
pub const MANTLE: Rgb = Rgb(0x18, 0x18, 0x25);
pub const CRUST: Rgb = Rgb(0x11, 0x11, 0x1b);
pub const SURFACE0: Rgb = Rgb(0x31, 0x32, 0x44);
pub const SURFACE1: Rgb = Rgb(0x45, 0x47, 0x5a);
pub const OVERLAY0: Rgb = Rgb(0x6c, 0x70, 0x86);
pub const SUBTEXT0: Rgb = Rgb(0xa6, 0xad, 0xc8);
pub const TEXT: Rgb = Rgb(0xcd, 0xd6, 0xf4);
pub const LAVENDER: Rgb = Rgb(0xb4, 0xbe, 0xfe);
pub const MAUVE: Rgb = Rgb(0xcb, 0xa6, 0xf7);
pub const GREEN: Rgb = Rgb(0xa6, 0xe3, 0xa1);
pub const RED: Rgb = Rgb(0xf3, 0x8b, 0xa8);
pub const PEACH: Rgb = Rgb(0xfa, 0xb3, 0x87);

const FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
    "/usr/share/fonts/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
];

pub struct LockRenderer {
    font: Option<FontVec>,
}

impl LockRenderer {
    pub fn new() -> Self {
        let mut font = None;
        for path in FONT_CANDIDATES {
            if Path::new(path).is_file() {
                if let Ok(bytes) = fs::read(path) {
                    if let Ok(parsed) = FontVec::try_from_vec(bytes) {
                        font = Some(parsed);
                        break;
                    }
                }
            }
        }
        Self { font }
    }

    pub fn draw_frame(
        &self,
        pixels: &mut [u32],
        width: usize,
        height: usize,
        user_host: &str,
        ring_state: RingState,
    ) {
        if width == 0 || height == 0 || pixels.len() < width * height {
            return;
        }

        // Smooth vertical Catppuccin Mocha gradient (Crust -> Base -> Mantle)
        for y in 0..height {
            let t = y as f32 / height.max(1) as f32;
            let row_rgb = if t < 0.5 {
                CRUST.blend(BASE, t * 2.0)
            } else {
                BASE.blend(MANTLE, (t - 0.5) * 2.0)
            };
            let px = row_rgb.to_xrgb();
            let start = y * width;
            pixels[start..start + width].fill(px);
        }

        // Top subtle Mauve accent bar
        for y in 0..3.min(height) {
            pixels[y * width..(y + 1) * width].fill(MAUVE.to_xrgb());
        }

        let (clock_str, date_str) = current_local_clock_and_date();
        let cx = width as i32 / 2;
        let cy = height as i32 / 2;

        // Center card panel
        let card_w = 420usize.min(width.saturating_sub(32));
        let card_h = 340usize.min(height.saturating_sub(32));
        let card_x = (width.saturating_sub(card_w)) / 2;
        let card_y = (height.saturating_sub(card_h)) / 2;
        draw_rounded_panel(pixels, width, height, card_x, card_y, card_w, card_h, 18, MANTLE, SURFACE0);

        // Clock & date above ring
        self.draw_centered_text(
            pixels,
            width,
            height,
            cx,
            card_y as i32 + 56,
            64.0,
            &clock_str,
            TEXT,
        );
        self.draw_centered_text(
            pixels,
            width,
            height,
            cx,
            card_y as i32 + 104,
            18.0,
            &date_str,
            SUBTEXT0,
        );

        // Password status ring
        let ring_cy = cy + 28;
        let (ring_color, status_label) = match ring_state {
            RingState::Idle => (LAVENDER, "Locked — Enter password"),
            RingState::Typing(len) => {
                if len % 2 == 0 {
                    (MAUVE, " Typing password...")
                } else {
                    (GREEN, " Typing password...")
                }
            }
            RingState::Verifying => (PEACH, "Verifying..."),
            RingState::Failed => (RED, "Incorrect password — try again"),
        };

        draw_ring(
            pixels,
            width,
            height,
            cx,
            ring_cy,
            46,
            38,
            ring_color,
            SURFACE0,
            ring_state,
        );

        // User@Host pill & status caption
        self.draw_centered_text(
            pixels,
            width,
            height,
            cx,
            card_y as i32 + card_h as i32 - 52,
            17.0,
            user_host,
            LAVENDER,
        );
        self.draw_centered_text(
            pixels,
            width,
            height,
            cx,
            card_y as i32 + card_h as i32 - 24,
            14.0,
            status_label.trim(),
            match ring_state {
                RingState::Failed => RED,
                _ => OVERLAY0,
            },
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_centered_text(
        &self,
        pixels: &mut [u32],
        width: usize,
        height: usize,
        center_x: i32,
        baseline_y: i32,
        px_size: f32,
        text: &str,
        color: Rgb,
    ) {
        if let Some(ref font) = self.font {
            let scale = PxScale::from(px_size);
            let scaled = font.as_scaled(scale);

            let mut total_w = 0.0f32;
            let mut prev = None;
            for ch in text.chars() {
                let gid = font.glyph_id(ch);
                if let Some(p) = prev {
                    total_w += scaled.kern(p, gid);
                }
                total_w += scaled.h_advance(gid);
                prev = Some(gid);
            }

            let mut pen_x = center_x as f32 - total_w * 0.5;
            prev = None;
            for ch in text.chars() {
                let gid = font.glyph_id(ch);
                if let Some(p) = prev {
                    pen_x += scaled.kern(p, gid);
                }
                let glyph = gid.with_scale_and_position(scale, ab_glyph::point(pen_x, baseline_y as f32));
                if let Some(outlined) = font.outline_glyph(glyph) {
                    let bounds = outlined.px_bounds();
                    outlined.draw(|gx, gy, cov| {
                        let px = bounds.min.x as i32 + gx as i32;
                        let py = bounds.min.y as i32 + gy as i32;
                        if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                            let idx = py as usize * width + px as usize;
                            let cur = pixels[idx];
                            let bg = Rgb(
                                ((cur >> 16) & 0xFF) as u8,
                                ((cur >> 8) & 0xFF) as u8,
                                (cur & 0xFF) as u8,
                            );
                            pixels[idx] = bg.blend(color, cov).to_xrgb();
                        }
                    });
                }
                pen_x += scaled.h_advance(gid);
                prev = Some(gid);
            }
        } else {
            // Pure vector/block 5x7 scalable fallback so clock/labels are always visible
            let scale = ((px_size / 10.0).round() as i32).max(1);
            let adv = 6 * scale;
            let total_w = text.chars().count() as i32 * adv;
            let mut x = center_x - total_w / 2;
            let y = baseline_y - 7 * scale;
            for ch in text.chars() {
                draw_mini_glyph(pixels, width, height, x, y, scale, ch, color.to_xrgb());
                x += adv;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_rounded_panel(
    pixels: &mut [u32],
    width: usize,
    height: usize,
    x0: usize,
    y0: usize,
    w: usize,
    h: usize,
    radius: i32,
    fill: Rgb,
    border: Rgb,
) {
    let fill_px = fill.to_xrgb();
    let border_px = border.to_xrgb();
    let r2 = radius * radius;
    let inner_r2 = (radius - 2).max(0) * (radius - 2).max(0);

    for dy in 0..h {
        let py = y0 + dy;
        if py >= height {
            break;
        }
        for dx in 0..w {
            let px = x0 + dx;
            if px >= width {
                break;
            }

            let cx = if (dx as i32) < radius {
                radius - dx as i32
            } else if (dx as i32) >= (w as i32 - radius) {
                dx as i32 - (w as i32 - radius - 1)
            } else {
                0
            };

            let cy = if (dy as i32) < radius {
                radius - dy as i32
            } else if (dy as i32) >= (h as i32 - radius) {
                dy as i32 - (h as i32 - radius - 1)
            } else {
                0
            };

            let dist2 = cx * cx + cy * cy;
            if dist2 <= r2 {
                let is_edge = dx < 2 || dy < 2 || dx + 2 >= w || dy + 2 >= h || (cx > 0 && cy > 0 && dist2 >= inner_r2);
                pixels[py * width + px] = if is_edge { border_px } else { fill_px };
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_ring(
    pixels: &mut [u32],
    width: usize,
    height: usize,
    cx: i32,
    cy: i32,
    outer_r: i32,
    inner_r: i32,
    ring_color: Rgb,
    interior_color: Rgb,
    state: RingState,
) {
    let r_out_f = outer_r as f32;
    let r_in_f = inner_r as f32;

    for dy in -(outer_r + 2)..=(outer_r + 2) {
        let py = cy + dy;
        if py < 0 || py >= height as i32 {
            continue;
        }
        for dx in -(outer_r + 2)..=(outer_r + 2) {
            let px = cx + dx;
            if px < 0 || px >= width as i32 {
                continue;
            }
            let dist = ((dx * dx + dy * dy) as f32).sqrt();
            let idx = py as usize * width + px as usize;
            let cur = pixels[idx];
            let bg = Rgb(
                ((cur >> 16) & 0xFF) as u8,
                ((cur >> 8) & 0xFF) as u8,
                (cur & 0xFF) as u8,
            );

            if dist <= r_in_f - 1.0 {
                pixels[idx] = interior_color.to_xrgb();
            } else if dist <= r_out_f + 1.0 {
                let outer_cov = (r_out_f + 0.5 - dist).clamp(0.0, 1.0);
                let inner_cov = (dist - (r_in_f - 0.5)).clamp(0.0, 1.0);
                let ring_cov = outer_cov.min(inner_cov);
                let base_under = if dist < r_in_f { interior_color } else { bg };
                pixels[idx] = base_under.blend(ring_color, ring_cov).to_xrgb();
            }
        }
    }

    // Draw password dot indicators inside the ring when typing
    if let RingState::Typing(count) = state {
        let dots = count.min(6);
        let spacing = 10i32;
        let start_x = cx - ((dots as i32 - 1) * spacing) / 2;
        for i in 0..dots {
            let dcx = start_x + i as i32 * spacing;
            for dy in -3..=3 {
                for dx in -3..=3 {
                    if dx * dx + dy * dy <= 9 {
                        let px = dcx + dx;
                        let py = cy + dy;
                        if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                            pixels[py as usize * width + px as usize] = TEXT.to_xrgb();
                        }
                    }
                }
            }
        }
    } else if state == RingState::Idle {
        // Minimal padlock icon inside idle ring
        for dy in -2..=8 {
            for dx in -7..=7 {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                    pixels[py as usize * width + px as usize] = SURFACE1.to_xrgb();
                }
            }
        }
        for dy in -9..=-2 {
            for dx in -5..=5 {
                let r2 = dx * dx + (dy + 3) * (dy + 3);
                if (12..=26).contains(&r2) {
                    let px = cx + dx;
                    let py = cy + dy;
                    if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                        pixels[py as usize * width + px as usize] = LAVENDER.to_xrgb();
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_mini_glyph(
    pixels: &mut [u32],
    width: usize,
    height: usize,
    x0: i32,
    y0: i32,
    scale: i32,
    ch: char,
    color: u32,
) {
    let bitmap: [u8; 7] = match ch.to_ascii_uppercase() {
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        ':' => [0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000],
        '-' | '—' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '@' => [0b01110, 0b10001, 0b10111, 0b10101, 0b10111, 0b10000, 0b01110],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b00100],
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        ' ' => [0; 7],
        _ => [0b01110, 0b10001, 0b00010, 0b00100, 0b00100, 0b00000, 0b00100],
    };

    for (row, &bits) in bitmap.iter().enumerate() {
        for col in 0..5i32 {
            if (bits & (0b10000 >> col)) != 0 {
                for sy in 0..scale {
                    for sx in 0..scale {
                        let px = x0 + col * scale + sx;
                        let py = y0 + row as i32 * scale + sy;
                        if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                            pixels[py as usize * width + px as usize] = color;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(unix)]
fn current_local_clock_and_date() -> (String, String) {
    unsafe {
        let mut now: libc::time_t = 0;
        libc::time(&mut now);
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&now, &mut tm);

        let weekdays = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
        let months = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];

        let wday = weekdays
            .get(tm.tm_wday.max(0) as usize)
            .copied()
            .unwrap_or("Day");
        let mon = months
            .get(tm.tm_mon.max(0) as usize)
            .copied()
            .unwrap_or("Jan");

        let clock = format!("{:02}:{:02}", tm.tm_hour, tm.tm_min);
        let date = format!("{}, {:02} {} {}", wday, tm.tm_mday, mon, 1900 + tm.tm_year);
        (clock, date)
    }
}

#[cfg(not(unix))]
fn current_local_clock_and_date() -> (String, String) {
    ("12:00".to_string(), "GullOS Session Lock".to_string())
}
