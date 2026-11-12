use std::collections::VecDeque;

use vte::{Params, Perform};

use crate::theme::{self, ansi_256_to_rgb, Rgb, ANSI_16};

const MAX_SCROLLBACK: usize = 4000;

#[derive(Clone, Copy, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: Rgb,
    pub bg: Rgb,
    pub bold: bool,
    pub underline: bool,
    pub inverse: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: theme::TEXT,
            bg: theme::BASE,
            bold: false,
            underline: false,
            inverse: false,
        }
    }
}

pub struct TerminalGrid {
    pub cols: usize,
    pub rows: usize,
    cells: Vec<Vec<Cell>>,
    alt_cells: Option<Vec<Vec<Cell>>>,
    alt_saved_cursor: (usize, usize),
    scrollback: VecDeque<Vec<Cell>>,
    pub scroll_offset: usize,
    pub cursor_col: usize,
    pub cursor_row: usize,
    pub cursor_visible: bool,
    saved_cursor: (usize, usize),
    scroll_top: usize,
    scroll_bottom: usize,
    wrap_pending: bool,
    cur_fg: Rgb,
    cur_bg: Rgb,
    cur_bold: bool,
    cur_dim: bool,
    cur_underline: bool,
    cur_inverse: bool,
    pub title: String,
    pub app_cursor_keys: bool,
    pub dirty: bool,
}

impl TerminalGrid {
    pub fn new(cols: usize, rows: usize) -> Self {
        let cols = cols.max(10);
        let rows = rows.max(4);
        let cells = vec![vec![Cell::default(); cols]; rows];
        Self {
            cols,
            rows,
            cells,
            alt_cells: None,
            alt_saved_cursor: (0, 0),
            scrollback: VecDeque::with_capacity(512),
            scroll_offset: 0,
            cursor_col: 0,
            cursor_row: 0,
            cursor_visible: true,
            saved_cursor: (0, 0),
            scroll_top: 0,
            scroll_bottom: rows - 1,
            wrap_pending: false,
            cur_fg: theme::TEXT,
            cur_bg: theme::BASE,
            cur_bold: false,
            cur_dim: false,
            cur_underline: false,
            cur_inverse: false,
            title: "GullTerm — GullOS".to_string(),
            app_cursor_keys: false,
            dirty: true,
        }
    }

    pub fn resize(&mut self, new_cols: usize, new_rows: usize) {
        let new_cols = new_cols.max(10);
        let new_rows = new_rows.max(4);
        if new_cols == self.cols && new_rows == self.rows {
            return;
        }

        let mut resized = vec![vec![Cell::default(); new_cols]; new_rows];
        let copy_rows = self.rows.min(new_rows);
        let copy_cols = self.cols.min(new_cols);

        for r in 0..copy_rows {
            resized[r][..copy_cols].copy_from_slice(&self.cells[r][..copy_cols]);
        }

        self.cells = resized;
        if self.alt_cells.is_some() {
            self.alt_cells = Some(vec![vec![Cell::default(); new_cols]; new_rows]);
        }
        self.cols = new_cols;
        self.rows = new_rows;
        self.cursor_col = self.cursor_col.min(new_cols - 1);
        self.cursor_row = self.cursor_row.min(new_rows - 1);
        self.scroll_top = 0;
        self.scroll_bottom = new_rows - 1;
        self.wrap_pending = false;
        self.dirty = true;
    }

    pub fn scroll_viewport(&mut self, delta_lines: i32) {
        if self.alt_cells.is_some() {
            return;
        }
        let max_off = self.scrollback.len();
        if delta_lines > 0 {
            self.scroll_offset = (self.scroll_offset + delta_lines as usize).min(max_off);
        } else {
            let sub = (-delta_lines) as usize;
            self.scroll_offset = self.scroll_offset.saturating_sub(sub);
        }
        self.dirty = true;
    }

    pub fn visible_row(&self, screen_row: usize) -> Option<&[Cell]> {
        if self.scroll_offset == 0 || self.alt_cells.is_some() {
            self.cells.get(screen_row).map(|r| r.as_slice())
        } else {
            let sb_len = self.scrollback.len();
            let virtual_start = sb_len.saturating_sub(self.scroll_offset);
            let idx = virtual_start + screen_row;
            if idx < sb_len {
                self.scrollback.get(idx).map(|r| r.as_slice())
            } else {
                self.cells.get(idx - sb_len).map(|r| r.as_slice())
            }
        }
    }

    fn blank_cell_with_bg(&self) -> Cell {
        Cell {
            ch: ' ',
            fg: self.cur_fg,
            bg: self.cur_bg,
            bold: false,
            underline: false,
            inverse: false,
        }
    }

