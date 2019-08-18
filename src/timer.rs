pub struct Timer {
    divider:        u16,
    timer_counter:  u8,
    timer_modulo:   u8,

    timer_enable:   bool,
    clock_select:   u8
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            divider:        0,
            timer_counter:  0,
            timer_modulo:   0,

            timer_enable:   false,
            clock_select:   0
        }
    }

    pub fn read(&self, loc: u16) -> u8 {
        match loc {
            0xFF03 => (self.divider & 0xFF) as u8,
            0xFF04 => (self.divider >> 8) as u8,
            0xFF05 => self.timer_counter,
            0xFF06 => self.timer_modulo,
            0xFF07 => {
                let enable = if self.timer_enable {4} else {0};
                enable | self.clock_select
            },
            _ => 0,
        }
    }

    pub fn write(&mut self, loc: u16, val: u8) {
        match loc {
            0xFF03 => self.divider = 0,
            0xFF04 => self.divider = 0,
            0xFF05 => self.timer_counter = val,
            0xFF06 => self.timer_modulo = val,
            0xFF07 => {
                self.timer_enable = (val & 4) != 0;
                self.clock_select = val & 0b11;
            },
            _ => {},
        }
    }

    // Call this every cycle. Returns true if an interrupt is triggered.
    pub fn update(&mut self) -> bool {
        self.divider = (self.divider as u32 + 4) as u16;

        if self.timer_enable {
            let inc = match self.clock_select {
                0 => (self.divider & 0x3FF) == 0,
                1 => (self.divider & 0xF) == 0,
                2 => (self.divider & 0x3F) == 0,
                3 => (self.divider & 0xFF) == 0,
                _ => false,
            };
            if inc {
                self.timer_counter = (self.timer_counter as u16 + 1) as u8;
                if self.timer_counter == 0 {
                    self.timer_counter = self.timer_modulo;
                    return true;
                }
            }
        }

        return false;
    }
}
