#[derive(Clone, Copy, PartialEq)]
enum PpuMode {
    OamScan,   // Mode 2
    Drawing,   // Mode 3
    HBlank,    // Mode 0
    VBlank,    // Mode 1
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

        self.mode_clock += t_cycles as u32;

        match self.mode {
            PpuMode::OamScan => {
                if self.mode_clock >= 80 {
                    self.mode_clock -= 80;
                    self.mode = PpuMode::Drawing;
                }
            }
            PpuMode::Drawing => {
                if self.mode_clock >= 172 {
                    self.mode_clock -= 172;
                    self.mode = PpuMode::HBlank;
                    self.render_scanline(vram, oam);
                    self.check_stat_interrupt(0);
                }
            }
            PpuMode::HBlank => {
                if self.mode_clock >= 204 {
                    self.mode_clock -= 204;
                    self.ly += 1;
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
                if self.mode_clock >= 456 {
                    self.mode_clock -= 456;
                    self.ly += 1;
                    if self.ly > 153 {
                        self.ly = 0;
                        self.mode = PpuMode::OamScan;
                        self.check_stat_interrupt(2);
                    }
                    self.check_lyc();
                }
            }
        }
    }

    fn check_lyc(&mut self) {
        if self.ly == self.lyc && self.stat & 0x40 != 0 {
            self.stat_interrupt = true;
        }
    }

    fn check_stat_interrupt(&mut self, mode: u8) {
        let bit = match mode {
            0 => 0x08, // HBlank
            1 => 0x10, // VBlank
            2 => 0x20, // OAM
            _ => 0,
        };
        if self.stat & bit != 0 {
            self.stat_interrupt = true;
        }
    }

    fn render_scanline(&mut self, vram: &[u8], oam: &[u8]) {
        if self.lcdc & 0x01 != 0 {
            self.render_bg_scanline(vram);
        }
        if self.lcdc & 0x20 != 0 {
            self.render_window_scanline(vram);
        }
        if self.lcdc & 0x02 != 0 {
            self.render_sprite_scanline(vram, oam);
        }
    }

    fn render_bg_scanline(&mut self, vram: &[u8]) {
        let tile_data_base: u16 = if self.lcdc & 0x10 != 0 { 0x0000 } else { 0x0800 };
        let tile_map_base: u16 = if self.lcdc & 0x08 != 0 { 0x1C00 } else { 0x1800 };
        let signed_addressing = self.lcdc & 0x10 == 0;

        let y = self.ly.wrapping_add(self.scy);
        let tile_row = (y / 8) as u16;
        let pixel_row = (y % 8) as u16;

        for x_pixel in 0u8..160 {
            let x = x_pixel.wrapping_add(self.scx);
            let tile_col = (x / 8) as u16;
            let pixel_col = (x % 8) as u16;

            let map_addr = tile_map_base + tile_row * 32 + tile_col;
            let tile_index = vram[map_addr as usize];

            let tile_addr = if signed_addressing {
                let signed_index = tile_index as i8 as i16;
                (tile_data_base as i16 + (signed_index + 128) * 16 + pixel_row as i16 * 2) as u16
            } else {
                tile_data_base + tile_index as u16 * 16 + pixel_row * 2
            };

            let byte1 = vram[tile_addr as usize];
            let byte2 = vram[(tile_addr + 1) as usize];

            let bit = 7 - pixel_col;
            let color_num = ((byte2 >> bit) & 1) << 1 | ((byte1 >> bit) & 1);
            let color = (self.bgp >> (color_num * 2)) & 0x03;

            let fb_idx = self.ly as usize * 160 + x_pixel as usize;
            self.framebuffer[fb_idx] = color;
        }
    }

