use minifb::{Window, WindowOptions};
use super::font;
use super::{BG_COLOR, HEADER_COLOR};

const TILE_W: usize = 16; // tiles per row in atlas
const TILE_H: usize = 24; // tile rows in atlas (384 tiles)
const ATLAS_PX_H: usize = TILE_H * 8; // 192

// Window layout:
// Left: atlas (128px) + 8px gap + label area
// Bottom: two tile maps side by side (256x256 each, scaled to 128x128)
const WIN_W: usize = 520;
const WIN_H: usize = 480;

pub struct TileViewer {
    pub window: Window,
    buf: Vec<u32>,
}

impl TileViewer {
    pub fn new() -> Self {
        let window = Window::new(
            "Tiles / VRAM",
            WIN_W,
            WIN_H,
            WindowOptions::default(),
        ).expect("Failed to create tile viewer window");
        TileViewer {
            window,
            buf: vec![BG_COLOR; WIN_W * WIN_H],
        }
    }

    pub fn update(&mut self, vram: &[u8; 0x2000], bgp: u8, palette: &[u32; 4]) {
        self.buf.fill(BG_COLOR);

        // Map BGP palette indices to display colors
        let pal = decode_palette(bgp, palette);

        // --- Draw tile atlas (all 384 tiles) ---
        font::draw_string(&mut self.buf, WIN_W, 4, 2, "TILE ATLAS", HEADER_COLOR);
        let atlas_y = 14;
        for tile_idx in 0..384usize {
            let tile_data = decode_tile(vram, tile_idx * 16);
            let tx = (tile_idx % TILE_W) * 8;
            let ty = atlas_y + (tile_idx / TILE_W) * 8;
            draw_tile_pixels(&mut self.buf, WIN_W, tx + 4, ty, &tile_data, &pal);
        }

        // --- Draw tile map 0 ($9800) ---
        let map_y = atlas_y + ATLAS_PX_H + 12;
        font::draw_string(&mut self.buf, WIN_W, 4, map_y - 10, "MAP 0 ($9800)", HEADER_COLOR);
        draw_tilemap(&mut self.buf, WIN_W, 4, map_y, vram, 0x1800, bgp, palette);

        // --- Draw tile map 1 ($9C00) ---
        font::draw_string(&mut self.buf, WIN_W, 264, map_y - 10, "MAP 1 ($9C00)", HEADER_COLOR);
        draw_tilemap(&mut self.buf, WIN_W, 264, map_y, vram, 0x1C00, bgp, palette);

        self.window.update_with_buffer(&self.buf, WIN_W, WIN_H).ok();
    }

    pub fn is_open(&self) -> bool {
        self.window.is_open()
    }
}

fn decode_palette(bgp: u8, display_pal: &[u32; 4]) -> [u32; 4] {
    [
        display_pal[(bgp & 0x03) as usize],
        display_pal[((bgp >> 2) & 0x03) as usize],
        display_pal[((bgp >> 4) & 0x03) as usize],
        display_pal[((bgp >> 6) & 0x03) as usize],
    ]
}

/// Decode 16 bytes of tile data into 64 pixel color indices (0-3).
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

fn draw_tile_pixels(buf: &mut [u32], buf_w: usize, x: usize, y: usize, pixels: &[u8; 64], pal: &[u32; 4]) {
    for row in 0..8 {
        for col in 0..8 {
            let px = x + col;
            let py = y + row;
            if px < buf_w && py * buf_w + px < buf.len() {
                buf[py * buf_w + px] = pal[pixels[row * 8 + col] as usize];
            }
        }
    }
}

fn draw_tilemap(
    buf: &mut [u32], buf_w: usize, x: usize, y: usize,
    vram: &[u8], map_offset: usize, bgp: u8, palette: &[u32; 4],
) {
    let pal = decode_palette(bgp, palette);
    // LCDC bit 4 determines addressing mode; for debug we show both modes
    // We use unsigned addressing (like LCDC bit 4 = 1) for simplicity
    for ty in 0..32 {
        for tx in 0..32 {
            let tile_idx = vram[map_offset + ty * 32 + tx] as usize;
            let tile_data = decode_tile(vram, tile_idx * 16);
            // Draw at half scale (skip every other pixel)
            for row in 0..8 {
                for col in 0..8 {
                    let px = x + tx * 8 + col;
                    let py = y + ty * 8 + row;
                    if px < buf_w && py * buf_w + px < buf.len() {
                        buf[py * buf_w + px] = pal[tile_data[row * 8 + col] as usize];
                    }
                }
            }
        }
    }
}
