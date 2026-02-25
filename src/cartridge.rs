use std::fs;
use std::path::Path;
use std::time::SystemTime;

enum Mbc {
    NoMbc,
    Mbc1 {
        rom_bank: u8,
        ram_bank: u8,
        ram_enabled: bool,
        banking_mode: bool,
    },
    Mbc3 {
        rom_bank: u8,
        ram_bank: u8,
        ram_enabled: bool,
        rtc: Rtc,
        rtc_latch: u8,
    },
    Mbc5 {
        rom_bank: u16,
        ram_bank: u8,
        ram_enabled: bool,
    },
}

struct Rtc {
    seconds: u8,
    minutes: u8,
    hours: u8,
    days_low: u8,
    days_high: u8, // bit 0 = day MSB, bit 6 = halt, bit 7 = day overflow
    latched: [u8; 5],
    base_timestamp: u64,
}

impl Rtc {
    fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Rtc {
            seconds: 0,
            minutes: 0,
            hours: 0,
            days_low: 0,
            days_high: 0,
            latched: [0; 5],
            base_timestamp: now,
        }
    }

    fn latch(&mut self) {
        if self.days_high & 0x40 != 0 {
            // Halted: use stored values directly
            self.latched = [
                self.seconds,
                self.minutes,
                self.hours,
                self.days_low,
                self.days_high,
            ];
            return;
        }

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let elapsed = now.saturating_sub(self.base_timestamp);

        let total_seconds = elapsed;
        let s = (total_seconds % 60) as u8;
        let m = ((total_seconds / 60) % 60) as u8;
        let h = ((total_seconds / 3600) % 24) as u8;
        let days = (total_seconds / 86400) as u32;

        let day_low = (days & 0xFF) as u8;
        let day_msb = if days > 0xFF { 1 } else { 0 };
        let day_overflow = if days > 0x1FF { 0x80 } else { 0 };
        let day_high = (self.days_high & 0x40) | day_overflow | day_msb;

        self.latched = [s, m, h, day_low, day_high];
    }

    fn read(&self, reg: u8) -> u8 {
        match reg {
            0x08 => self.latched[0],
            0x09 => self.latched[1],
            0x0A => self.latched[2],
            0x0B => self.latched[3],
            0x0C => self.latched[4],
            _ => 0xFF,
        }
    }

    fn write(&mut self, reg: u8, value: u8) {
        // When writing RTC registers, update stored values and reset base_timestamp
        match reg {
            0x08 => self.seconds = value & 0x3F,
            0x09 => self.minutes = value & 0x3F,
            0x0A => self.hours = value & 0x1F,
            0x0B => self.days_low = value,
            0x0C => self.days_high = value & 0xC1,
            _ => {}
        }
        // Rebase timestamp from current register values
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let days = ((self.days_high as u32 & 0x01) << 8) | self.days_low as u32;
        let total_seconds =
            days as u64 * 86400 + self.hours as u64 * 3600 + self.minutes as u64 * 60 + self.seconds as u64;
        self.base_timestamp = now.saturating_sub(total_seconds);
    }
}

pub struct Cartridge {
    rom: Vec<u8>,
    ram: Vec<u8>,
    pub title: String,
    pub cartridge_type: u8,
    mbc: Mbc,
}

fn ram_size_from_code(code: u8) -> usize {
    match code {
        0x00 => 0,
        0x01 => 2 * 1024,
        0x02 => 8 * 1024,
        0x03 => 32 * 1024,
        0x04 => 128 * 1024,
        0x05 => 64 * 1024,
        _ => 0,
    }
}

