pub struct Channel3 {
    pub enabled: bool,
    pub dac_enabled: bool,

    // Raw register bytes
    pub nr30: u8,
    pub nr31: u8,
    pub nr32: u8,
    pub nr33: u8,
    pub nr34: u8,

    // Wave RAM (16 bytes = 32 4-bit samples)
    pub wave_ram: [u8; 16],

    // Length counter
    pub length_counter: u16,

    // Frequency timer
    frequency_timer: i32,
    position_counter: u8, // 0-31

    // Last sample byte read (for DMG wave RAM access quirk)
    sample_buffer: u8,

    // DMG wave RAM access timing: true only during the T-cycle when
    // the frequency timer expires and wave RAM is read internally
    wave_just_read: bool,

}

impl Channel3 {
    // --- Field accessors ---
    fn frequency(&self) -> u16 { self.nr33 as u16 | ((self.nr34 as u16 & 0x07) << 8) }
    fn length_enable(&self) -> bool { self.nr34 & 0x40 != 0 }
    fn volume_code(&self) -> u8 { (self.nr32 >> 5) & 0x03 }

    // --- Register writes ---

    pub fn write_nr30(&mut self, val: u8) {
        self.nr30 = val;
        self.dac_enabled = val & 0x80 != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    pub fn write_nr31(&mut self, val: u8) {
        self.nr31 = val;
        self.length_counter = 256 - val as u16;
    }

    pub fn write_nr32(&mut self, val: u8) {
        self.nr32 = val;
    }

    pub fn write_nr33(&mut self, val: u8) {
        self.nr33 = val;
    }

    pub fn write_nr34(&mut self, val: u8, frame_step: u8) {
        let triggering = val & 0x80 != 0;
        let old_length_enable = self.length_enable();
        let new_length_enable = val & 0x40 != 0;
        self.nr34 = val;

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
            // DMG wave RAM corruption: retriggering while channel is active
            // when the internal frequency timer aligns with an APU cycle boundary
            // where the sample countdown would be 0 (SameBoy equivalent).
            // In our T-cycle model, timer == 2 maps to SameBoy's countdown == 0.
            if self.enabled && self.frequency_timer == 2 {
                // Use next position's byte (position hasn't advanced yet at timer==2)
                let offset = (((self.position_counter as usize) + 1) >> 1) & 0xF;
                if offset < 4 {
                    self.wave_ram[0] = self.wave_ram[offset];
                } else {
                    let aligned = offset & !3;
                    let src = [
                        self.wave_ram[aligned],
                        self.wave_ram[aligned + 1],
                        self.wave_ram[aligned + 2],
                        self.wave_ram[aligned + 3],
                    ];
                    self.wave_ram[0] = src[0];
                    self.wave_ram[1] = src[1];
                    self.wave_ram[2] = src[2];
                    self.wave_ram[3] = src[3];
                }
            }
            self.trigger(frame_step);
        }
    }

    pub fn write_length(&mut self, val: u8) {
        self.length_counter = 256 - val as u16;
    }

    // --- Wave RAM access ---

    pub fn read_wave_ram(&self, offset: u8) -> u8 {
        if self.enabled {
            // DMG quirk: reads only succeed during the T-cycle when wave RAM
            // was just accessed internally; otherwise return 0xFF
            if self.wave_just_read {
                self.wave_ram[(self.position_counter / 2) as usize]
            } else {
                0xFF
            }
        } else {
            self.wave_ram[offset as usize]
        }
    }

    pub fn write_wave_ram(&mut self, offset: u8, val: u8) {
        if self.enabled {
            // DMG quirk: writes only succeed during the T-cycle when wave RAM
            // was just accessed internally; otherwise the write is lost
            if self.wave_just_read {
                self.wave_ram[(self.position_counter / 2) as usize] = val;
            }
        } else {
            self.wave_ram[offset as usize] = val;
        }
    }

    // --- Trigger ---