    fn render_window_scanline(&mut self, vram: &[u8]) {
        if self.wy > self.ly {
            return;
        }
        let wx = self.wx.wrapping_sub(7);
        let tile_data_base: u16 = if self.lcdc & 0x10 != 0 { 0x0000 } else { 0x0800 };
        let tile_map_base: u16 = if self.lcdc & 0x40 != 0 { 0x1C00 } else { 0x1800 };
        let signed_addressing = self.lcdc & 0x10 == 0;

        let y = self.ly - self.wy;
        let tile_row = (y / 8) as u16;
        let pixel_row = (y % 8) as u16;

        for x_pixel in wx..160 {
            let x = x_pixel - wx;
            let tile_col = (x / 8) as u16;
            let pixel_col = (x % 8) as u16;

            let map_addr = tile_map_base + tile_row * 32 + tile_col;
            let tile_index = vram[map_addr as usize];

            let tile_addr = if signed_addressing {
                let signed_index = tile_index as i8 as i16;
                (tile_data_base as i16 + (signed_index + 128) * 16 + pixel_row as i16 * 2) as u16
            } else {
                tile_data_base + tile_index as u16 * 16 + pixel_row * 2
            };

            let byte1 = vram[tile_addr as usize];
            let byte2 = vram[(tile_addr + 1) as usize];

            let bit = 7 - pixel_col;
            let color_num = ((byte2 >> bit) & 1) << 1 | ((byte1 >> bit) & 1);
            let color = (self.bgp >> (color_num * 2)) & 0x03;

            let fb_idx = self.ly as usize * 160 + x_pixel as usize;
            self.framebuffer[fb_idx] = color;
        }
    }

    fn render_sprite_scanline(&mut self, vram: &[u8], oam: &[u8]) {
        let sprite_height: u8 = if self.lcdc & 0x04 != 0 { 16 } else { 8 };
        let mut sprites_on_line: Vec<(u8, u8, u8, u8)> = Vec::new(); // (x, y, tile, flags)

        for i in 0..40usize {
            let base = i * 4;
            let sy = oam[base];
            let sx = oam[base + 1];
            let tile = oam[base + 2];
            let flags = oam[base + 3];

            let screen_y = sy.wrapping_sub(16);
            if self.ly >= screen_y && self.ly < screen_y.wrapping_add(sprite_height) {
                sprites_on_line.push((sx, sy, tile, flags));
                if sprites_on_line.len() >= 10 {
                    break;
                }
            }
        }

        // Draw in reverse order so lower-x sprites have priority (drawn last)
        for &(sx, sy, tile, flags) in sprites_on_line.iter().rev() {
            let palette = if flags & 0x10 != 0 { self.obp1 } else { self.obp0 };
            let x_flip = flags & 0x20 != 0;
            let y_flip = flags & 0x40 != 0;
            let behind_bg = flags & 0x80 != 0;

            let mut row = self.ly.wrapping_sub(sy.wrapping_sub(16));
            let tile_num = if sprite_height == 16 {
                if y_flip { row = sprite_height - 1 - row; }
                if row >= 8 { (tile | 0x01, row - 8) } else { (tile & 0xFE, row) }
            } else {
                if y_flip { row = 7 - row; }
                (tile, row)
            };

            let tile_addr = tile_num.0 as u16 * 16 + tile_num.1 as u16 * 2;
            let byte1 = vram[tile_addr as usize];
            let byte2 = vram[(tile_addr + 1) as usize];

            for pixel in 0u8..8 {
                let screen_x = sx.wrapping_sub(8).wrapping_add(pixel);
                if screen_x >= 160 {
                    continue;
                }

                let bit = if x_flip { pixel } else { 7 - pixel };
                let color_num = ((byte2 >> bit) & 1) << 1 | ((byte1 >> bit) & 1);
                if color_num == 0 {
                    continue; // transparent
                }

                let fb_idx = self.ly as usize * 160 + screen_x as usize;
                if behind_bg && self.framebuffer[fb_idx] != 0 {
                    continue;
                }

                let color = (palette >> (color_num * 2)) & 0x03;
                self.framebuffer[fb_idx] = color;
            }
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
        }
    }
}
