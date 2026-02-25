pub mod channel1;
pub mod channel2;
pub mod channel3;
pub mod channel4;

use channel1::Channel1;
use channel2::Channel2;
use channel3::Channel3;
use channel4::Channel4;

// OR masks for APU registers: unused/write-only bits read as 1
// Indexed by (address - 0xFF10)
const OR_MASKS: [u8; 23] = [
    0x80, // 0xFF10 NR10
    0x3F, // 0xFF11 NR11
    0x00, // 0xFF12 NR12
    0xFF, // 0xFF13 NR13 (write-only)
    0xBF, // 0xFF14 NR14
    0xFF, // 0xFF15 unused
    0x3F, // 0xFF16 NR21
    0x00, // 0xFF17 NR22
    0xFF, // 0xFF18 NR23 (write-only)
    0xBF, // 0xFF19 NR24
    0x7F, // 0xFF1A NR30
    0xFF, // 0xFF1B NR31 (write-only)
    0x9F, // 0xFF1C NR32
    0xFF, // 0xFF1D NR33 (write-only)
    0xBF, // 0xFF1E NR34
    0xFF, // 0xFF1F unused
    0xFF, // 0xFF20 NR41 (write-only)
    0x00, // 0xFF21 NR42
    0x00, // 0xFF22 NR43
    0xBF, // 0xFF23 NR44
    0x00, // 0xFF24 NR50
    0x00, // 0xFF25 NR51
    0x70, // 0xFF26 NR52
];

pub struct Apu {
    pub channel1: Channel1,
    pub channel2: Channel2,
    pub channel3: Channel3,
    pub channel4: Channel4,

    // Master control registers
    pub nr50: u8, // Master volume / Vin
    pub nr51: u8, // Sound panning
    pub power: bool, // NR52 bit 7

    // Frame sequencer
    pub frame_step: u8, // 0-7

    // Sample generation
    pub sample_buffer: Vec<f32>,
    pub sample_rate: u32,
    sample_timer: u32,
}

impl Apu {
    pub fn read_register(&self, address: u16) -> u8 {
        match address {
            0xFF10..=0xFF26 => {
                let index = (address - 0xFF10) as usize;
                let or_mask = OR_MASKS[index];
                let raw = self.read_register_raw(address);
                raw | or_mask
            }
            0xFF27..=0xFF2F => 0xFF, // Unused
            0xFF30..=0xFF3F => self.channel3.read_wave_ram((address - 0xFF30) as u8),
            _ => 0xFF,
        }
    }

    fn read_register_raw(&self, address: u16) -> u8 {
        match address {
            // Channel 1
            0xFF10 => self.channel1.nr10,
            0xFF11 => self.channel1.nr11,
            0xFF12 => self.channel1.nr12,
            0xFF13 => self.channel1.nr13,
            0xFF14 => self.channel1.nr14,

            // Channel 2
            0xFF15 => 0xFF, // unused
            0xFF16 => self.channel2.nr21,
            0xFF17 => self.channel2.nr22,
            0xFF18 => self.channel2.nr23,
            0xFF19 => self.channel2.nr24,

            // Channel 3
            0xFF1A => self.channel3.nr30,
            0xFF1B => self.channel3.nr31,
            0xFF1C => self.channel3.nr32,
            0xFF1D => self.channel3.nr33,
            0xFF1E => self.channel3.nr34,

            // Channel 4
            0xFF1F => 0xFF, // unused
            0xFF20 => self.channel4.nr41,
            0xFF21 => self.channel4.nr42,
            0xFF22 => self.channel4.nr43,
            0xFF23 => self.channel4.nr44,

            // Control
            0xFF24 => self.nr50,
            0xFF25 => self.nr51,
            0xFF26 => {
                let mut val: u8 = if self.power { 0x80 } else { 0x00 };
                if self.channel1.enabled { val |= 0x01; }
                if self.channel2.enabled { val |= 0x02; }
                if self.channel3.enabled { val |= 0x04; }
                if self.channel4.enabled { val |= 0x08; }
                val
            }

            _ => 0xFF,
        }
    }

