#[derive(Clone, Copy, PartialEq)]
enum PpuMode {
    OamScan,   // Mode 2
    Drawing,   // Mode 3
    HBlank,    // Mode 0
    VBlank,    // Mode 1
}

#[derive(Clone, Copy)]
struct FifoPixel {
    color: u8,       // 2-bit color number (0-3)
    palette: u8,     // palette register value
    bg_priority: bool, // OAM BG-over-OBJ flag
    is_sprite: bool,
}

impl FifoPixel {
    fn blank() -> Self {
        FifoPixel { color: 0, palette: 0, bg_priority: false, is_sprite: false }
    }
}

struct PixelFifo {
    pixels: [FifoPixel; 16],
    head: u8,
    len: u8,
}

impl PixelFifo {
    fn new() -> Self {
        PixelFifo { pixels: [FifoPixel::blank(); 16], head: 0, len: 0 }
    }

    fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }

    fn len(&self) -> u8 {
        self.len
    }

    fn push_row(&mut self, row: [FifoPixel; 8]) {
        for p in row {
            let idx = (self.head + self.len) & 15;
            self.pixels[idx as usize] = p;
            self.len += 1;
        }
    }

    fn pop(&mut self) -> FifoPixel {
        let p = self.pixels[self.head as usize];
        self.head = (self.head + 1) & 15;
        self.len -= 1;
        p
    }
}

#[derive(Clone, Copy, PartialEq)]
enum FetcherState {
    ReadTileId,
    ReadTileDataLow,
    ReadTileDataHigh,
    Push,
}

struct Fetcher {
    state: FetcherState,
    tick: u8,           // counts 0/1 within each state (2 T-cycles per state)
    tile_index: u8,     // tile ID read from tilemap
    tile_data_low: u8,
    tile_data_high: u8,
    tile_x: u8,         // current tile column in tilemap
    fetching_window: bool,
}

impl Fetcher {
    fn new() -> Self {
        Fetcher {
            state: FetcherState::ReadTileId,
            tick: 0,
            tile_index: 0,
            tile_data_low: 0,
            tile_data_high: 0,
            tile_x: 0,
            fetching_window: false,
        }
    }

    fn reset(&mut self) {
        self.state = FetcherState::ReadTileId;
        self.tick = 0;
        self.tile_index = 0;
        self.tile_data_low = 0;
        self.tile_data_high = 0;
    }
}

#[derive(Clone, Copy)]
struct SpriteEntry {
    oam_index: u8,
    x: u8,
    y: u8,
    tile: u8,
    flags: u8,
}

impl SpriteEntry {
    fn blank() -> Self {
        SpriteEntry { oam_index: 0, x: 0, y: 0, tile: 0, flags: 0 }
    }
}

pub struct Ppu {
    pub framebuffer: [u8; 160 * 144],
    mode: PpuMode,
    mode_clock: u32,
    pub ly: u8,
    pub lyc: u8,
    pub lcdc: u8,
    stat: u8,
    pub scy: u8,
    pub scx: u8,
    pub bgp: u8,
    pub wy: u8,
    pub wx: u8,
    pub obp0: u8,
    pub obp1: u8,
    pub vblank_interrupt: bool,
    pub stat_interrupt: bool,

    // Pixel FIFO fields
    bg_fifo: PixelFifo,
    obj_fifo: PixelFifo,
    fetcher: Fetcher,
    scanline_sprites: [SpriteEntry; 10],
    sprite_count: u8,
    pixel_x: u8,
    scx_discard: u8,
    window_line_counter: u8,
    window_active: bool,
    wy_triggered: bool,
    sprite_fetching: bool,
    sprite_fetch_step: u8,
    sprite_fetch_idx: u8,  // index into scanline_sprites
    sprite_tile_data_low: u8,
    sprite_tile_data_high: u8,
    drawing_cycles: u32,
    oam_scan_index: u8, // OAM entry being scanned (0-39)
    oam_scan_tick: u8,   // 0 or 1 within each 2-T-cycle OAM check
}

