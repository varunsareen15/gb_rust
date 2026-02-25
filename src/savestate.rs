use std::fs;
use std::path::{Path, PathBuf};

use crate::gameboy::GameBoy;

const MAGIC: [u8; 4] = *b"GBSS";
const VERSION: u8 = 0x02;

// --- Write helpers ---

pub fn write_u8(buf: &mut Vec<u8>, val: u8) {
    buf.push(val);
}

pub fn write_u16_le(buf: &mut Vec<u8>, val: u16) {
    buf.extend_from_slice(&val.to_le_bytes());
}

pub fn write_u32_le(buf: &mut Vec<u8>, val: u32) {
    buf.extend_from_slice(&val.to_le_bytes());
}

pub fn write_bool(buf: &mut Vec<u8>, val: bool) {
    buf.push(if val { 1 } else { 0 });
}

pub fn write_bytes(buf: &mut Vec<u8>, data: &[u8]) {
    buf.extend_from_slice(data);
}

// --- Read helpers ---

pub fn read_u8(data: &[u8], cursor: &mut usize) -> u8 {
    let val = data[*cursor];
    *cursor += 1;
    val
}

pub fn read_u16_le(data: &[u8], cursor: &mut usize) -> u16 {
    let val = u16::from_le_bytes([data[*cursor], data[*cursor + 1]]);
    *cursor += 2;
    val
}

pub fn read_u32_le(data: &[u8], cursor: &mut usize) -> u32 {
    let val = u32::from_le_bytes([
        data[*cursor],
        data[*cursor + 1],
        data[*cursor + 2],
        data[*cursor + 3],
    ]);
    *cursor += 4;
    val
}

pub fn read_bool(data: &[u8], cursor: &mut usize) -> bool {
    let val = data[*cursor] != 0;
    *cursor += 1;
    val
}

pub fn read_bytes<'a>(data: &'a [u8], cursor: &mut usize, len: usize) -> &'a [u8] {
    let slice = &data[*cursor..*cursor + len];
    *cursor += len;
    slice
}

// --- Path helper ---

pub fn save_state_path(rom_path: &str, slot: u8) -> PathBuf {
    let path = Path::new(rom_path);
    let parent = path.parent().unwrap_or(Path::new("."));
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    parent
        .join("saves")
        .join(stem.as_ref())
        .join(format!("{}.ss{}", stem, slot))
}

// --- Top-level save/load ---

pub fn save(gb: &GameBoy) -> Vec<u8> {
    let mut buf = Vec::new();

    // Header
    write_bytes(&mut buf, &MAGIC);
    write_u8(&mut buf, VERSION);
    write_u8(&mut buf, gb.cpu.bus.cartridge.mbc_type_tag());
    write_u32_le(&mut buf, gb.cpu.bus.cartridge.ram_len() as u32);

    // Body
    gb.cpu.save_state(&mut buf);

    buf
}

pub fn load(gb: &mut GameBoy, data: &[u8]) -> Result<(), String> {
    if data.len() < 10 {
        return Err("Save state too small".to_string());
    }

    let mut cursor = 0;

    // Validate header
    let magic = read_bytes(data, &mut cursor, 4);
    if magic != MAGIC {
        return Err("Invalid save state magic".to_string());
    }

    let version = read_u8(data, &mut cursor);
    if version != VERSION {
        return Err(format!("Unsupported save state version: {}", version));
    }

    let mbc_tag = read_u8(data, &mut cursor);
    if mbc_tag != gb.cpu.bus.cartridge.mbc_type_tag() {
        return Err("MBC type mismatch".to_string());
    }

    let ram_len = read_u32_le(data, &mut cursor) as usize;
    if ram_len != gb.cpu.bus.cartridge.ram_len() {
        return Err("Cartridge RAM size mismatch".to_string());
    }

    // Body
    gb.cpu.load_state(data, &mut cursor);

    Ok(())
}

// --- File I/O wrappers ---

pub fn save_to_file(gb: &GameBoy, path: &Path) -> Result<(), String> {
    let data = save(gb);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create save state directory: {}", e))?;
    }
    fs::write(path, &data).map_err(|e| format!("Failed to write save state: {}", e))?;
    Ok(())
}

pub fn load_from_file(gb: &mut GameBoy, path: &Path) -> Result<(), String> {
    let data = fs::read(path).map_err(|e| format!("Failed to read save state: {}", e))?;
    load(gb, &data)
}
