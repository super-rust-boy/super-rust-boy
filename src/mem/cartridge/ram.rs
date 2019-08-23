// RAM
use chrono::{
    DateTime,
    Duration,
    Utc
};

use std::{
    io::{
        BufReader,
        BufWriter,
        Read,
        Write
    },
    fs::{
        File,
        OpenOptions
    }
};

use crate::mem::MemDevice;

pub trait RAM: MemDevice {
    fn set_bank(&mut self, bank: u8, loc: u16);
    fn flush(&mut self) {}
}

// Banked RAM
pub struct BankedRAM {
    ram:    Vec<u8>,
    offset: usize
}

impl BankedRAM {
    pub fn new(ram_size: usize) -> Self {
        BankedRAM {
            ram: vec![0; ram_size],
            offset: 0
        }
    }
}

impl MemDevice for BankedRAM {
    fn read(&self, loc: u16) -> u8 {
        self.ram[self.offset + (loc as usize)]
    }

    fn write(&mut self, loc: u16, val: u8) {
        self.ram[self.offset + (loc as usize)] = val;
    }
}

impl RAM for BankedRAM {
    fn set_bank(&mut self, bank: u8, _: u16) {
        self.offset = (bank as usize) * 0x2000;
    }
}

// Battery backed RAM
pub struct BatteryRAM {
    save_file:  String,
    offset:     usize,
    ram:        Vec<u8>,
    dirty:      bool,
}

impl BatteryRAM {
    pub fn new(ram_size: usize, save_file_name: &str) -> Result<Self, String> {
        let mut ram = vec![0; ram_size];

        if let Ok(file) = File::open(save_file_name) {
            let mut save_reader = BufReader::new(file);
            save_reader.read_exact(&mut ram).map_err(|e| e.to_string())?;
        } else {
            let file = File::create(save_file_name).map_err(|e| e.to_string())?;
            file.set_len(ram_size as u64).map_err(|e| e.to_string())?;
        }

        Ok(BatteryRAM {
            save_file:  save_file_name.to_string(),
            offset:     0,
            ram:        ram,
            dirty:      false
        })
    }
}

impl MemDevice for BatteryRAM {
    fn read(&self, loc: u16) -> u8 {
        self.ram[self.offset + (loc as usize)]
    }

    fn write(&mut self, loc: u16, val: u8) {
        let pos = self.offset + (loc as usize);

        self.ram[pos] = val;

        self.dirty = true;
    }
}

impl RAM for BatteryRAM {
    fn set_bank(&mut self, bank: u8, _: u16) {
        self.offset = (bank as usize) * 0x2000;
    }

    fn flush(&mut self) {
        if self.dirty {
            let save_f = OpenOptions::new()
                .write(true)
                .open(self.save_file.as_str())
                .expect("Couldn't open file");

            let mut bufwriter = BufWriter::new(save_f);

            bufwriter.write_all(&self.ram).expect("Couldn't write to file");

            self.dirty = false;
        }
    }
}

// Battery backed RAM with real-time clock

// What maps to the area of cart RAM.
enum RamMap {
    RAM,    // RAM
    S,      // Seconds
    M,      // Minutes
    H,      // Hours
    DL,     // Low 8 bits of day
    DH      // High bit of day, carry bit, halt flag
}

pub struct ClockRAM {
    save_file:  String,
    offset:     usize,
    ram:        Vec<u8>,
    dirty:      bool,
    ram_map:    RamMap,

    seconds:    u8,
    minutes:    u8,
    hours:      u8,
    days:       u16,
    time:       DateTime<Utc>,
    latch:      bool,
}

impl ClockRAM {
    pub fn new(ram_size: usize, save_file_name: &str) -> Result<Self, String> {
        let mut ram = vec![0; ram_size];
        let now = Utc::now();
        let timer_size = 5 + now.to_rfc3339().len();
        let mut timer = vec![0; timer_size];

        let mut seconds = 0;
        let mut minutes = 0;
        let mut hours = 0;
        let mut days = 0;

        if let Ok(file) = File::open(save_file_name) {
            let mut save_reader = BufReader::new(file);
            save_reader.read_exact(&mut ram).map_err(|e| e.to_string())?;

            // Calc difference in time since last time this was saved.
            save_reader.read_exact(&mut timer).map_err(|e| e.to_string())?;

            seconds = timer[0];
            minutes = timer[1];
            hours = timer[2];
            days = timer[3] as u16 | ((timer[4] as u16) << 8);

            let time_string = String::from_utf8(timer[5..].to_vec()).expect(&format!("Couldn't read time: {:?}", &timer[5..]));
            let old_time = chrono::DateTime::parse_from_rfc3339(&time_string).expect(&format!("Couldn't parse time: {}", time_string));
            let diff = now.signed_duration_since(old_time);

            update_times(&diff, &mut seconds, &mut minutes, &mut hours, &mut days);
        } else {
            let file = File::create(save_file_name).map_err(|e| e.to_string())?;
            file.set_len((ram_size + timer_size) as u64).map_err(|e| e.to_string())?;
        }

        Ok(ClockRAM {
            save_file:  save_file_name.to_string(),
            offset:     0,
            ram:        ram,
            dirty:      false,
            ram_map:    RamMap::RAM,

            seconds:    seconds,
            minutes:    minutes,
            hours:      hours,
            days:       days,
            time:       now,
            latch:      false
        })
    }
}

