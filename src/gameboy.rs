use std::collections::HashSet;
use crate::cpu::CPU;
use crate::cartridge::Cartridge;
use crate::savestate;

pub const CYCLES_PER_FRAME: u32 = 70224;

pub struct GameBoy {
    pub cpu: CPU,
}

impl GameBoy {
    pub fn new(cartridge: Cartridge) -> Self {
        let cpu = CPU::new(cartridge);
        GameBoy { cpu }
    }

    pub fn run_frame(&mut self) {
        let mut cycles_this_frame: u32 = 0;
        while cycles_this_frame < CYCLES_PER_FRAME {
            self.cpu.bus.cycles_ticked = 0;
            let cycles = self.cpu.step();

            // Tick timer for remaining cycles not already ticked during bus accesses
            let remaining = cycles.saturating_sub(self.cpu.bus.cycles_ticked);
            if remaining > 0 {
                self.cpu.bus.timer.tick(remaining, &mut self.cpu.bus.apu);
                if self.cpu.bus.timer.interrupt {
                    self.cpu.bus.if_register |= 0x04;
                    self.cpu.bus.timer.interrupt = false;
                }
            }

            // Tick PPU
            let vram_copy = self.cpu.bus.vram;
            let oam_copy = self.cpu.bus.oam;
            self.cpu.bus.ppu.tick(cycles, &vram_copy, &oam_copy);
            if self.cpu.bus.ppu.vblank_interrupt {
                self.cpu.bus.if_register |= 0x01; // VBlank interrupt
            }
            if self.cpu.bus.ppu.stat_interrupt {
                self.cpu.bus.if_register |= 0x02; // LCD STAT interrupt
            }

            // Joypad interrupt
            if self.cpu.bus.joypad.interrupt {
                self.cpu.bus.if_register |= 0x10; // Joypad interrupt
                self.cpu.bus.joypad.interrupt = false;
            }

            cycles_this_frame += cycles as u32;
        }
    }

    /// Execute a single CPU instruction + tick timer/PPU/joypad.
    pub fn run_step(&mut self) -> u8 {
        self.cpu.bus.cycles_ticked = 0;
        let cycles = self.cpu.step();

        let remaining = cycles.saturating_sub(self.cpu.bus.cycles_ticked);
        if remaining > 0 {
            self.cpu.bus.timer.tick(remaining, &mut self.cpu.bus.apu);
            if self.cpu.bus.timer.interrupt {
                self.cpu.bus.if_register |= 0x04;
                self.cpu.bus.timer.interrupt = false;
            }
        }

        let vram_copy = self.cpu.bus.vram;
        let oam_copy = self.cpu.bus.oam;
        self.cpu.bus.ppu.tick(cycles, &vram_copy, &oam_copy);
        if self.cpu.bus.ppu.vblank_interrupt {
            self.cpu.bus.if_register |= 0x01;
        }
        if self.cpu.bus.ppu.stat_interrupt {
            self.cpu.bus.if_register |= 0x02;
        }

        if self.cpu.bus.joypad.interrupt {
            self.cpu.bus.if_register |= 0x10;
            self.cpu.bus.joypad.interrupt = false;
        }

        cycles
    }

    /// Run a frame, checking PC against breakpoints after each step.
    /// Returns true if a breakpoint was hit (frame not fully completed).
    pub fn run_frame_with_breakpoints(&mut self, breakpoints: &HashSet<u16>) -> bool {
        let mut cycles_this_frame: u32 = 0;
        while cycles_this_frame < CYCLES_PER_FRAME {
            let cycles = self.run_step();
            cycles_this_frame += cycles as u32;

            if breakpoints.contains(&self.cpu.pc) {
                return true;
            }
        }
        false
    }

    pub fn framebuffer(&self) -> &[u8; 160 * 144] {
        &self.cpu.bus.ppu.framebuffer
    }

    pub fn save_state_to_slot(&self, slot: u8) -> Result<(), String> {
        let rom_path = self.cpu.bus.cartridge.rom_path()
            .ok_or_else(|| "No ROM path available".to_string())?;
        let path = savestate::save_state_path(rom_path, slot);
        savestate::save_to_file(self, &path)?;
        eprintln!("State saved to {}", path.display());
        Ok(())
    }

    pub fn load_state_from_slot(&mut self, slot: u8) -> Result<(), String> {
        let rom_path = self.cpu.bus.cartridge.rom_path()
            .ok_or_else(|| "No ROM path available".to_string())?
            .to_string();
        let path = savestate::save_state_path(&rom_path, slot);
        savestate::load_from_file(self, &path)?;
        eprintln!("State loaded from {}", path.display());
        Ok(())
    }
}
