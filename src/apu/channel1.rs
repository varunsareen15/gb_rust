const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
    [1, 0, 0, 0, 0, 0, 0, 1], // 25%
    [1, 0, 0, 0, 0, 1, 1, 1], // 50%
    [0, 1, 1, 1, 1, 1, 1, 0], // 75%
];

pub struct Channel1 {
    pub enabled: bool,
    pub dac_enabled: bool,

    // Raw register bytes (for reads)
    pub nr10: u8,
    pub nr11: u8,
    pub nr12: u8,
    pub nr13: u8,
    pub nr14: u8,

    // Length counter
    pub length_counter: u16,

    // Volume envelope state
    volume: u8,
    envelope_timer: u8,
    envelope_running: bool,

    // Frequency timer
    frequency_timer: i32,
    duty_position: u8,

    // Sweep state
    sweep_timer: u8,
    sweep_enabled: bool,
    sweep_shadow_frequency: u16,
    sweep_negate_used: bool,
}

impl Channel1 {
    // --- NR10 field accessors ---
    fn sweep_period(&self) -> u8 { (self.nr10 >> 4) & 0x07 }
    fn sweep_negate(&self) -> bool { self.nr10 & 0x08 != 0 }
    fn sweep_shift(&self) -> u8 { self.nr10 & 0x07 }

    // --- NR11 field accessors ---
    fn duty(&self) -> u8 { (self.nr11 >> 6) & 0x03 }

    // --- NR12 field accessors ---
    fn envelope_initial_volume(&self) -> u8 { (self.nr12 >> 4) & 0x0F }
    fn envelope_add_mode(&self) -> bool { self.nr12 & 0x08 != 0 }
    fn envelope_period(&self) -> u8 { self.nr12 & 0x07 }

    // --- NR13/NR14 field accessors ---
    fn frequency(&self) -> u16 { self.nr13 as u16 | ((self.nr14 as u16 & 0x07) << 8) }
    fn length_enable(&self) -> bool { self.nr14 & 0x40 != 0 }

    // --- Register writes ---

    pub fn write_nr10(&mut self, val: u8) {
        let old_negate = self.sweep_negate();
        self.nr10 = val;
        // Negate quirk: switching from negate to positive after negate was used disables channel
        if old_negate && !self.sweep_negate() && self.sweep_negate_used {
            self.enabled = false;
        }
    }

    pub fn write_nr11(&mut self, val: u8) {
        self.nr11 = val;
        self.length_counter = 64 - (val & 0x3F) as u16;
    }