    fn trigger(&mut self, frame_step: u8) {
        self.enabled = true;

        if self.length_counter == 0 {
            self.length_counter = 256;
            if self.length_enable() && (frame_step & 1 != 0) {
                self.length_counter -= 1;
            }
        }

        self.frequency_timer = self.period() + 6;
        self.position_counter = 0;

        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    // --- Clocking ---

    pub fn tick(&mut self) {
        // wave_just_read is per-T-cycle (for wave RAM read/write access window)
        self.wave_just_read = false;

        self.frequency_timer -= 1;
        if self.frequency_timer <= 0 {
            self.frequency_timer = self.period();
            self.position_counter = (self.position_counter + 1) & 31;
            // Read the sample byte at the new position
            self.sample_buffer = self.wave_ram[(self.position_counter / 2) as usize];
            // DMG: wave RAM is accessible during this T-cycle
            self.wave_just_read = true;
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

    // --- Output ---

    pub fn output(&self) -> u8 {
        if !self.enabled || !self.dac_enabled {
            return 0;
        }

        // Get current 4-bit sample
        let sample = if self.position_counter & 1 == 0 {
            (self.sample_buffer >> 4) & 0x0F
        } else {
            self.sample_buffer & 0x0F
        };

        // Apply volume shift
        match self.volume_code() {
            0 => 0,          // Mute
            1 => sample,     // 100%
            2 => sample >> 1, // 50%
            3 => sample >> 2, // 25%
            _ => 0,
        }
    }

    fn period(&self) -> i32 {
        ((2048 - self.frequency() as i32) * 2).max(1)
    }

    pub fn power_off(&mut self) {
        self.nr30 = 0;
        self.nr31 = 0;
        self.nr32 = 0;
        self.nr33 = 0;
        self.nr34 = 0;
        self.enabled = false;
        self.dac_enabled = false;
        self.frequency_timer = 0;
        self.position_counter = 0;
        self.sample_buffer = 0;
        self.wave_just_read = false;
        // length_counter preserved on DMG
        // wave_ram preserved on power off
    }

    // --- Savestate ---

    pub fn save_state(&self, buf: &mut Vec<u8>) {
        use crate::savestate::*;
        write_bool(buf, self.enabled);
        write_bool(buf, self.dac_enabled);
        write_u8(buf, self.nr30);
        write_u8(buf, self.nr31);
        write_u8(buf, self.nr32);
        write_u8(buf, self.nr33);
        write_u8(buf, self.nr34);
        write_bytes(buf, &self.wave_ram);
        write_u16_le(buf, self.length_counter);
        write_u32_le(buf, self.frequency_timer as u32);
        write_u8(buf, self.position_counter);
        write_u8(buf, self.sample_buffer);
        write_bool(buf, self.wave_just_read);
    }

    pub fn load_state(&mut self, data: &[u8], cursor: &mut usize) {
        use crate::savestate::*;
        self.enabled = read_bool(data, cursor);
        self.dac_enabled = read_bool(data, cursor);
        self.nr30 = read_u8(data, cursor);
        self.nr31 = read_u8(data, cursor);
        self.nr32 = read_u8(data, cursor);
        self.nr33 = read_u8(data, cursor);
        self.nr34 = read_u8(data, cursor);
        let ram = read_bytes(data, cursor, 16);
        self.wave_ram.copy_from_slice(ram);
        self.length_counter = read_u16_le(data, cursor);
        self.frequency_timer = read_u32_le(data, cursor) as i32;
        self.position_counter = read_u8(data, cursor);
        self.sample_buffer = read_u8(data, cursor);
        self.wave_just_read = read_bool(data, cursor);
    }
}

impl Default for Channel3 {
    fn default() -> Self {
        Channel3 {
            enabled: false,
            dac_enabled: false,
            nr30: 0,
            nr31: 0,
            nr32: 0,
            nr33: 0,
            nr34: 0,
            wave_ram: [0; 16],
            length_counter: 0,
            frequency_timer: 0,
            position_counter: 0,
            sample_buffer: 0,
            wave_just_read: false,
        }
    }
}
