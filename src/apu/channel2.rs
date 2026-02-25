const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
    [1, 0, 0, 0, 0, 0, 0, 1], // 25%
    [1, 0, 0, 0, 0, 1, 1, 1], // 50%
    [0, 1, 1, 1, 1, 1, 1, 0], // 75%
];

pub struct Channel2 {
    pub enabled: bool,
    pub dac_enabled: bool,

    // Raw register bytes
    pub nr21: u8,
    pub nr22: u8,
    pub nr23: u8,
    pub nr24: u8,

    // Length counter
    pub length_counter: u16,

    // Volume envelope state
    volume: u8,
    envelope_timer: u8,
    envelope_running: bool,

    // Frequency timer
    frequency_timer: i32,
    duty_position: u8,
}

impl Channel2 {
    // --- Field accessors ---
    fn duty(&self) -> u8 { (self.nr21 >> 6) & 0x03 }
    fn envelope_initial_volume(&self) -> u8 { (self.nr22 >> 4) & 0x0F }
    fn envelope_add_mode(&self) -> bool { self.nr22 & 0x08 != 0 }
    fn envelope_period(&self) -> u8 { self.nr22 & 0x07 }
    fn frequency(&self) -> u16 { self.nr23 as u16 | ((self.nr24 as u16 & 0x07) << 8) }
    fn length_enable(&self) -> bool { self.nr24 & 0x40 != 0 }

    // --- Register writes ---

    pub fn write_nr21(&mut self, val: u8) {
        self.nr21 = val;
        self.length_counter = 64 - (val & 0x3F) as u16;
    }

    pub fn write_nr22(&mut self, val: u8) {
        self.nr22 = val;
        self.dac_enabled = val & 0xF8 != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    pub fn write_nr23(&mut self, val: u8) {
        self.nr23 = val;
    }

    pub fn write_nr24(&mut self, val: u8, frame_step: u8) {
        let triggering = val & 0x80 != 0;
        let old_length_enable = self.length_enable();
        let new_length_enable = val & 0x40 != 0;
        self.nr24 = val;

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
            self.duty_position = (self.duty_position + 1) & 7;
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
        DUTY_TABLE[self.duty() as usize][self.duty_position as usize] * self.volume
    }

    fn period(&self) -> i32 {
        ((2048 - self.frequency() as i32) * 4).max(1)
    }

    pub fn power_off(&mut self) {
        self.nr21 = 0;
        self.nr22 = 0;
        self.nr23 = 0;
        self.nr24 = 0;
        self.enabled = false;
        self.dac_enabled = false;
        self.volume = 0;
        self.envelope_timer = 0;
        self.envelope_running = false;
        self.frequency_timer = 0;
        self.duty_position = 0;
    }

    // --- Savestate ---

    pub fn save_state(&self, buf: &mut Vec<u8>) {
        use crate::savestate::*;
        write_bool(buf, self.enabled);
        write_bool(buf, self.dac_enabled);
        write_u8(buf, self.nr21);
        write_u8(buf, self.nr22);
        write_u8(buf, self.nr23);
        write_u8(buf, self.nr24);
        write_u16_le(buf, self.length_counter);
        write_u8(buf, self.volume);
        write_u8(buf, self.envelope_timer);
        write_bool(buf, self.envelope_running);
        write_u32_le(buf, self.frequency_timer as u32);
        write_u8(buf, self.duty_position);
    }

    pub fn load_state(&mut self, data: &[u8], cursor: &mut usize) {
        use crate::savestate::*;
        self.enabled = read_bool(data, cursor);
        self.dac_enabled = read_bool(data, cursor);
        self.nr21 = read_u8(data, cursor);
        self.nr22 = read_u8(data, cursor);
        self.nr23 = read_u8(data, cursor);
        self.nr24 = read_u8(data, cursor);
        self.length_counter = read_u16_le(data, cursor);
        self.volume = read_u8(data, cursor);
        self.envelope_timer = read_u8(data, cursor);
        self.envelope_running = read_bool(data, cursor);
        self.frequency_timer = read_u32_le(data, cursor) as i32;
        self.duty_position = read_u8(data, cursor);
    }
}

impl Default for Channel2 {
    fn default() -> Self {
        Channel2 {
            enabled: false,
            dac_enabled: false,
            nr21: 0,
            nr22: 0,
            nr23: 0,
            nr24: 0,
            length_counter: 0,
            volume: 0,
            envelope_timer: 0,
            envelope_running: false,
            frequency_timer: 0,
            duty_position: 0,
        }
    }
}