impl Ppu {
    pub fn read_stat(&self) -> u8 {
        let mode_bits = match self.mode {
            PpuMode::HBlank => 0,
            PpuMode::VBlank => 1,
            PpuMode::OamScan => 2,
            PpuMode::Drawing => 3,
        };
        let lyc_flag = if self.ly == self.lyc { 0x04 } else { 0 };
        (self.stat & 0xF8) | lyc_flag | mode_bits
    }

    pub fn write_stat(&mut self, byte: u8) {
        self.stat = (byte & 0xF8) | (self.stat & 0x07);
    }

    pub fn tick(&mut self, t_cycles: u8, vram: &[u8], oam: &[u8]) {
        self.vblank_interrupt = false;
        self.stat_interrupt = false;

        if self.lcdc & 0x80 == 0 {
            return;
        }

        let mut remaining = t_cycles as u32;
        while remaining > 0 {
            match self.mode {
                PpuMode::Drawing => {
                    self.mode_clock += 1;
                    self.tick_drawing(vram, oam);
                    remaining -= 1;
                }
                PpuMode::OamScan => {
                    let until_end = 80u32.saturating_sub(self.mode_clock);
                    let consume = remaining.min(until_end);
                    self.mode_clock += consume;
                    remaining -= consume;
                    if self.mode_clock >= 80 {
                        self.do_full_oam_scan(oam);
                        self.start_drawing();
                    }
                }
                PpuMode::HBlank => {
                    let until_end = 456u32.saturating_sub(self.mode_clock);
                    let consume = remaining.min(until_end);
                    self.mode_clock += consume;
                    remaining -= consume;
                    if self.mode_clock >= 456 {
                        self.mode_clock -= 456;
                        self.ly += 1;
                        if self.window_active {
                            self.window_line_counter += 1;
                        }
                        if self.ly == 144 {
                            self.mode = PpuMode::VBlank;
                            self.vblank_interrupt = true;
                            self.check_stat_interrupt(1);
                        } else {
                            self.mode = PpuMode::OamScan;
                            self.check_stat_interrupt(2);
                        }
                        self.check_lyc();
                    }
                }
                PpuMode::VBlank => {
                    let until_end = 456u32.saturating_sub(self.mode_clock);
                    let consume = remaining.min(until_end);
                    self.mode_clock += consume;
                    remaining -= consume;
                    if self.mode_clock >= 456 {
                        self.mode_clock -= 456;
                        self.ly += 1;
                        if self.ly > 153 {
                            self.ly = 0;
                            self.mode = PpuMode::OamScan;
                            self.window_line_counter = 0;
                            self.wy_triggered = false;
                            self.check_stat_interrupt(2);
                        }
                        self.check_lyc();
                    }
                }
            }
        }
    }

    fn do_full_oam_scan(&mut self, oam: &[u8]) {
        self.sprite_count = 0;
        let sprite_height: u8 = if self.lcdc & 0x04 != 0 { 16 } else { 8 };
        for i in 0..40u8 {
            if self.sprite_count >= 10 { break; }
            let base = i as usize * 4;
            let sy = oam[base];
            let sx = oam[base + 1];
            let tile = oam[base + 2];
            let flags = oam[base + 3];
            let screen_y = sy.wrapping_sub(16);
            if self.ly >= screen_y && self.ly < screen_y.wrapping_add(sprite_height) {
                self.scanline_sprites[self.sprite_count as usize] = SpriteEntry {
                    oam_index: i, x: sx, y: sy, tile, flags,
                };
                self.sprite_count += 1;
            }
        }
    }

