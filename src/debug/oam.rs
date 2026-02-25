use minifb::{Window, WindowOptions};
use super::font;
use super::{BG_COLOR, TEXT_COLOR, HEADER_COLOR};

const WIN_W: usize = 560;
const WIN_H: usize = 340;
const SPRITES_PER_COL: usize = 20;

pub struct OamViewer {
    pub window: Window,
    buf: Vec<u32>,
}

impl OamViewer {
    pub fn new() -> Self {
        let window = Window::new(
            "OAM / Sprites",
            WIN_W,
            WIN_H,
            WindowOptions::default(),
        ).expect("Failed to create OAM viewer window");
        OamViewer {
            window,
            buf: vec![BG_COLOR; WIN_W * WIN_H],
        }
    }

    pub fn update(&mut self, vram: &[u8; 0x2000], oam: &[u8; 0xA0], obp0: u8, obp1: u8, palette: &[u32; 4]) {
        self.buf.fill(BG_COLOR);

        font::draw_string(&mut self.buf, WIN_W, 4, 2, "OAM SPRITES (40)", HEADER_COLOR);

        for i in 0..40 {
            let base = i * 4;
            let y_pos = oam[base] as i16 - 16;
            let x_pos = oam[base + 1] as i16 - 8;
            let tile_idx = oam[base + 2] as usize;
            let flags = oam[base + 3];

            let priority = (flags >> 7) & 1;
            let y_flip = (flags >> 6) & 1;
            let x_flip = (flags >> 5) & 1;
            let pal_num = (flags >> 4) & 1;

            // Column layout
            let col = i / SPRITES_PER_COL;
            let row = i % SPRITES_PER_COL;
            let base_x = 4 + col * 276;
            let base_y = 16 + row * 16;

            // Decode and draw sprite tile
            let obp = if pal_num == 0 { obp0 } else { obp1 };
            let pal = decode_obj_palette(obp, palette);
            let tile_data = decode_tile(vram, tile_idx * 16);
            draw_sprite(&mut self.buf, WIN_W, base_x, base_y, &tile_data, &pal, x_flip != 0, y_flip != 0);

            // Text info
            let info = format!(
                "#{:02} ({:>3},{:>3}) T:{:02X} {}{}{}{}",
                i, x_pos, y_pos, tile_idx,
                if priority != 0 { 'P' } else { '-' },
                if y_flip != 0 { 'Y' } else { '-' },
                if x_flip != 0 { 'X' } else { '-' },
                if pal_num != 0 { '1' } else { '0' },
            );
            font::draw_string(&mut self.buf, WIN_W, base_x + 12, base_y + 1, &info, TEXT_COLOR);
        }

        self.window.update_with_buffer(&self.buf, WIN_W, WIN_H).ok();
    }

    pub fn is_open(&self) -> bool {
        self.window.is_open()
    }
}

fn decode_obj_palette(obp: u8, display_pal: &[u32; 4]) -> [u32; 4] {
    // Color 0 is transparent for sprites, but we render it as BG for the viewer
    [
        BG_COLOR,
        display_pal[((obp >> 2) & 0x03) as usize],
        display_pal[((obp >> 4) & 0x03) as usize],
        display_pal[((obp >> 6) & 0x03) as usize],
    ]
}

fn decode_tile(vram: &[u8], addr: usize) -> [u8; 64] {
    let mut pixels = [0u8; 64];
    for row in 0..8 {
        let byte1 = vram.get(addr + row * 2).copied().unwrap_or(0);
        let byte2 = vram.get(addr + row * 2 + 1).copied().unwrap_or(0);
        for col in 0..8 {
            let bit = 7 - col;
            let lo = (byte1 >> bit) & 1;
            let hi = (byte2 >> bit) & 1;
            pixels[row * 8 + col] = (hi << 1) | lo;
        }
    }
    pixels
}

fn draw_sprite(
    buf: &mut [u32], buf_w: usize, x: usize, y: usize,
    pixels: &[u8; 64], pal: &[u32; 4], x_flip: bool, y_flip: bool,
) {
    for row in 0..8 {
        for col in 0..8 {
            let src_row = if y_flip { 7 - row } else { row };
            let src_col = if x_flip { 7 - col } else { col };
            let color_idx = pixels[src_row * 8 + src_col] as usize;
            let px = x + col;
            let py = y + row;
            if px < buf_w && py * buf_w + px < buf.len() {
                buf[py * buf_w + px] = pal[color_idx];
            }
        }
    }
}
