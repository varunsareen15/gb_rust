use crate::apu::Apu;

pub struct Timer {
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
    pub internal_counter: u16,
    pub interrupt: bool,
}

impl Timer {
    pub fn read(&self, address: u16) -> u8 {
        match address {
            0xFF04 => (self.internal_counter >> 8) as u8,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac,
            _ => 0,
        }
    }

    pub fn write(&mut self, address: u16, byte: u8, apu: &mut Apu) {
        match address {
            0xFF04 => {
                // DIV reset: detect falling edge of bit 12 before clearing
                let old_bit12 = (self.internal_counter >> 12) & 1;
                self.internal_counter = 0;
                // If bit 12 was high, resetting causes a falling edge
                if old_bit12 == 1 {
                    apu.clock_frame_sequencer();
                }
            }
            0xFF05 => self.tima = byte,
            0xFF06 => self.tma = byte,
            0xFF07 => self.tac = byte,
            _ => {}
        }
    }

    pub fn tick(&mut self, t_cycles: u8, apu: &mut Apu) {
        self.interrupt = false;
        let cycles = t_cycles as u16;

        for _ in 0..cycles {
            let old_counter = self.internal_counter;
            self.internal_counter = self.internal_counter.wrapping_add(1);

            // Detect falling edge of bit 12 for APU frame sequencer (512 Hz)
            let old_bit12 = (old_counter >> 12) & 1;
            let new_bit12 = (self.internal_counter >> 12) & 1;
            if old_bit12 == 1 && new_bit12 == 0 {
                apu.clock_frame_sequencer();
            }

            // Tick APU one T-cycle (advance channel frequency timers + samples)
            apu.tick_one_t_cycle();

            // Timer (TIMA) falling edge detection
            if self.tac & 0x04 != 0 {
                let bit = match self.tac & 0x03 {
                    0 => 9,  // 4096 Hz
                    1 => 3,  // 262144 Hz
                    2 => 5,  // 65536 Hz
                    3 => 7,  // 16384 Hz
                    _ => unreachable!(),
                };

                // Falling edge detection
                let old_bit = (old_counter >> bit) & 1;
                let new_bit = (self.internal_counter >> bit) & 1;
                if old_bit == 1 && new_bit == 0 {
                    let (new_tima, overflow) = self.tima.overflowing_add(1);
                    if overflow {
                        self.tima = self.tma;
                        self.interrupt = true;
                    } else {
                        self.tima = new_tima;
                    }
                }
            }
        }
    }
}

impl Timer {
    pub fn save_state(&self, buf: &mut Vec<u8>) {
        use crate::savestate::*;
        write_u8(buf, self.tima);
        write_u8(buf, self.tma);
        write_u8(buf, self.tac);
        write_u16_le(buf, self.internal_counter);
        write_bool(buf, self.interrupt);
    }

    pub fn load_state(&mut self, data: &[u8], cursor: &mut usize) {
        use crate::savestate::*;
        self.tima = read_u8(data, cursor);
        self.tma = read_u8(data, cursor);
        self.tac = read_u8(data, cursor);
        self.internal_counter = read_u16_le(data, cursor);
        self.interrupt = read_bool(data, cursor);
    }
}

impl Default for Timer {
    fn default() -> Self {
        Timer {
            tima: 0,
            tma: 0,
            tac: 0,
            internal_counter: 0,
            interrupt: false,
        }
    }
}