    fn start_drawing(&mut self) {
        self.mode = PpuMode::Drawing;
        self.bg_fifo.clear();
        self.obj_fifo.clear();
        self.pixel_x = 0;
        self.scx_discard = self.scx & 7;
        self.fetcher.reset();
        self.fetcher.tile_x = self.scx / 8;
        self.fetcher.fetching_window = false;
        self.window_active = false;
        self.sprite_fetching = false;

        // Check if window Y condition met on this frame
        if self.ly == self.wy {
            self.wy_triggered = true;
        }
    }

    // --- Drawing (Mode 3): variable length ---

    #[inline(always)]
    fn tick_drawing(&mut self, vram: &[u8], oam: &[u8]) {
        if self.sprite_fetching {
            self.tick_sprite_fetch(vram);
            return;
        }

        // Tick BG/window fetcher first so a Push fills the FIFO before sprite check
        self.tick_fetcher(vram);

        // Check sprite trigger — must happen after fetcher (so FIFO has data on push
        // cycles) but before pixel output (so sprites aren't skipped)
        if self.lcdc & 0x02 != 0 && self.bg_fifo.len() > 0 {
            if self.check_sprite_trigger() {
                self.tick_sprite_fetch(vram);
                return;
            }
        }

        // Try to push a pixel to the framebuffer
        self.try_push_pixel(oam);
    }

    // --- BG/Window Fetcher state machine (2 T-cycles per state) ---

    #[inline(always)]
    fn tick_fetcher(&mut self, vram: &[u8]) {
        self.fetcher.tick += 1;
        if self.fetcher.tick < 2 {
            return;
        }
        self.fetcher.tick = 0;

        match self.fetcher.state {
            FetcherState::ReadTileId => {
                let tile_map_base: u16 = if self.fetcher.fetching_window {
                    if self.lcdc & 0x40 != 0 { 0x1C00 } else { 0x1800 }
                } else {
                    if self.lcdc & 0x08 != 0 { 0x1C00 } else { 0x1800 }
                };

                let y = if self.fetcher.fetching_window {
                    self.window_line_counter
                } else {
                    self.ly.wrapping_add(self.scy)
                };

                let tile_row = (y / 8) as u16;
                let tile_col = (self.fetcher.tile_x & 31) as u16;
                let map_addr = tile_map_base + tile_row * 32 + tile_col;
                self.fetcher.tile_index = vram[map_addr as usize];
                self.fetcher.state = FetcherState::ReadTileDataLow;
            }
            FetcherState::ReadTileDataLow => {
                let addr = self.tile_data_addr();
                self.fetcher.tile_data_low = vram[addr as usize];
                self.fetcher.state = FetcherState::ReadTileDataHigh;
            }
            FetcherState::ReadTileDataHigh => {
                let addr = self.tile_data_addr() + 1;
                self.fetcher.tile_data_high = vram[addr as usize];
                self.fetcher.state = FetcherState::Push;
            }
            FetcherState::Push => {
                if self.bg_fifo.len() > 0 {
                    // Stall — wait until FIFO has space
                    self.fetcher.tick = 0;
                    return;
                }
                let mut row = [FifoPixel::blank(); 8];
                for bit in 0..8u8 {
                    let shift = 7 - bit;
                    let lo = (self.fetcher.tile_data_low >> shift) & 1;
                    let hi = (self.fetcher.tile_data_high >> shift) & 1;
                    let color = (hi << 1) | lo;
                    row[bit as usize] = FifoPixel {
                        color,
                        palette: 0, // BG uses bgp, resolved at output
                        bg_priority: false,
                        is_sprite: false,
                    };
                }
                self.bg_fifo.push_row(row);
                self.fetcher.tile_x = self.fetcher.tile_x.wrapping_add(1);
                self.fetcher.state = FetcherState::ReadTileId;
            }
        }
    }

