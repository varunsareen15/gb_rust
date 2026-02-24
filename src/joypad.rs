pub struct Joypad {
    select: u8,
    pub buttons: u8,   // Start, Select, B, A (active low: 0 = pressed)
    pub dpad: u8,      // Down, Up, Left, Right (active low: 0 = pressed)
    pub interrupt: bool,
}

impl Joypad {
    pub fn read(&self) -> u8 {
        let mut result = self.select | 0xC0;
        if self.select & 0x20 == 0 {
            result = (result & 0xF0) | (self.buttons & 0x0F);
        }
        if self.select & 0x10 == 0 {
            result = (result & 0xF0) | (self.dpad & 0x0F);
        }
        result
    }

    pub fn write(&mut self, byte: u8) {
        self.select = byte & 0x30;
    }

    pub fn key_down(&mut self, key: JoypadKey) {
        match key {
            JoypadKey::Right  => self.dpad &= !0x01,
            JoypadKey::Left   => self.dpad &= !0x02,
            JoypadKey::Up     => self.dpad &= !0x04,
            JoypadKey::Down   => self.dpad &= !0x08,
            JoypadKey::A      => self.buttons &= !0x01,
            JoypadKey::B      => self.buttons &= !0x02,
            JoypadKey::Select => self.buttons &= !0x04,
            JoypadKey::Start  => self.buttons &= !0x08,
        }
        self.interrupt = true;
    }

    pub fn key_up(&mut self, key: JoypadKey) {
        match key {
            JoypadKey::Right  => self.dpad |= 0x01,
            JoypadKey::Left   => self.dpad |= 0x02,
            JoypadKey::Up     => self.dpad |= 0x04,
            JoypadKey::Down   => self.dpad |= 0x08,
            JoypadKey::A      => self.buttons |= 0x01,
            JoypadKey::B      => self.buttons |= 0x02,
            JoypadKey::Select => self.buttons |= 0x04,
            JoypadKey::Start  => self.buttons |= 0x08,
        }
    }
}

#[derive(Clone, Copy)]
pub enum JoypadKey {
    Right, Left, Up, Down,
    A, B, Select, Start,
}

impl Default for Joypad {
    fn default() -> Self {
        Joypad {
            select: 0x30,
            buttons: 0x0F,
            dpad: 0x0F,
            interrupt: false,
        }
    }
}
