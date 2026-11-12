/// Catppuccin Mocha color palette packed as `0x00RRGGBB` `u32` pixels for `softbuffer`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const fn to_u32(self) -> u32 {
        ((self.0 as u32) << 16) | ((self.1 as u32) << 8) | (self.2 as u32)
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

pub const BASE: Rgb = Rgb(0x1e, 0x1e, 0x2e);      // #1e1e2e
pub const MANTLE: Rgb = Rgb(0x18, 0x18, 0x25);    // #181825
pub const CRUST: Rgb = Rgb(0x11, 0x11, 0x1b);     // #11111b
pub const SURFACE0: Rgb = Rgb(0x31, 0x32, 0x44);  // #313244
pub const SURFACE1: Rgb = Rgb(0x45, 0x47, 0x5a);  // #45475a
pub const SURFACE2: Rgb = Rgb(0x58, 0x5b, 0x70);  // #585b70
pub const OVERLAY0: Rgb = Rgb(0x6c, 0x70, 0x86);  // #6c7086
pub const SUBTEXT1: Rgb = Rgb(0xba, 0xc2, 0xde);  // #bac2de
pub const SUBTEXT0: Rgb = Rgb(0xa6, 0xad, 0xc8);  // #a6adc8
pub const TEXT: Rgb = Rgb(0xcd, 0xd6, 0xf4);      // #cdd6f4
pub const LAVENDER: Rgb = Rgb(0xb4, 0xbe, 0xfe);  // #b4befe
pub const MAUVE: Rgb = Rgb(0xcb, 0xa6, 0xf7);     // #cba6f7
pub const RED: Rgb = Rgb(0xf3, 0x8b, 0xa8);       // #f38ba8
pub const PEACH: Rgb = Rgb(0xfa, 0xb3, 0x87);     // #fab387
pub const YELLOW: Rgb = Rgb(0xf9, 0xe2, 0xaf);    // #f9e2af
pub const GREEN: Rgb = Rgb(0xa6, 0xe3, 0xa1);     // #a6e3a1
pub const TEAL: Rgb = Rgb(0x94, 0xe2, 0xd5);      // #94e2d5
pub const SKY: Rgb = Rgb(0x89, 0xdc, 0xeb);       // #89dceb
pub const BLUE: Rgb = Rgb(0x89, 0xb4, 0xfa);      // #89b4fa
pub const PINK: Rgb = Rgb(0xf5, 0xc2, 0xe7);      // #f5c2e7
pub const ROSEWATER: Rgb = Rgb(0xf5, 0xe0, 0xdc); // #f5e0dc

/// Catppuccin Mocha 16-color ANSI palette (0..15).
pub const ANSI_16: [Rgb; 16] = [
    SURFACE1, // 0: Black
    RED,      // 1: Red
    GREEN,    // 2: Green
    YELLOW,   // 3: Yellow
    BLUE,     // 4: Blue
    PINK,     // 5: Magenta
    TEAL,     // 6: Cyan
    SUBTEXT1, // 7: White
    SURFACE2, // 8: Bright Black
    RED,      // 9: Bright Red
    GREEN,    // 10: Bright Green
    YELLOW,   // 11: Bright Yellow
    LAVENDER, // 12: Bright Blue
    MAUVE,    // 13: Bright Magenta
    SKY,      // 14: Bright Cyan
    ROSEWATER,// 15: Bright White
];

/// Resolve an xterm 256-color index into a Catppuccin Mocha-harmonized `Rgb`.
pub fn ansi_256_to_rgb(idx: u8) -> Rgb {
    match idx {
        0..=15 => ANSI_16[idx as usize],
        16..=231 => {
            let n = idx - 16;
            let r = n / 36;
            let g = (n % 36) / 6;
            let b = n % 6;
            let to_level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            Rgb(to_level(r), to_level(g), to_level(b))
        }
        232..=255 => {
            let gray = 8 + (idx - 232) * 10;
            Rgb(gray, gray, gray)
        }
    }
}