    fn tile_data_addr(&self) -> u16 {
        let signed_addressing = self.lcdc & 0x10 == 0;
        let y = if self.fetcher.fetching_window {
            self.window_line_counter
        } else {
            self.ly.wrapping_add(self.scy)
        };
        let pixel_row = (y % 8) as u16;

        if signed_addressing {
            let signed_index = self.fetcher.tile_index as i8 as i16;
            (0x0800i16 + (signed_index + 128) * 16 + pixel_row as i16 * 2) as u16
        } else {
            self.fetcher.tile_index as u16 * 16 + pixel_row * 2
        }
    }

    // --- Pixel output ---

    #[inline(always)]
    fn try_push_pixel(&mut self, _oam: &[u8]) {
        if self.bg_fifo.len() == 0 {
            return;
        }

        let bg_pixel = self.bg_fifo.pop();

        // Discard SCX % 8 pixels at start of scanline (BG only — sprites are absolute)
        if self.scx_discard > 0 {
            self.scx_discard -= 1;
            return;
        }

        if self.pixel_x >= 160 {
            return;
        }

        // Get sprite pixel if available
        let obj_pixel = if self.obj_fifo.len() > 0 {
            Some(self.obj_fifo.pop())
        } else {
            None
        };

        // Resolve final color
        let fb_idx = self.ly as usize * 160 + self.pixel_x as usize;
        let bg_enabled = self.lcdc & 0x01 != 0;

        let bg_color_num = if bg_enabled { bg_pixel.color } else { 0 };
        let bg_color = (self.bgp >> (bg_color_num * 2)) & 0x03;

        let final_color = if let Some(op) = obj_pixel {
            if op.color == 0 || !op.is_sprite {
                // Sprite transparent
                bg_color
            } else if op.bg_priority && bg_color_num != 0 {
                // BG-over-OBJ and BG is not color 0
                bg_color
            } else {
                (op.palette >> (op.color * 2)) & 0x03
            }
        } else {
            bg_color
        };

        self.framebuffer[fb_idx] = final_color;
        self.pixel_x += 1;

        // Check window trigger
        if !self.window_active && self.wy_triggered && self.lcdc & 0x20 != 0 {
            if self.wx <= 166 && self.pixel_x >= self.wx.wrapping_sub(7) {
                self.activate_window();
            }
        }

        // Check if scanline is done
        if self.pixel_x >= 160 {
            self.mode = PpuMode::HBlank;
            self.check_stat_interrupt(0);
        }
    }

    // --- Window activation ---

    fn activate_window(&mut self) {
        self.window_active = true;
        self.bg_fifo.clear();
        self.fetcher.reset();
        self.fetcher.tile_x = 0;
        self.fetcher.fetching_window = true;
    }

    // --- Sprite fetching ---

    fn check_sprite_trigger(&mut self) -> bool {
        let check_x = self.pixel_x;
        for i in 0..self.sprite_count {
            let sprite = &self.scanline_sprites[i as usize];
            if sprite.x == 0 {
                continue; // consumed or fully off-screen
            }
            // Sprite screen X = sprite.x - 8; clamp to 0 for partially off-screen sprites
            let trigger_x = if sprite.x >= 8 { sprite.x - 8 } else { 0 };
            if trigger_x == check_x {
                self.sprite_fetching = true;
                self.sprite_fetch_step = 0;
                self.sprite_fetch_idx = i;
                return true;
            }
        }
        false
    }