fn mbc_from_type(cartridge_type: u8) -> Mbc {
    match cartridge_type {
        0x00 => Mbc::NoMbc,
        0x01..=0x03 => Mbc::Mbc1 {
            rom_bank: 1,
            ram_bank: 0,
            ram_enabled: false,
            banking_mode: false,
        },
        0x0F..=0x13 => Mbc::Mbc3 {
            rom_bank: 1,
            ram_bank: 0,
            ram_enabled: false,
            rtc: Rtc::new(),
            rtc_latch: 0xFF,
        },
        0x19..=0x1E => Mbc::Mbc5 {
            rom_bank: 1,
            ram_bank: 0,
            ram_enabled: false,
        },
        _ => Mbc::NoMbc,
    }
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
        let ram_code = data[0x0149];
        let ram_size = ram_size_from_code(ram_code);
        let mbc = mbc_from_type(cartridge_type);

        Ok(Cartridge {
            rom: data,
            ram: vec![0; ram_size],
            title,
            cartridge_type,
            mbc,
        })
    }

    fn num_rom_banks(&self) -> usize {
        (self.rom.len() / 0x4000).max(2)
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        match &self.mbc {
            Mbc::NoMbc => self.read_no_mbc(address),
            Mbc::Mbc1 { rom_bank, ram_bank, ram_enabled, banking_mode } => {
                self.read_mbc1(address, *rom_bank, *ram_bank, *ram_enabled, *banking_mode)
            }
            Mbc::Mbc3 { rom_bank, ram_bank, ram_enabled, rtc, .. } => {
                self.read_mbc3(address, *rom_bank, *ram_bank, *ram_enabled, rtc)
            }
            Mbc::Mbc5 { rom_bank, ram_bank, ram_enabled } => {
                self.read_mbc5(address, *rom_bank, *ram_bank, *ram_enabled)
            }
        }
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        match &mut self.mbc {
            Mbc::NoMbc => {} // writes ignored
            Mbc::Mbc1 { ref mut rom_bank, ref mut ram_bank, ref mut ram_enabled, ref mut banking_mode } => {
                match address {
                    0x0000..=0x1FFF => *ram_enabled = (value & 0x0F) == 0x0A,
                    0x2000..=0x3FFF => {
                        let bank = value & 0x1F;
                        *rom_bank = if bank == 0 { 1 } else { bank };
                    }
                    0x4000..=0x5FFF => *ram_bank = value & 0x03,
                    0x6000..=0x7FFF => *banking_mode = (value & 0x01) != 0,
                    0xA000..=0xBFFF => {
                        if *ram_enabled && !self.ram.is_empty() {
                            let bank = if *banking_mode { *ram_bank as usize } else { 0 };
                            let offset = bank * 0x2000 + (address as usize - 0xA000);
                            if offset < self.ram.len() {
                                self.ram[offset] = value;
                            }
                        }
                    }
                    _ => {}
                }
            }
            Mbc::Mbc3 { ref mut rom_bank, ref mut ram_bank, ref mut ram_enabled, ref mut rtc, ref mut rtc_latch } => {
                match address {
                    0x0000..=0x1FFF => *ram_enabled = (value & 0x0F) == 0x0A,
                    0x2000..=0x3FFF => {
                        let bank = value & 0x7F;
                        *rom_bank = if bank == 0 { 1 } else { bank };
                    }
                    0x4000..=0x5FFF => *ram_bank = value,
                    0x6000..=0x7FFF => {
                        if *rtc_latch == 0x00 && value == 0x01 {
                            rtc.latch();
                        }
                        *rtc_latch = value;
                    }
                    0xA000..=0xBFFF => {
                        if *ram_enabled {
                            if *ram_bank <= 0x03 {
                                let offset = *ram_bank as usize * 0x2000 + (address as usize - 0xA000);
                                if offset < self.ram.len() {
                                    self.ram[offset] = value;
                                }
                            } else if *ram_bank >= 0x08 && *ram_bank <= 0x0C {
                                rtc.write(*ram_bank, value);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Mbc::Mbc5 { ref mut rom_bank, ref mut ram_bank, ref mut ram_enabled } => {
                match address {
                    0x0000..=0x1FFF => *ram_enabled = (value & 0x0F) == 0x0A,
                    0x2000..=0x2FFF => {
                        *rom_bank = (*rom_bank & 0x100) | value as u16;
                    }
                    0x3000..=0x3FFF => {
                        *rom_bank = (*rom_bank & 0xFF) | ((value as u16 & 0x01) << 8);
                    }
                    0x4000..=0x5FFF => *ram_bank = value & 0x0F,
                    0xA000..=0xBFFF => {
                        if *ram_enabled && !self.ram.is_empty() {
                            let offset = *ram_bank as usize * 0x2000 + (address as usize - 0xA000);
                            if offset < self.ram.len() {
                                self.ram[offset] = value;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // --- NoMbc ---

    fn read_no_mbc(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF => {
                let addr = address as usize;
                if addr < self.rom.len() { self.rom[addr] } else { 0xFF }
            }
            0xA000..=0xBFFF => 0xFF,
            _ => 0xFF,
        }
    }

    // --- MBC1 ---

    fn read_mbc1(&self, address: u16, rom_bank: u8, ram_bank: u8, ram_enabled: bool, banking_mode: bool) -> u8 {
        let num_banks = self.num_rom_banks();
        match address {
            0x0000..=0x3FFF => {
                let bank = if banking_mode {
                    ((ram_bank as usize) << 5) % num_banks
                } else {
                    0
                };
                let addr = bank * 0x4000 + address as usize;
                if addr < self.rom.len() { self.rom[addr] } else { 0xFF }
            }
            0x4000..=0x7FFF => {
                let mut bank = ((ram_bank as usize) << 5) | rom_bank as usize;
                // rom_bank lower 5 bits can't be 0
                if bank & 0x1F == 0 {
                    bank |= 1;
                }
                bank %= num_banks;
                let addr = bank * 0x4000 + (address as usize - 0x4000);
                if addr < self.rom.len() { self.rom[addr] } else { 0xFF }
            }
            0xA000..=0xBFFF => {
                if ram_enabled && !self.ram.is_empty() {
                    let bank = if banking_mode { ram_bank as usize } else { 0 };
                    let offset = bank * 0x2000 + (address as usize - 0xA000);
                    if offset < self.ram.len() { self.ram[offset] } else { 0xFF }
                } else {
                    0xFF
                }
            }
            _ => 0xFF,
        }
    }

    // --- MBC3 ---

    fn read_mbc3(&self, address: u16, rom_bank: u8, ram_bank: u8, ram_enabled: bool, rtc: &Rtc) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                let addr = address as usize;
                if addr < self.rom.len() { self.rom[addr] } else { 0xFF }
            }
            0x4000..=0x7FFF => {
                let bank = (rom_bank as usize) % self.num_rom_banks();
                let addr = bank * 0x4000 + (address as usize - 0x4000);
                if addr < self.rom.len() { self.rom[addr] } else { 0xFF }
            }
            0xA000..=0xBFFF => {
                if !ram_enabled {
                    return 0xFF;
                }
                if ram_bank <= 0x03 {
                    let offset = ram_bank as usize * 0x2000 + (address as usize - 0xA000);
                    if offset < self.ram.len() { self.ram[offset] } else { 0xFF }
                } else if ram_bank >= 0x08 && ram_bank <= 0x0C {
                    rtc.read(ram_bank)
                } else {
                    0xFF
                }
            }
            _ => 0xFF,
        }
    }

    // --- MBC5 ---

    fn read_mbc5(&self, address: u16, rom_bank: u16, ram_bank: u8, ram_enabled: bool) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                let addr = address as usize;
                if addr < self.rom.len() { self.rom[addr] } else { 0xFF }
            }
            0x4000..=0x7FFF => {
                let bank = (rom_bank as usize) % self.num_rom_banks();
                let addr = bank * 0x4000 + (address as usize - 0x4000);
                if addr < self.rom.len() { self.rom[addr] } else { 0xFF }
            }
            0xA000..=0xBFFF => {
                if ram_enabled && !self.ram.is_empty() {
                    let offset = ram_bank as usize * 0x2000 + (address as usize - 0xA000);
                    if offset < self.ram.len() { self.ram[offset] } else { 0xFF }
                } else {
                    0xFF
                }
            }
            _ => 0xFF,
        }
    }
}

impl Default for Cartridge {
    fn default() -> Self {
        Cartridge {
            rom: vec![0; 0x8000],
            ram: Vec::new(),
            title: String::new(),
            cartridge_type: 0,
            mbc: Mbc::NoMbc,
        }
    }
}