    fn effective_fg(&self) -> Rgb {
        if self.cur_dim {
            self.cur_fg.blend(theme::OVERLAY0, 0.45)
        } else {
            self.cur_fg
        }
    }

    fn scroll_up_region(&mut self, top: usize, bottom: usize, count: usize) {
        let top = top.min(self.rows - 1);
        let bottom = bottom.min(self.rows - 1).max(top);
        let blank = self.blank_cell_with_bg();

        for _ in 0..count {
            let removed = self.cells.remove(top);
            if top == 0 && self.alt_cells.is_none() {
                if self.scrollback.len() >= MAX_SCROLLBACK {
                    self.scrollback.pop_front();
                }
                self.scrollback.push_back(removed);
            }
            self.cells.insert(bottom, vec![blank; self.cols]);
        }
    }

    fn scroll_down_region(&mut self, top: usize, bottom: usize, count: usize) {
        let top = top.min(self.rows - 1);
        let bottom = bottom.min(self.rows - 1).max(top);
        let blank = self.blank_cell_with_bg();

        for _ in 0..count {
            self.cells.remove(bottom);
            self.cells.insert(top, vec![blank; self.cols]);
        }
    }

    fn newline(&mut self) {
        self.wrap_pending = false;
        if self.cursor_row == self.scroll_bottom {
            self.scroll_up_region(self.scroll_top, self.scroll_bottom, 1);
        } else if self.cursor_row + 1 < self.rows {
            self.cursor_row += 1;
        }
    }

    fn reset_sgr(&mut self) {
        self.cur_fg = theme::TEXT;
        self.cur_bg = theme::BASE;
        self.cur_bold = false;
        self.cur_dim = false;
        self.cur_underline = false;
        self.cur_inverse = false;
    }

    fn handle_sgr(&mut self, params: &Params) {
        let flat: Vec<u16> = params
            .iter()
            .map(|sub| sub.first().copied().unwrap_or(0))
            .collect();

        if flat.is_empty() {
            self.reset_sgr();
            return;
        }

        let mut i = 0;
        while i < flat.len() {
            match flat[i] {
                0 => self.reset_sgr(),
                1 => {
                    self.cur_bold = true;
                    self.cur_dim = false;
                }
                2 => self.cur_dim = true,
                4 => self.cur_underline = true,
                7 => self.cur_inverse = true,
                22 => {
                    self.cur_bold = false;
                    self.cur_dim = false;
                }
                24 => self.cur_underline = false,
                27 => self.cur_inverse = false,
                30..=37 => {
                    let idx = (flat[i] - 30) as usize;
                    self.cur_fg = ANSI_16[if self.cur_bold { idx + 8 } else { idx }];
                }
                38 => {
                    if let Some(&mode) = flat.get(i + 1) {
                        if mode == 5 {
                            if let Some(&idx) = flat.get(i + 2) {
                                self.cur_fg = ansi_256_to_rgb(idx as u8);
                                i += 2;
                            }
                        } else if mode == 2 && i + 4 < flat.len() {
                            self.cur_fg = Rgb(
                                flat[i + 2] as u8,
                                flat[i + 3] as u8,
                                flat[i + 4] as u8,
                            );
                            i += 4;
                        }
                    }
                }
                39 => self.cur_fg = theme::TEXT,
                40..=47 => {
                    self.cur_bg = ANSI_16[(flat[i] - 40) as usize];
                }
                48 => {
                    if let Some(&mode) = flat.get(i + 1) {
                        if mode == 5 {
                            if let Some(&idx) = flat.get(i + 2) {
                                self.cur_bg = ansi_256_to_rgb(idx as u8);
                                i += 2;
                            }
                        } else if mode == 2 && i + 4 < flat.len() {
                            self.cur_bg = Rgb(
                                flat[i + 2] as u8,
                                flat[i + 3] as u8,
                                flat[i + 4] as u8,
                            );
                            i += 4;
                        }
                    }
                }
                49 => self.cur_bg = theme::BASE,
                90..=97 => {
                    self.cur_fg = ANSI_16[(flat[i] - 90 + 8) as usize];
                }
                100..=107 => {
                    self.cur_bg = ANSI_16[(flat[i] - 100 + 8) as usize];
                }
                _ => {}
            }
            i += 1;
        }
    }

    fn enter_alt_screen(&mut self) {
        if self.alt_cells.is_none() {
            self.alt_saved_cursor = (self.cursor_col, self.cursor_row);
            let primary = std::mem::replace(
                &mut self.cells,
                vec![vec![Cell::default(); self.cols]; self.rows],
            );
            self.alt_cells = Some(primary);
            self.cursor_col = 0;
            self.cursor_row = 0;
            self.scroll_offset = 0;
        }
    }