    fn tick_sprite_fetch(&mut self, vram: &[u8]) {
        self.sprite_fetch_step += 1;

        // 6 T-cycles total for sprite fetch (3 steps × 2 T-cycles)
        if self.sprite_fetch_step == 2 {
            // Step 1 complete: read tile ID (already in OAM entry)
        } else if self.sprite_fetch_step == 4 {
            // Step 2 complete: read tile data low
            let sprite = self.scanline_sprites[self.sprite_fetch_idx as usize];
            let sprite_height: u8 = if self.lcdc & 0x04 != 0 { 16 } else { 8 };
            let y_flip = sprite.flags & 0x40 != 0;

            let mut row = self.ly.wrapping_sub(sprite.y.wrapping_sub(16));
            let tile = if sprite_height == 16 {
                if y_flip { row = sprite_height - 1 - row; }
                if row >= 8 { (sprite.tile | 0x01, row - 8) } else { (sprite.tile & 0xFE, row) }
            } else {
                if y_flip { row = 7 - row; }
                (sprite.tile, row)
            };

            let addr = tile.0 as u16 * 16 + tile.1 as u16 * 2;
            self.sprite_tile_data_low = vram[addr as usize];
            self.sprite_tile_data_high = vram[(addr + 1) as usize];
        } else if self.sprite_fetch_step >= 6 {
            // Step 3 complete: mix into obj_fifo
            self.mix_sprite_pixels();
            self.sprite_fetching = false;

            // Mark sprite as consumed by setting x=0
            self.scanline_sprites[self.sprite_fetch_idx as usize].x = 0;

            // Check if another sprite triggers at same pixel_x
            if self.lcdc & 0x02 != 0 && self.check_sprite_trigger() {
                return; // Continue with next sprite fetch
            }
        }
    }

    fn mix_sprite_pixels(&mut self) {
        let sprite = self.scanline_sprites[self.sprite_fetch_idx as usize];
        let x_flip = sprite.flags & 0x20 != 0;
        let palette = if sprite.flags & 0x10 != 0 { self.obp1 } else { self.obp0 };
        let bg_priority = sprite.flags & 0x80 != 0;

        // Sprites with X < 8 are partially off the left edge — clip leading pixels
        let clip_left = if sprite.x < 8 { 8 - sprite.x } else { 0 };
        let pixels_to_write = 8 - clip_left;

        // Ensure obj_fifo has at least pixels_to_write entries (pad with transparent)
        while self.obj_fifo.len() < pixels_to_write {
            let idx = (self.obj_fifo.head + self.obj_fifo.len) & 15;
            self.obj_fifo.pixels[idx as usize] = FifoPixel { color: 0, palette: 0, bg_priority: false, is_sprite: false };
            self.obj_fifo.len += 1;
        }

        // Overlay sprite pixels — only write if existing pixel is transparent (first sprite wins)
        for i in 0..pixels_to_write {
            let bit = clip_left + i;
            let shift = if x_flip { bit } else { 7 - bit };
            let lo = (self.sprite_tile_data_low >> shift) & 1;
            let hi = (self.sprite_tile_data_high >> shift) & 1;
            let color = (hi << 1) | lo;

            let fifo_idx = (self.obj_fifo.head + i) & 15;
            let existing = &self.obj_fifo.pixels[fifo_idx as usize];
            if !existing.is_sprite || existing.color == 0 {
                self.obj_fifo.pixels[fifo_idx as usize] = FifoPixel {
                    color,
                    palette,
                    bg_priority,
                    is_sprite: true,
                };
            }
        }
    }

    // --- STAT interrupt helpers ---

    fn check_lyc(&mut self) {
        if self.ly == self.lyc && self.stat & 0x40 != 0 {
            self.stat_interrupt = true;
        }
    }

    fn check_stat_interrupt(&mut self, mode: u8) {
        let bit = match mode {
            0 => 0x08,
            1 => 0x10,
            2 => 0x20,
            _ => 0,
        };
        if self.stat & bit != 0 {
            self.stat_interrupt = true;
        }
    }
}