    pub fn write_register(&mut self, address: u16, val: u8) {
        // Wave RAM is always writable
        if (0xFF30..=0xFF3F).contains(&address) {
            self.channel3.write_wave_ram((address - 0xFF30) as u8, val);
            return;
        }

        // NR52 is always writable (power control)
        if address == 0xFF26 {
            let was_on = self.power;
            self.power = val & 0x80 != 0;

            if was_on && !self.power {
                self.power_off();
            } else if !was_on && self.power {
                self.frame_step = 0;
            }
            return;
        }

        // When power is off, only length counter writes are accepted (DMG)
        if !self.power {
            match address {
                0xFF11 => self.channel1.write_length(val),
                0xFF16 => self.channel2.write_length(val),
                0xFF1B => self.channel3.write_length(val),
                0xFF20 => self.channel4.write_length(val),
                _ => {} // All other writes blocked
            }
            return;
        }

        match address {
            // Channel 1
            0xFF10 => self.channel1.write_nr10(val),
            0xFF11 => self.channel1.write_nr11(val),
            0xFF12 => self.channel1.write_nr12(val),
            0xFF13 => self.channel1.write_nr13(val),
            0xFF14 => self.channel1.write_nr14(val, self.frame_step),

            // Channel 2
            0xFF15 => {} // unused
            0xFF16 => self.channel2.write_nr21(val),
            0xFF17 => self.channel2.write_nr22(val),
            0xFF18 => self.channel2.write_nr23(val),
            0xFF19 => self.channel2.write_nr24(val, self.frame_step),

            // Channel 3
            0xFF1A => self.channel3.write_nr30(val),
            0xFF1B => self.channel3.write_nr31(val),
            0xFF1C => self.channel3.write_nr32(val),
            0xFF1D => self.channel3.write_nr33(val),
            0xFF1E => self.channel3.write_nr34(val, self.frame_step),

            // Channel 4
            0xFF1F => {} // unused
            0xFF20 => self.channel4.write_nr41(val),
            0xFF21 => self.channel4.write_nr42(val),
            0xFF22 => self.channel4.write_nr43(val),
            0xFF23 => self.channel4.write_nr44(val, self.frame_step),

            // Control
            0xFF24 => self.nr50 = val,
            0xFF25 => self.nr51 = val,

            _ => {} // Unused addresses
        }
    }

    /// Called when DIV bit 12 has a falling edge
    pub fn clock_frame_sequencer(&mut self) {
        match self.frame_step {
            0 => {
                self.channel1.clock_length();
                self.channel2.clock_length();
                self.channel3.clock_length();
                self.channel4.clock_length();
            }
            1 => {}
            2 => {
                self.channel1.clock_length();
                self.channel2.clock_length();
                self.channel3.clock_length();
                self.channel4.clock_length();
                self.channel1.clock_sweep();
            }
            3 => {}
            4 => {
                self.channel1.clock_length();
                self.channel2.clock_length();
                self.channel3.clock_length();
                self.channel4.clock_length();
            }
            5 => {}
            6 => {
                self.channel1.clock_length();
                self.channel2.clock_length();
                self.channel3.clock_length();
                self.channel4.clock_length();
                self.channel1.clock_sweep();
            }
            7 => {
                self.channel1.clock_envelope();
                self.channel2.clock_envelope();
                self.channel4.clock_envelope();
            }
            _ => {}
        }
        self.frame_step = (self.frame_step + 1) & 7;
    }

    /// Advance channel frequency timers by one T-cycle
    pub fn tick_one_t_cycle(&mut self) {
        self.channel1.tick();
        self.channel2.tick();
        self.channel3.tick();
        self.channel4.tick();

        // Sample generation: accumulate and produce sample when threshold reached
        if self.sample_rate > 0 {
            self.sample_timer += self.sample_rate;
            if self.sample_timer >= 4_194_304 {
                self.sample_timer -= 4_194_304;
                self.generate_sample();
            }
        }
    }

