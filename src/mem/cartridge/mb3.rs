use time::PreciseTime;

pub struct MB3 {
    pub ram_select: bool,
    reg_select:     u8,

    second_reg:     u8,
    minute_reg:     u8,
    hour_reg:       u8,
    day_count:      u16,
    day_overflow:   u8,

    halt:           u8,
    time:           PreciseTime,
}

impl MB3 {
    pub fn new() -> MB3 {
        MB3 {
            ram_select:     true,
            reg_select:     8,
            second_reg:     0,
            minute_reg:     0,
            hour_reg:       0,
            day_count:      0,
            day_overflow:   0,
            halt:           0,
            time:           PreciseTime::now(),
        }
    }

    pub fn get_rtc_reg(&self) -> u8 {
        let (sec,min,hour,day,over) = self.calc_time();
        match self.reg_select {
            0x8 => sec,
            0x9 => min,
            0xA => hour,
            0xB => (day % 512) as u8,
            _ => (day >> 8) as u8 | (self.halt << 6) | (over << 7),
        }
    }

    pub fn set_rtc_reg(&mut self, data: u8) {
        self.calc_set_time();
        match self.reg_select {
            0x8 => self.second_reg = data,
            0x9 => self.minute_reg = data,
            0xA => self.hour_reg = data,
            0xB => self.day_count = (self.day_count & 0xFF00) | (data as u16),
            _ => {self.day_count = (self.day_count & 0x00FF) | (((data as u16) & 1) << 8);
                  self.halt = (data >> 6) & 1;
                  self.day_overflow = (data >> 7) & 1;}
        }
    }

    pub fn select_rtc(&mut self, reg: u8) {
        self.reg_select = reg;
    }

    pub fn latch_clock(&mut self) {
        if self.halt == 0 {
            self.calc_time();
            self.halt = 1;
        } else {
            self.time = PreciseTime::now();
            self.halt = 0;
        }
    }

    // calculate reg values from time
    fn calc_set_time(&mut self) {
        if self.halt != 0 {return;}

        let time_diff = self.time.to(PreciseTime::now());
        self.time = PreciseTime::now();
        let seconds = ((time_diff.num_seconds() % 60) as u16) + (self.second_reg as u16);
        let minutes = ((time_diff.num_minutes() % 60) as u16) + (self.minute_reg as u16) + (seconds / 60);
        let hours = ((time_diff.num_hours() % 24) as u16) + (self.hour_reg as u16) + (minutes / 60);
        let days = (time_diff.num_days() as u16) + (self.day_count as u16) + (hours / 24);

        self.second_reg = seconds as u8;
        self.minute_reg = minutes as u8;
        self.hour_reg = hours as u8;
        self.day_count = days % 512;
        self.day_overflow = if days > 511 {1} else {self.day_overflow};
    }

    fn calc_time(&self) -> (u8,u8,u8,u16,u8) {
        if self.halt == 0 {
            let time_diff = self.time.to(PreciseTime::now());
            let seconds = ((time_diff.num_seconds() % 60) as u16) + (self.second_reg as u16);
            let minutes = ((time_diff.num_minutes() % 60) as u16) + (self.minute_reg as u16) + (seconds / 60);
            let hours = ((time_diff.num_hours() % 24) as u16) + (self.hour_reg as u16) + (minutes / 60);
            let days = (time_diff.num_days() as u16) + (self.day_count as u16) + (hours / 24);
            let over = if days > 511 {1} else {self.day_overflow};
            (seconds as u8,minutes as u8,hours as u8,days,over)
        }
        else {
            (self.second_reg,self.minute_reg,self.hour_reg,self.day_count,self.day_overflow)
        }
    }
}
