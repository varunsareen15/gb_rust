use std::fs;
use std::path::Path;

pub struct Cartridge {
    rom: Vec<u8>,
    pub title: String,
    pub cartridge_type: u8,
}

impl Cartridge {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Cartridge, String> {
        let data = fs::read(path).map_err(|e| format!("Failed to read ROM: {}", e))?;
        if data.len() < 0x150 {
            return Err("ROM too small to contain header".to_string());
        }

        let title_bytes = &data[0x0134..0x0144];
        let title = String::from_utf8_lossy(title_bytes)
            .trim_end_matches('\0')
            .to_string();

        let cartridge_type = data[0x0147];

        Ok(Cartridge {
            rom: data,
            title,
            cartridge_type,
        })
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        let addr = address as usize;
        if addr < self.rom.len() {
            self.rom[addr]
        } else {
            0xFF
        }
    }
}

impl Default for Cartridge {
    fn default() -> Self {
        Cartridge {
            rom: vec![0; 0x8000],
            title: String::new(),
            cartridge_type: 0,
        }
    }
}
