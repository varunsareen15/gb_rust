pub struct Timer {
    pub div: u8,
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
    internal_counter: u16,
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

    pub fn write(&mut self, address: u16, byte: u8) {
        match address {
            0xFF04 => self.internal_counter = 0,
            0xFF05 => self.tima = byte,
            0xFF06 => self.tma = byte,
            0xFF07 => self.tac = byte,
            _ => {}
        }
    }

    pub fn tick(&mut self, t_cycles: u8) {
        self.interrupt = false;
        let m_cycles = t_cycles as u16;

        for _ in 0..m_cycles {
            let old_counter = self.internal_counter;
            self.internal_counter = self.internal_counter.wrapping_add(1);

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

impl Default for Timer {
    fn default() -> Self {
        Timer {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            internal_counter: 0,
            interrupt: false,
        }
    }
}
