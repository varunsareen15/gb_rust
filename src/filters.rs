pub const PALETTE_CLASSIC: [u32; 4] = [0x00E0F8D0, 0x0088C070, 0x00346856, 0x00081820];
pub const PALETTE_DMG_GREEN: [u32; 4] = [0x009BBC0F, 0x008BAC0F, 0x00306230, 0x000F380F];
pub const PALETTE_GRAYSCALE: [u32; 4] = [0x00FFFFFF, 0x00AAAAAA, 0x00555555, 0x00000000];
pub const PALETTE_POCKET: [u32; 4] = [0x00C4CFA1, 0x008B956D, 0x004D533C, 0x001F1F1F];

pub const PALETTES: [(&str, [u32; 4]); 4] = [
    ("Classic", PALETTE_CLASSIC),
    ("DMG Green", PALETTE_DMG_GREEN),
    ("Grayscale", PALETTE_GRAYSCALE),
    ("Pocket", PALETTE_POCKET),
];

pub fn upscale_nearest(src: &[u32], dst: &mut [u32], src_w: usize, src_h: usize) {
    let dst_w = src_w * 2;
    for y in 0..src_h {
        for x in 0..src_w {
            let color = src[y * src_w + x];
            let dx = x * 2;
            let dy = y * 2;
            dst[dy * dst_w + dx] = color;
            dst[dy * dst_w + dx + 1] = color;
            dst[(dy + 1) * dst_w + dx] = color;
            dst[(dy + 1) * dst_w + dx + 1] = color;
        }
    }
}

pub fn apply_scanlines(buf: &mut [u32], width: usize, height: usize) {
    for y in (1..height).step_by(2) {
        let row_start = y * width;
        for x in 0..width {
            let c = buf[row_start + x];
            let r = ((c >> 16) & 0xFF) * 60 / 100;
            let g = ((c >> 8) & 0xFF) * 60 / 100;
            let b = (c & 0xFF) * 60 / 100;
            buf[row_start + x] = (r << 16) | (g << 8) | b;
        }
    }
}