    fn generate_sample(&mut self) {
        if !self.power {
            self.sample_buffer.push(0.0);
            self.sample_buffer.push(0.0);
            return;
        }

        let ch_outputs: [f32; 4] = [
            self.dac_output_ch1(),
            self.dac_output_ch2(),
            self.dac_output_ch3(),
            self.dac_output_ch4(),
        ];

        let mut left = 0.0f32;
        let mut right = 0.0f32;

        for i in 0..4 {
            if self.nr51 & (1 << (i + 4)) != 0 { left += ch_outputs[i]; }
            if self.nr51 & (1 << i) != 0 { right += ch_outputs[i]; }
        }

        let left_vol = ((self.nr50 >> 4) & 0x07) as f32 + 1.0;
        let right_vol = (self.nr50 & 0x07) as f32 + 1.0;

        // Normalize: 4 channels max, 8 volume levels
        left = left * left_vol / 32.0;
        right = right * right_vol / 32.0;

        self.sample_buffer.push(left);
        self.sample_buffer.push(right);
    }

    fn dac_output_ch1(&self) -> f32 {
        if !self.channel1.dac_enabled { return 0.0; }
        if !self.channel1.enabled { return 0.0; }
        (self.channel1.output() as f32 / 7.5) - 1.0
    }

    fn dac_output_ch2(&self) -> f32 {
        if !self.channel2.dac_enabled { return 0.0; }
        if !self.channel2.enabled { return 0.0; }
        (self.channel2.output() as f32 / 7.5) - 1.0
    }

    fn dac_output_ch3(&self) -> f32 {
        if !self.channel3.dac_enabled { return 0.0; }
        if !self.channel3.enabled { return 0.0; }
        (self.channel3.output() as f32 / 7.5) - 1.0
    }

    fn dac_output_ch4(&self) -> f32 {
        if !self.channel4.dac_enabled { return 0.0; }
        if !self.channel4.enabled { return 0.0; }
        (self.channel4.output() as f32 / 7.5) - 1.0
    }

    fn power_off(&mut self) {
        self.channel1.power_off();
        self.channel2.power_off();
        self.channel3.power_off();
        self.channel4.power_off();
        self.nr50 = 0;
        self.nr51 = 0;
        // wave_ram is preserved (handled by channel3.power_off not touching it)
    }

    pub fn set_sample_rate(&mut self, rate: u32) {
        self.sample_rate = rate;
    }

    // --- Savestate ---

    pub fn save_state(&self, buf: &mut Vec<u8>) {
        use crate::savestate::*;
        write_u8(buf, self.nr50);
        write_u8(buf, self.nr51);
        write_bool(buf, self.power);
        write_u8(buf, self.frame_step);
        write_u32_le(buf, self.sample_rate);
        write_u32_le(buf, self.sample_timer);
        self.channel1.save_state(buf);
        self.channel2.save_state(buf);
        self.channel3.save_state(buf);
        self.channel4.save_state(buf);
    }

    pub fn load_state(&mut self, data: &[u8], cursor: &mut usize) {
        use crate::savestate::*;
        self.nr50 = read_u8(data, cursor);
        self.nr51 = read_u8(data, cursor);
        self.power = read_bool(data, cursor);
        self.frame_step = read_u8(data, cursor);
        self.sample_rate = read_u32_le(data, cursor);
        self.sample_timer = read_u32_le(data, cursor);
        self.channel1.load_state(data, cursor);
        self.channel2.load_state(data, cursor);
        self.channel3.load_state(data, cursor);
        self.channel4.load_state(data, cursor);
        // Clear sample buffer on load
        self.sample_buffer.clear();
    }
}

impl Default for Apu {
    fn default() -> Self {
        Apu {
            channel1: Channel1::default(),
            channel2: Channel2::default(),
            channel3: Channel3::default(),
            channel4: Channel4::default(),
            nr50: 0,
            nr51: 0,
            power: false,
            frame_step: 0,
            sample_buffer: Vec::new(),
            sample_rate: 44100,
            sample_timer: 0,
        }
    }
}