    fn leave_alt_screen(&mut self) {
        if let Some(primary) = self.alt_cells.take() {
            self.cells = primary;
            self.cursor_col = self.alt_saved_cursor.0.min(self.cols - 1);
            self.cursor_row = self.alt_saved_cursor.1.min(self.rows - 1);
            self.wrap_pending = false;
        }
    }
}

impl Perform for TerminalGrid {
    fn print(&mut self, ch: char) {
        self.scroll_offset = 0;
        if self.wrap_pending {
            self.cursor_col = 0;
            self.newline();
        }

        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            let fg = self.effective_fg();
            self.cells[self.cursor_row][self.cursor_col] = Cell {
                ch,
                fg,
                bg: self.cur_bg,
                bold: self.cur_bold,
                underline: self.cur_underline,
                inverse: self.cur_inverse,
            };
        }

        if self.cursor_col + 1 >= self.cols {
            self.wrap_pending = true;
        } else {
            self.cursor_col += 1;
        }
        self.dirty = true;
    }

    fn execute(&mut self, byte: u8) {
        self.dirty = true;
        match byte {
            0x08 => {
                // Backspace
                self.wrap_pending = false;
                self.cursor_col = self.cursor_col.saturating_sub(1);
            }
            0x09 => {
                // Horizontal tab
                self.wrap_pending = false;
                let next_tab = ((self.cursor_col / 8) + 1) * 8;
                self.cursor_col = next_tab.min(self.cols - 1);
            }
            0x0A..=0x0C => {
                // LF, VT, FF
                self.newline();
            }
            0x0D => {
                // CR
                self.wrap_pending = false;
                self.cursor_col = 0;
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        self.dirty = true;
        match (intermediates, byte) {
            ([], b'7') => self.saved_cursor = (self.cursor_col, self.cursor_row),
            ([], b'8') => {
                self.cursor_col = self.saved_cursor.0.min(self.cols - 1);
                self.cursor_row = self.saved_cursor.1.min(self.rows - 1);
            }
            ([], b'D') => self.newline(),
            ([], b'E') => {
                self.cursor_col = 0;
                self.newline();
            }
            ([], b'M') => {
                if self.cursor_row == self.scroll_top {
                    self.scroll_down_region(self.scroll_top, self.scroll_bottom, 1);
                } else {
                    self.cursor_row = self.cursor_row.saturating_sub(1);
                }
            }
            ([], b'c') => {
                self.reset_sgr();
                let blank = Cell::default();
                for row in &mut self.cells {
                    row.fill(blank);
                }
                self.cursor_col = 0;
                self.cursor_row = 0;
                self.scroll_top = 0;
                self.scroll_bottom = self.rows - 1;
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        self.dirty = true;
        self.wrap_pending = false;

        let p0 = |default: usize| -> usize {
            params
                .iter()
                .next()
                .and_then(|s| s.first().copied())
                .map(|v| if v == 0 { default } else { v as usize })
                .unwrap_or(default)
        };
        let p_raw = |idx: usize, default: usize| -> usize {
            params
                .iter()
                .nth(idx)
                .and_then(|s| s.first().copied())
                .map(|v| v as usize)
                .unwrap_or(default)
        };

        let is_private = intermediates.first() == Some(&b'?');

        match action {
            'A' => {
                let n = p0(1);
                self.cursor_row = self.cursor_row.saturating_sub(n);
            }
            'B' => {
                let n = p0(1);
                self.cursor_row = (self.cursor_row + n).min(self.rows - 1);
            }
            'C' => {
                let n = p0(1);
                self.cursor_col = (self.cursor_col + n).min(self.cols - 1);
            }
            'D' => {
                let n = p0(1);
                self.cursor_col = self.cursor_col.saturating_sub(n);
            }
            'E' => {
                let n = p0(1);
                self.cursor_row = (self.cursor_row + n).min(self.rows - 1);
                self.cursor_col = 0;
            }
            'F' => {
                let n = p0(1);
                self.cursor_row = self.cursor_row.saturating_sub(n);
                self.cursor_col = 0;
            }
            'G' | '`' => {
                let col = p0(1).saturating_sub(1);
                self.cursor_col = col.min(self.cols - 1);
            }
            'd' => {
                let row = p0(1).saturating_sub(1);
                self.cursor_row = row.min(self.rows - 1);
            }
            'H' | 'f' => {
                let row = p_raw(0, 1).max(1) - 1;
                let col = p_raw(1, 1).max(1) - 1;
                self.cursor_row = row.min(self.rows - 1);
                self.cursor_col = col.min(self.cols - 1);
            }
            'J' => {
                let mode = p_raw(0, 0);
                let blank = self.blank_cell_with_bg();
                match mode {
                    0 => {
                        for c in self.cursor_col..self.cols {
                            self.cells[self.cursor_row][c] = blank;
                        }
                        for r in (self.cursor_row + 1)..self.rows {
                            self.cells[r].fill(blank);
                        }
                    }
                    1 => {
                        for r in 0..self.cursor_row {
                            self.cells[r].fill(blank);
                        }
                        for c in 0..=self.cursor_col.min(self.cols - 1) {
                            self.cells[self.cursor_row][c] = blank;
                        }
                    }
                    2 | 3 => {
                        for r in 0..self.rows {
                            self.cells[r].fill(blank);
                        }
                        if mode == 3 {
                            self.scrollback.clear();
                            self.scroll_offset = 0;
                        }
                    }
                    _ => {}
                }
            }
            'K' => {
                let mode = p_raw(0, 0);
                let blank = self.blank_cell_with_bg();
                match mode {
                    0 => {
                        for c in self.cursor_col..self.cols {
                            self.cells[self.cursor_row][c] = blank;
                        }
                    }
                    1 => {
                        for c in 0..=self.cursor_col.min(self.cols - 1) {
                            self.cells[self.cursor_row][c] = blank;
                        }
                    }
                    2 => {
                        self.cells[self.cursor_row].fill(blank);
                    }
                    _ => {}
                }
            }
            'L' => {
                let n = p0(1);
                if self.cursor_row >= self.scroll_top && self.cursor_row <= self.scroll_bottom {
                    self.scroll_down_region(self.cursor_row, self.scroll_bottom, n);
                }
            }
            'M' => {
                let n = p0(1);
                if self.cursor_row >= self.scroll_top && self.cursor_row <= self.scroll_bottom {
                    self.scroll_up_region(self.cursor_row, self.scroll_bottom, n);
                }
            }
            'P' => {
                let n = p0(1);
                let blank = self.blank_cell_with_bg();
                let row = &mut self.cells[self.cursor_row];
                for _ in 0..n {
                    if self.cursor_col < row.len() {
                        row.remove(self.cursor_col);
                        row.push(blank);
                    }
                }
            }
            '@' => {
                let n = p0(1);
                let blank = self.blank_cell_with_bg();
                let row = &mut self.cells[self.cursor_row];
                for _ in 0..n {
                    if self.cursor_col < row.len() {
                        row.insert(self.cursor_col, blank);
                        row.truncate(self.cols);
                    }
                }
            }
            'X' => {
                let n = p0(1);
                let blank = self.blank_cell_with_bg();
                let end = (self.cursor_col + n).min(self.cols);
                for c in self.cursor_col..end {
                    self.cells[self.cursor_row][c] = blank;
                }
            }
            'S' => {
                let n = p0(1);
                self.scroll_up_region(self.scroll_top, self.scroll_bottom, n);
            }
            'T' => {
                let n = p0(1);
                self.scroll_down_region(self.scroll_top, self.scroll_bottom, n);
            }
            'r' => {
                let top = p_raw(0, 1).max(1) - 1;
                let bottom = p_raw(1, self.rows).max(1) - 1;
                if top < bottom && bottom < self.rows {
                    self.scroll_top = top;
                    self.scroll_bottom = bottom;
                    self.cursor_col = 0;
                    self.cursor_row = 0;
                }
            }
            's' => {
                self.saved_cursor = (self.cursor_col, self.cursor_row);
            }
            'u' => {
                self.cursor_col = self.saved_cursor.0.min(self.cols - 1);
                self.cursor_row = self.saved_cursor.1.min(self.rows - 1);
            }
            'm' => self.handle_sgr(params),
            'h' if is_private => {
                for sub in params.iter() {
                    if let Some(&code) = sub.first() {
                        match code {
                            1 => self.app_cursor_keys = true,
                            25 => self.cursor_visible = true,
                            47 | 1047 | 1049 => self.enter_alt_screen(),
                            _ => {}
                        }
                    }
                }
            }
            'l' if is_private => {
                for sub in params.iter() {
                    if let Some(&code) = sub.first() {
                        match code {
                            1 => self.app_cursor_keys = false,
                            25 => self.cursor_visible = false,
                            47 | 1047 | 1049 => self.leave_alt_screen(),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.len() >= 2 && (params[0] == b"0" || params[0] == b"2") {
            if let Ok(new_title) = std::str::from_utf8(params[1]) {
                let cleaned = new_title.trim();
                if !cleaned.is_empty() {
                    self.title = cleaned.to_string();
                    self.dirty = true;
                }
            }
        }
    }
}
