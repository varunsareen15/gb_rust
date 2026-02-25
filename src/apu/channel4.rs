const DIVISOR_TABLE: [u32; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

pub struct Channel4 {
    pub enabled: bool,
    pub dac_enabled: bool,

    // Raw register bytes
    pub nr41: u8,
    pub nr42: u8,
    pub nr43: u8,
    pub nr44: u8,

    // Length counter
    pub length_counter: u16,

    // Volume envelope state
    volume: u8,
    envelope_timer: u8,
    envelope_running: bool,

    // LFSR
    lfsr: u16,
    frequency_timer: i32,
}

impl Channel4 {
    // --- Field accessors ---
    fn envelope_initial_volume(&self) -> u8 { (self.nr42 >> 4) & 0x0F }
    fn envelope_add_mode(&self) -> bool { self.nr42 & 0x08 != 0 }
    fn envelope_period(&self) -> u8 { self.nr42 & 0x07 }
    fn clock_shift(&self) -> u8 { (self.nr43 >> 4) & 0x0F }
    fn width_mode(&self) -> bool { self.nr43 & 0x08 != 0 } // true = 7-bit
    fn divisor_code(&self) -> u8 { self.nr43 & 0x07 }
    fn length_enable(&self) -> bool { self.nr44 & 0x40 != 0 }

    // --- Register writes ---

    pub fn write_nr41(&mut self, val: u8) {
        self.nr41 = val;
        self.length_counter = 64 - (val & 0x3F) as u16;
    }

    pub fn write_nr42(&mut self, val: u8) {
        self.nr42 = val;
        self.dac_enabled = val & 0xF8 != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    pub fn write_nr43(&mut self, val: u8) {
        self.nr43 = val;
    }

    pub fn write_nr44(&mut self, val: u8, frame_step: u8) {
        let triggering = val & 0x80 != 0;
        let old_length_enable = self.length_enable();
        let new_length_enable = val & 0x40 != 0;
        self.nr44 = val;

        // Extra length clocking
        if !old_length_enable && new_length_enable && (frame_step & 1 != 0) {
            if self.length_counter > 0 {
                self.length_counter -= 1;
                if self.length_counter == 0 && !triggering {
                    self.enabled = false;
                }
            }
        }

        if triggering {
            self.trigger(frame_step);
        }
    }

    pub fn write_length(&mut self, val: u8) {
        self.length_counter = 64 - (val & 0x3F) as u16;
    }

    // --- Trigger ---

    fn trigger(&mut self, frame_step: u8) {
        self.enabled = true;

        if self.length_counter == 0 {
            self.length_counter = 64;
            if self.length_enable() && (frame_step & 1 != 0) {
                self.length_counter -= 1;
            }
        }

        self.frequency_timer = self.period();
        self.lfsr = 0x7FFF;
        self.volume = self.envelope_initial_volume();
        self.envelope_timer = self.envelope_period();
        self.envelope_running = true;

        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    // --- Clocking ---

    pub fn tick(&mut self) {
        self.frequency_timer -= 1;
        if self.frequency_timer <= 0 {
            self.frequency_timer = self.period();

            // LFSR clock: XOR bits 0 and 1
            let xor_result = (self.lfsr & 0x01) ^ ((self.lfsr >> 1) & 0x01);
            self.lfsr >>= 1;
            self.lfsr |= xor_result << 14; // Set bit 14

            if self.width_mode() {
                // 7-bit mode: also set bit 6
                self.lfsr &= !(1 << 6);
                self.lfsr |= xor_result << 6;
            }
        }
    }

    pub fn clock_length(&mut self) {
        if self.length_enable() && self.length_counter > 0 {
            self.length_counter -= 1;
            if self.length_counter == 0 {
                self.enabled = false;
            }
        }
    }

    pub fn clock_envelope(&mut self) {
        if self.envelope_period() == 0 { return; }

        if self.envelope_timer > 0 {
            self.envelope_timer -= 1;
        }
        if self.envelope_timer == 0 {
            self.envelope_timer = self.envelope_period();
            if self.envelope_running {
                if self.envelope_add_mode() && self.volume < 15 {
                    self.volume += 1;
                } else if !self.envelope_add_mode() && self.volume > 0 {
                    self.volume -= 1;
                }
                if self.volume == 0 || self.volume == 15 {
                    self.envelope_running = false;
                }
            }
        }
    }

    // --- Output ---

    pub fn output(&self) -> u8 {
        if !self.enabled || !self.dac_enabled {
            return 0;
        }
        // Output is inverted bit 0 of LFSR
        let bit = (!self.lfsr) & 0x01;
        bit as u8 * self.volume
    }

    fn period(&self) -> i32 {
        let divisor = DIVISOR_TABLE[self.divisor_code() as usize];
        (divisor << self.clock_shift() as u32).max(1) as i32
    }

    pub fn power_off(&mut self) {
        self.nr41 = 0;
        self.nr42 = 0;
        self.nr43 = 0;
        self.nr44 = 0;
        self.enabled = false;
        self.dac_enabled = false;
        self.volume = 0;
        self.envelope_timer = 0;
        self.envelope_running = false;
        self.lfsr = 0;
        self.frequency_timer = 0;
        // length_counter preserved on DMG
    }

    // --- Savestate ---

    pub fn save_state(&self, buf: &mut Vec<u8>) {
        use crate::savestate::*;
        write_bool(buf, self.enabled);
        write_bool(buf, self.dac_enabled);
        write_u8(buf, self.nr41);
        write_u8(buf, self.nr42);
        write_u8(buf, self.nr43);
        write_u8(buf, self.nr44);
        write_u16_le(buf, self.length_counter);
        write_u8(buf, self.volume);
        write_u8(buf, self.envelope_timer);
        write_bool(buf, self.envelope_running);
        write_u16_le(buf, self.lfsr);
        write_u32_le(buf, self.frequency_timer as u32);
    }

    pub fn load_state(&mut self, data: &[u8], cursor: &mut usize) {
        use crate::savestate::*;
        self.enabled = read_bool(data, cursor);
        self.dac_enabled = read_bool(data, cursor);
        self.nr41 = read_u8(data, cursor);
        self.nr42 = read_u8(data, cursor);
        self.nr43 = read_u8(data, cursor);
        self.nr44 = read_u8(data, cursor);
        self.length_counter = read_u16_le(data, cursor);
        self.volume = read_u8(data, cursor);
        self.envelope_timer = read_u8(data, cursor);
        self.envelope_running = read_bool(data, cursor);
        self.lfsr = read_u16_le(data, cursor);
        self.frequency_timer = read_u32_le(data, cursor) as i32;
    }
}

impl Default for Channel4 {
    fn default() -> Self {
        Channel4 {
            enabled: false,
            dac_enabled: false,
            nr41: 0,
            nr42: 0,
            nr43: 0,
            nr44: 0,
            length_counter: 0,
            volume: 0,
            envelope_timer: 0,
            envelope_running: false,
            lfsr: 0x7FFF,
            frequency_timer: 0,
        }
    }
}
