use crate::font::MonoFont;
use crate::grid::{Cell, TerminalGrid};
use crate::theme::{self, Rgb};

pub const PADDING_X: usize = 12;
pub const PADDING_Y: usize = 10;

pub fn compute_grid_dimensions(
    pixel_w: u32,
    pixel_h: u32,
    font: &MonoFont,
) -> (usize, usize) {
    let usable_w = (pixel_w as usize).saturating_sub(PADDING_X * 2);
    let usable_h = (pixel_h as usize).saturating_sub(PADDING_Y * 2);
    let cols = (usable_w / font.cell_w.max(1)).max(10);
    let rows = (usable_h / font.cell_h.max(1)).max(4);
    (cols, rows)
}

pub fn render_grid_to_buffer(
    pixels: &mut [u32],
    buf_w: usize,
    buf_h: usize,
    grid: &TerminalGrid,
    font: &mut MonoFont,
    focused: bool,
) {
    if buf_w == 0 || buf_h == 0 || pixels.len() < buf_w * buf_h {
        return;
    }

    let base_u32 = theme::BASE.to_u32();
    pixels[..buf_w * buf_h].fill(base_u32);

    // Subtle 1px top accent bar in Catppuccin Mocha Mauve (focused) or Surface1 (unfocused)
    let accent = if focused {
        theme::MAUVE.to_u32()
    } else {
        theme::SURFACE1.to_u32()
    };
    for y in 0..2usize.min(buf_h) {
        let row_start = y * buf_w;
        pixels[row_start..row_start + buf_w].fill(accent);
    }

    let cell_w = font.cell_w;
    let cell_h = font.cell_h;

    for row_idx in 0..grid.rows {
        let py_start = PADDING_Y + row_idx * cell_h;
        if py_start >= buf_h {
            break;
        }

        let Some(row_cells) = grid.visible_row(row_idx) else {
            continue;
        };

        for col_idx in 0..grid.cols {
            let px_start = PADDING_X + col_idx * cell_w;
            if px_start >= buf_w {
                break;
            }

            let cell = row_cells.get(col_idx).copied().unwrap_or_else(Cell::default);
            let is_cursor = grid.cursor_visible
                && grid.scroll_offset == 0
                && row_idx == grid.cursor_row
                && col_idx == grid.cursor_col;

            let (mut fg, mut bg) = if cell.inverse {
                (cell.bg, cell.fg)
            } else {
                (cell.fg, cell.bg)
            };

            if is_cursor && focused {
                bg = theme::ROSEWATER;
                fg = theme::CRUST;
            }

            // Fill cell background when distinct from BASE
            if bg != theme::BASE {
                fill_rect(pixels, buf_w, buf_h, px_start, py_start, cell_w, cell_h, bg.to_u32());
            }

            // Unfocused hollow cursor outline
            if is_cursor && !focused {
                stroke_rect(
                    pixels,
                    buf_w,
                    buf_h,
                    px_start,
                    py_start,
                    cell_w,
                    cell_h,
                    theme::LAVENDER.to_u32(),
                );
            }

            // Underline decoration
            if cell.underline {
                let uy = (py_start + cell_h.saturating_sub(2)).min(buf_h - 1);
                let end_x = (px_start + cell_w).min(buf_w);
                let fg_u32 = fg.to_u32();
                for x in px_start..end_x {
                    pixels[uy * buf_w + x] = fg_u32;
                }
            }

            // Glyph alpha-blended rasterization
            if cell.ch != ' ' && cell.ch != '\0' {
                let glyph = font.glyph(cell.ch, cell.bold);
                if glyph.width > 0 && glyph.height > 0 {
                    blit_glyph(
                        pixels,
                        buf_w,
                        buf_h,
                        px_start as i32 + glyph.offset_x,
                        py_start as i32 + glyph.offset_y,
                        glyph.width,
                        glyph.height,
                        &glyph.alpha,
                        fg,
                        bg,
                    );
                }
            }
        }
    }

    // Scrollback indicator pill on right edge if scrolled up
    if grid.scroll_offset > 0 && buf_w > 8 {
        let bar_x = buf_w - 5;
        fill_rect(
            pixels,
            buf_w,
            buf_h,
            bar_x,
            PADDING_Y,
            3,
            buf_h.saturating_sub(PADDING_Y * 2),
            theme::SURFACE1.to_u32(),
        );
        fill_rect(
            pixels,
            buf_w,
            buf_h,
            bar_x,
            PADDING_Y,
            3,
            28.min(buf_h.saturating_sub(PADDING_Y * 2)),
            theme::LAVENDER.to_u32(),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_rect(
    pixels: &mut [u32],
    buf_w: usize,
    buf_h: usize,
    x0: usize,
    y0: usize,
    w: usize,
    h: usize,
    color: u32,
) {
    let x1 = (x0 + w).min(buf_w);
    let y1 = (y0 + h).min(buf_h);
    for y in y0..y1 {
        let row = y * buf_w;
        pixels[row + x0..row + x1].fill(color);
    }
}

#[allow(clippy::too_many_arguments)]
fn stroke_rect(
    pixels: &mut [u32],
    buf_w: usize,
    buf_h: usize,
    x0: usize,
    y0: usize,
    w: usize,
    h: usize,
    color: u32,
) {
    if w == 0 || h == 0 {
        return;
    }
    let x1 = (x0 + w - 1).min(buf_w - 1);
    let y1 = (y0 + h - 1).min(buf_h - 1);
    for x in x0..=x1 {
        pixels[y0 * buf_w + x] = color;
        pixels[y1 * buf_w + x] = color;
    }
    for y in y0..=y1 {
        pixels[y * buf_w + x0] = color;
        pixels[y * buf_w + x1] = color;
    }
}

#[allow(clippy::too_many_arguments)]
fn blit_glyph(
    pixels: &mut [u32],
    buf_w: usize,
    buf_h: usize,
    dst_x: i32,
    dst_y: i32,
    gw: usize,
    gh: usize,
    alpha: &[u8],
    fg: Rgb,
    bg: Rgb,
) {
    let fg_u32 = fg.to_u32();
    for gy in 0..gh {
        let py = dst_y + gy as i32;
        if py < 0 || py >= buf_h as i32 {
            continue;
        }
        let row_off = py as usize * buf_w;
        let g_row = gy * gw;

        for gx in 0..gw {
            let cov = alpha[g_row + gx];
            if cov == 0 {
                continue;
            }
            let px = dst_x + gx as i32;
            if px < 0 || px >= buf_w as i32 {
                continue;
            }
            let idx = row_off + px as usize;
            if cov >= 250 {
                pixels[idx] = fg_u32;
            } else {
                let blended = bg.blend(fg, cov as f32 / 255.0);
                pixels[idx] = blended.to_u32();
            }
        }
    }
}
