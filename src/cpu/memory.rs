use crate::cartridge::Cartridge;
use crate::timer::Timer;
use crate::ppu::Ppu;
use crate::joypad::Joypad;

pub struct MemoryBus {
    pub cartridge: Cartridge,
    pub vram: [u8; 0x2000],
    pub wram: [u8; 0x2000],
    pub oam: [u8; 0xA0],
    pub io: [u8; 0x80],
    pub hram: [u8; 0x7F],
    pub ie_register: u8,
    pub if_register: u8,
    pub timer: Timer,
    pub ppu: Ppu,
    pub joypad: Joypad,
}

impl MemoryBus {
    pub fn new(cartridge: Cartridge) -> Self {
        MemoryBus {
            cartridge,
            vram: [0; 0x2000],
            wram: [0; 0x2000],
            oam: [0; 0xA0],
            io: [0; 0x80],
            hram: [0; 0x7F],
            ie_register: 0,
            if_register: 0,
            timer: Timer::default(),
            ppu: Ppu::default(),
            joypad: Joypad::default(),
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF => self.cartridge.read_byte(address),
            0x8000..=0x9FFF => self.vram[(address - 0x8000) as usize],
            0xA000..=0xBFFF => 0xFF,
            0xC000..=0xDFFF => self.wram[(address - 0xC000) as usize],
            0xE000..=0xFDFF => self.wram[(address - 0xE000) as usize],
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize],
            0xFEA0..=0xFEFF => 0xFF,
            0xFF00..=0xFF7F => self.read_io(address),
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize],
            0xFFFF => self.ie_register,
        }
    }

    pub fn write_byte(&mut self, address: u16, byte: u8) {
        match address {
            0x0000..=0x7FFF => { /* ROM: read-only for MBC0 */ }
            0x8000..=0x9FFF => self.vram[(address - 0x8000) as usize] = byte,
            0xA000..=0xBFFF => { /* No external RAM for MBC0 */ }
            0xC000..=0xDFFF => self.wram[(address - 0xC000) as usize] = byte,
            0xE000..=0xFDFF => self.wram[(address - 0xE000) as usize] = byte,
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize] = byte,
            0xFEA0..=0xFEFF => { /* unusable */ }
            0xFF00..=0xFF7F => self.write_io(address, byte),
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize] = byte,
            0xFFFF => self.ie_register = byte,
        }
    }

    fn read_io(&self, address: u16) -> u8 {
        match address {
            0xFF00 => self.joypad.read(),
            0xFF04..=0xFF07 => self.timer.read(address),
            0xFF0F => self.if_register,
            0xFF40 => self.ppu.lcdc,
            0xFF41 => self.ppu.read_stat(),
            0xFF42 => self.ppu.scy,
            0xFF43 => self.ppu.scx,
            0xFF44 => self.ppu.ly,
            0xFF45 => self.ppu.lyc,
            0xFF46 => 0, // DMA - write only
            0xFF47 => self.ppu.bgp,
            0xFF48 => self.ppu.obp0,
            0xFF49 => self.ppu.obp1,
            0xFF4A => self.ppu.wy,
            0xFF4B => self.ppu.wx,
            _ => self.io[(address - 0xFF00) as usize],
        }
    }

    fn write_io(&mut self, address: u16, byte: u8) {
        match address {
            0xFF00 => self.joypad.write(byte),
            0xFF04..=0xFF07 => self.timer.write(address, byte),
            0xFF0F => self.if_register = byte,
            0xFF40 => self.ppu.lcdc = byte,
            0xFF41 => self.ppu.write_stat(byte),
            0xFF42 => self.ppu.scy = byte,
            0xFF43 => self.ppu.scx = byte,
            0xFF44 => { /* LY is read-only */ }
            0xFF45 => self.ppu.lyc = byte,
            0xFF46 => self.oam_dma(byte),
            0xFF47 => self.ppu.bgp = byte,
            0xFF48 => self.ppu.obp0 = byte,
            0xFF49 => self.ppu.obp1 = byte,
            0xFF4A => self.ppu.wy = byte,
            0xFF4B => self.ppu.wx = byte,
            _ => self.io[(address - 0xFF00) as usize] = byte,
        }
    }

    fn oam_dma(&mut self, byte: u8) {
        let base = (byte as u16) << 8;
        for i in 0..0xA0u16 {
            let val = self.read_byte(base + i);
            self.oam[i as usize] = val;
        }
    }
}

impl Default for MemoryBus {
    fn default() -> Self {
        MemoryBus::new(Cartridge::default())
    }
}