    pub fn write_nr12(&mut self, val: u8) {
        self.nr12 = val;
        self.dac_enabled = val & 0xF8 != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    pub fn write_nr13(&mut self, val: u8) {
        self.nr13 = val;
    }

    pub fn write_nr14(&mut self, val: u8, frame_step: u8) {
        let triggering = val & 0x80 != 0;
        let old_length_enable = self.length_enable();
        let new_length_enable = val & 0x40 != 0;
        self.nr14 = val;

        // Extra length clocking when enabling length at an odd frame step
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

    /// Write length counter only (used during power-off on DMG)
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

        // Reload frequency timer
        self.frequency_timer = self.period();

        // Reload envelope
        self.volume = self.envelope_initial_volume();
        self.envelope_timer = self.envelope_period();
        self.envelope_running = true;

        // Reload sweep
        self.sweep_shadow_frequency = self.frequency();
        self.sweep_timer = if self.sweep_period() > 0 { self.sweep_period() } else { 8 };
        self.sweep_negate_used = false;
        self.sweep_enabled = self.sweep_period() > 0 || self.sweep_shift() > 0;

        // If shift > 0, do initial overflow check
        if self.sweep_shift() > 0 {
            self.calculate_sweep_frequency();
        }

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

    pub fn clock_sweep(&mut self) {
        if self.sweep_timer > 0 {
            self.sweep_timer -= 1;
        }
        if self.sweep_timer == 0 {
            self.sweep_timer = if self.sweep_period() > 0 { self.sweep_period() } else { 8 };

            if self.sweep_enabled && self.sweep_period() > 0 {
                let new_freq = self.calculate_sweep_frequency();
                if new_freq <= 2047 && self.sweep_shift() > 0 {
                    self.sweep_shadow_frequency = new_freq;
                    // Write back to frequency registers
                    self.nr13 = (new_freq & 0xFF) as u8;
                    self.nr14 = (self.nr14 & 0xF8) | ((new_freq >> 8) as u8 & 0x07);
                    // Second overflow check
                    self.calculate_sweep_frequency();
                }
            }
        }
    }

    fn calculate_sweep_frequency(&mut self) -> u16 {
        let delta = self.sweep_shadow_frequency >> self.sweep_shift();
        let new_freq = if self.sweep_negate() {
            self.sweep_negate_used = true;
            self.sweep_shadow_frequency.wrapping_sub(delta)
        } else {
            self.sweep_shadow_frequency + delta
        };

        if new_freq > 2047 {
            self.enabled = false;
        }

        new_freq
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

    // --- Power off: zero all registers ---
    pub fn power_off(&mut self) {
        self.nr10 = 0;
        self.nr11 = 0;
        self.nr12 = 0;
        self.nr13 = 0;
        self.nr14 = 0;
        self.enabled = false;
        self.dac_enabled = false;
        self.volume = 0;
        self.envelope_timer = 0;
        self.envelope_running = false;
        self.frequency_timer = 0;
        self.duty_position = 0;
        self.sweep_timer = 0;
        self.sweep_enabled = false;
        self.sweep_shadow_frequency = 0;
        self.sweep_negate_used = false;
        // length_counter is preserved on DMG
    }

    // --- Savestate ---

    pub fn save_state(&self, buf: &mut Vec<u8>) {
        use crate::savestate::*;
        write_bool(buf, self.enabled);
        write_bool(buf, self.dac_enabled);
        write_u8(buf, self.nr10);
        write_u8(buf, self.nr11);
        write_u8(buf, self.nr12);
        write_u8(buf, self.nr13);
        write_u8(buf, self.nr14);
        write_u16_le(buf, self.length_counter);
        write_u8(buf, self.volume);
        write_u8(buf, self.envelope_timer);
        write_bool(buf, self.envelope_running);
        write_u32_le(buf, self.frequency_timer as u32);
        write_u8(buf, self.duty_position);
        write_u8(buf, self.sweep_timer);
        write_bool(buf, self.sweep_enabled);
        write_u16_le(buf, self.sweep_shadow_frequency);
        write_bool(buf, self.sweep_negate_used);
    }

    pub fn load_state(&mut self, data: &[u8], cursor: &mut usize) {
        use crate::savestate::*;
        self.enabled = read_bool(data, cursor);
        self.dac_enabled = read_bool(data, cursor);
        self.nr10 = read_u8(data, cursor);
        self.nr11 = read_u8(data, cursor);
        self.nr12 = read_u8(data, cursor);
        self.nr13 = read_u8(data, cursor);
        self.nr14 = read_u8(data, cursor);
        self.length_counter = read_u16_le(data, cursor);
        self.volume = read_u8(data, cursor);
        self.envelope_timer = read_u8(data, cursor);
        self.envelope_running = read_bool(data, cursor);
        self.frequency_timer = read_u32_le(data, cursor) as i32;
        self.duty_position = read_u8(data, cursor);
        self.sweep_timer = read_u8(data, cursor);
        self.sweep_enabled = read_bool(data, cursor);
        self.sweep_shadow_frequency = read_u16_le(data, cursor);
        self.sweep_negate_used = read_bool(data, cursor);
    }
}

impl Default for Channel1 {
    fn default() -> Self {
        Channel1 {
            enabled: false,
            dac_enabled: false,
            nr10: 0,
            nr11: 0,
            nr12: 0,
            nr13: 0,
            nr14: 0,
            length_counter: 0,
            volume: 0,
            envelope_timer: 0,
            envelope_running: false,
            frequency_timer: 0,
            duty_position: 0,
            sweep_timer: 0,
            sweep_enabled: false,
            sweep_shadow_frequency: 0,
            sweep_negate_used: false,
        }
    }
}