impl Ppu {
    pub fn save_state(&self, buf: &mut Vec<u8>) {
        use crate::savestate::*;
        write_bytes(buf, &self.framebuffer);
        let mode_byte = match self.mode {
            PpuMode::HBlank => 0u8,
            PpuMode::VBlank => 1,
            PpuMode::OamScan => 2,
            PpuMode::Drawing => 3,
        };
        write_u8(buf, mode_byte);
        write_u32_le(buf, self.mode_clock);
        write_u8(buf, self.ly);
        write_u8(buf, self.lyc);
        write_u8(buf, self.lcdc);
        write_u8(buf, self.stat);
        write_u8(buf, self.scy);
        write_u8(buf, self.scx);
        write_u8(buf, self.bgp);
        write_u8(buf, self.wy);
        write_u8(buf, self.wx);
        write_u8(buf, self.obp0);
        write_u8(buf, self.obp1);
        write_bool(buf, self.vblank_interrupt);
        write_bool(buf, self.stat_interrupt);

        // FIFO state (v0x03)
        write_u8(buf, self.pixel_x);
        write_u8(buf, self.scx_discard);
        write_u8(buf, self.window_line_counter);
        write_bool(buf, self.window_active);
        write_bool(buf, self.wy_triggered);
        write_bool(buf, self.sprite_fetching);
        write_u8(buf, self.sprite_fetch_step);
        write_u8(buf, self.sprite_fetch_idx);
        write_u8(buf, self.sprite_tile_data_low);
        write_u8(buf, self.sprite_tile_data_high);
        write_u32_le(buf, self.drawing_cycles);
        write_u8(buf, self.oam_scan_index);
        write_u8(buf, self.oam_scan_tick);
        write_u8(buf, self.sprite_count);
        for i in 0..10 {
            let s = &self.scanline_sprites[i];
            write_u8(buf, s.oam_index);
            write_u8(buf, s.x);
            write_u8(buf, s.y);
            write_u8(buf, s.tile);
            write_u8(buf, s.flags);
        }
        // Fetcher
        write_u8(buf, match self.fetcher.state {
            FetcherState::ReadTileId => 0,
            FetcherState::ReadTileDataLow => 1,
            FetcherState::ReadTileDataHigh => 2,
            FetcherState::Push => 3,
        });
        write_u8(buf, self.fetcher.tick);
        write_u8(buf, self.fetcher.tile_index);
        write_u8(buf, self.fetcher.tile_data_low);
        write_u8(buf, self.fetcher.tile_data_high);
        write_u8(buf, self.fetcher.tile_x);
        write_bool(buf, self.fetcher.fetching_window);
        // BG FIFO
        write_u8(buf, self.bg_fifo.head);
        write_u8(buf, self.bg_fifo.len);
        for i in 0..16 {
            let p = &self.bg_fifo.pixels[i];
            write_u8(buf, p.color);
            write_u8(buf, p.palette);
            write_bool(buf, p.bg_priority);
            write_bool(buf, p.is_sprite);
        }
        // OBJ FIFO
        write_u8(buf, self.obj_fifo.head);
        write_u8(buf, self.obj_fifo.len);
        for i in 0..16 {
            let p = &self.obj_fifo.pixels[i];
            write_u8(buf, p.color);
            write_u8(buf, p.palette);
            write_bool(buf, p.bg_priority);
            write_bool(buf, p.is_sprite);
        }
    }