impl MemDevice for ClockRAM {
    fn read(&self, loc: u16) -> u8 {
        use RamMap::*;
        match self.ram_map {
            RAM => self.ram[self.offset + (loc as usize)],
            _ => {
                let mut seconds = self.seconds;
                let mut minutes = self.minutes;
                let mut hours = self.hours;
                let mut days = self.days;

                if !self.latch{
                    let now = Utc::now();
                    update_times(&now.signed_duration_since(self.time), &mut seconds, &mut minutes, &mut hours, &mut days);
                } else {
                    days |= 0x4000;
                }

                match self.ram_map {
                    S => seconds,
                    M => minutes,
                    H => hours,
                    DL => days as u8,
                    DH => (days >> 8) as u8,
                    _ => unreachable!()
                }
            },
        }
    }

    fn write(&mut self, loc: u16, val: u8) {
        use RamMap::*;

        match self.ram_map {
            RAM => {
                let pos = self.offset + (loc as usize);

                self.ram[pos] = val;
            },
            S => self.seconds = val,
            M => self.minutes = val,
            H => self.hours = val,
            DL => {
                self.days &= 0xFF00;
                self.days |= val as u16;
            },
            DH => {
                self.days &= 0xFF;
                self.days |= (val as u16) << 8;
            },
        }

        self.dirty = true;
    }
}

impl RAM for ClockRAM {
    fn set_bank(&mut self, bank: u8, loc: u16) {
        use RamMap::*;

        if loc < 0x6000 {
            self.ram_map = match bank & 0xF {
                x @ 0..=3 => { self.offset = (x as usize) * 0x2000; RAM },
                0x8 => S,
                0x9 => M,
                0xA => H,
                0xB => DL,
                _   => DH
            };
        } else {
            if (bank == 1) && !self.latch {
                self.latch = true;

                let now = Utc::now();
                update_times(&now.signed_duration_since(self.time), &mut self.seconds, &mut self.minutes, &mut self.hours, &mut self.days);

                self.time = now;
            } else if (bank == 1) && self.latch {
                self.latch = false;
            }
        }
    }

    fn flush(&mut self) {
        if self.dirty {
            let save_f = OpenOptions::new()
                .write(true)
                .open(self.save_file.as_str())
                .expect("Couldn't open file");

            let mut bufwriter = BufWriter::new(save_f);

            let old_time = self.time;
            self.time = Utc::now();
            update_times(&self.time.signed_duration_since(old_time), &mut self.seconds, &mut self.minutes, &mut self.hours, &mut self.days);

            let time = [
                self.seconds, self.minutes, self.hours,
                self.days as u8,
                (self.days >> 8) as u8
            ];

            bufwriter.write_all(&self.ram).expect("Couldn't write to file");
            bufwriter.write(&time).expect("Couldn't write time to file");
            bufwriter.write(&self.time.to_rfc3339().as_bytes()).expect("Couldn't write utc to file");

            self.dirty = false;
        }
    }
}

// Read in a duration and update time registers.
fn update_times(time_diff: &Duration, seconds: &mut u8, minutes: &mut u8, hours: &mut u8, days: &mut u16) {
    let new_seconds = (*seconds as i64) + time_diff.num_seconds();
    let new_minutes = (*minutes as i64) + (new_seconds / 60);
    let new_hours = (*hours as i64) + (new_minutes / 60);
    let new_days = ((*days & 0x1FF) as i64) + (new_hours / 24);

    *seconds = (new_seconds % 60) as u8;
    *minutes = (new_minutes % 60) as u8;
    *hours = (new_hours % 24) as u8;
    *days = (new_days % 512) as u16;
    if new_days > 511 {
        *days |= 0x8000;
    }
}