    pub fn load_state(&mut self, data: &[u8], cursor: &mut usize) {
        use crate::savestate::*;
        let fb = read_bytes(data, cursor, 160 * 144);
        self.framebuffer.copy_from_slice(fb);
        self.mode = match read_u8(data, cursor) {
            0 => PpuMode::HBlank,
            1 => PpuMode::VBlank,
            2 => PpuMode::OamScan,
            _ => PpuMode::Drawing,
        };
        self.mode_clock = read_u32_le(data, cursor);
        self.ly = read_u8(data, cursor);
        self.lyc = read_u8(data, cursor);
        self.lcdc = read_u8(data, cursor);
        self.stat = read_u8(data, cursor);
        self.scy = read_u8(data, cursor);
        self.scx = read_u8(data, cursor);
        self.bgp = read_u8(data, cursor);
        self.wy = read_u8(data, cursor);
        self.wx = read_u8(data, cursor);
        self.obp0 = read_u8(data, cursor);
        self.obp1 = read_u8(data, cursor);
        self.vblank_interrupt = read_bool(data, cursor);
        self.stat_interrupt = read_bool(data, cursor);

        // FIFO state (v0x03)
        self.pixel_x = read_u8(data, cursor);
        self.scx_discard = read_u8(data, cursor);
        self.window_line_counter = read_u8(data, cursor);
        self.window_active = read_bool(data, cursor);
        self.wy_triggered = read_bool(data, cursor);
        self.sprite_fetching = read_bool(data, cursor);
        self.sprite_fetch_step = read_u8(data, cursor);
        self.sprite_fetch_idx = read_u8(data, cursor);
        self.sprite_tile_data_low = read_u8(data, cursor);
        self.sprite_tile_data_high = read_u8(data, cursor);
        self.drawing_cycles = read_u32_le(data, cursor);
        self.oam_scan_index = read_u8(data, cursor);
        self.oam_scan_tick = read_u8(data, cursor);
        self.sprite_count = read_u8(data, cursor);
        for i in 0..10 {
            self.scanline_sprites[i] = SpriteEntry {
                oam_index: read_u8(data, cursor),
                x: read_u8(data, cursor),
                y: read_u8(data, cursor),
                tile: read_u8(data, cursor),
                flags: read_u8(data, cursor),
            };
        }
        // Fetcher
        self.fetcher.state = match read_u8(data, cursor) {
            0 => FetcherState::ReadTileId,
            1 => FetcherState::ReadTileDataLow,
            2 => FetcherState::ReadTileDataHigh,
            _ => FetcherState::Push,
        };
        self.fetcher.tick = read_u8(data, cursor);
        self.fetcher.tile_index = read_u8(data, cursor);
        self.fetcher.tile_data_low = read_u8(data, cursor);
        self.fetcher.tile_data_high = read_u8(data, cursor);
        self.fetcher.tile_x = read_u8(data, cursor);
        self.fetcher.fetching_window = read_bool(data, cursor);
        // BG FIFO
        self.bg_fifo.head = read_u8(data, cursor);
        self.bg_fifo.len = read_u8(data, cursor);
        for i in 0..16 {
            self.bg_fifo.pixels[i] = FifoPixel {
                color: read_u8(data, cursor),
                palette: read_u8(data, cursor),
                bg_priority: read_bool(data, cursor),
                is_sprite: read_bool(data, cursor),
            };
        }
        // OBJ FIFO
        self.obj_fifo.head = read_u8(data, cursor);
        self.obj_fifo.len = read_u8(data, cursor);
        for i in 0..16 {
            self.obj_fifo.pixels[i] = FifoPixel {
                color: read_u8(data, cursor),
                palette: read_u8(data, cursor),
                bg_priority: read_bool(data, cursor),
                is_sprite: read_bool(data, cursor),
            };
        }
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Ppu {
            framebuffer: [0; 160 * 144],
            mode: PpuMode::OamScan,
            mode_clock: 0,
            ly: 0,
            lyc: 0,
            lcdc: 0x91,
            stat: 0,
            scy: 0,
            scx: 0,
            bgp: 0xFC,
            wy: 0,
            wx: 0,
            obp0: 0xFF,
            obp1: 0xFF,
            vblank_interrupt: false,
            stat_interrupt: false,
            bg_fifo: PixelFifo::new(),
            obj_fifo: PixelFifo::new(),
            fetcher: Fetcher::new(),
            scanline_sprites: [SpriteEntry::blank(); 10],
            sprite_count: 0,
            pixel_x: 0,
            scx_discard: 0,
            window_line_counter: 0,
            window_active: false,
            wy_triggered: false,
            sprite_fetching: false,
            sprite_fetch_step: 0,
            sprite_fetch_idx: 0,
            sprite_tile_data_low: 0,
            sprite_tile_data_high: 0,
            drawing_cycles: 0,
            oam_scan_index: 0,
            oam_scan_tick: 0,
        }
    }
}
