//

use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::fs::File;

use time::{Duration, PreciseTime};

use super::MemDevice;

#[derive(Clone)]
enum MBC {
    _0,
    _1(MB1),
    _2,
    _3(MB3),
    _4(u8),
    _5(u8),
}

pub struct Cartridge {
    rom_bank_0: [u8; 0x4000],
    rom_bank_n: [u8; 0x4000],
    ram: Vec<u8>,

    rom_file: BufReader<File>,
    mem_bank: MBC,
    ram_enable: bool,
    ram_offset: usize,
    battery: bool,
}

impl Cartridge {
    pub fn new(rom_file: &str) -> Result<Cartridge, String> {
        let f = try!(File::open(rom_file).map_err(|e| e.to_string()));

        let mut reader = BufReader::new(f);
        let mut buf: [u8; 0x4000] = [0; 0x4000];
        //try!(reader.read_exact(&mut buf).map_err(|e| e.to_string()));
        try!(reader.read(&mut buf).map_err(|e| e.to_string()));

        let bank_type = match buf[0x147] {
            0x1...0x3 => MBC::_1(MB1::new()),
            0x5...0x6 => MBC::_2,
            0xF...0x13 => MBC::_3(MB3::new()),
            0x15...0x17 => MBC::_4(0),
            0x19...0x1E => MBC::_5(0),
            _ => MBC::_0,
        };

        let ram_size = match (bank_type.clone(), buf[0x149]) {
            (MBC::_2,_) => 0x200,
            (_,0x1) => 0x800,
            (_,0x2) => 0x2000,
            (_,0x3) => 0x8000,
            _ => 0,
        };

        let ret = Cartridge {
            rom_bank_0: buf,
            rom_bank_n: [0; 0x4000],
            ram: vec!(0; ram_size),
            rom_file: reader,
            mem_bank: bank_type,
            ram_enable: false,
            ram_offset: 0,
            battery: false,
        };
        Ok(ret)
    }

    pub fn swap_rom_bank(&mut self, bank: u8)/* -> Result<(), String>*/ {
        //println!("Swapping in bank: {}", bank);
        let pos = (bank as u64) * 0x4000;
        match self.rom_file.seek(SeekFrom::Start(pos)) {
            Ok(_) => {},
            Err(s) => panic!("Couldn't swap in bank: {}", s),
        }
        //try!(self.rom_file.read_exact(&mut self.rom_bank_n).map_err(|e| e.to_string()));
        match self.rom_file.read(&mut self.rom_bank_n) {
            Ok(_) => {},
            Err(s) => panic!("Couldn't swap in bank: {}", s),
        }
    }

    #[inline]
    pub fn swap_ram_bank(&mut self, bank: u8) {
        self.ram_offset = (bank as usize) * 0x2000;
    }

    #[inline]
    pub fn read_ram(&self, loc: u16) -> u8 {
        if self.ram_enable {
            match self.mem_bank {
                MBC::_3(ref mb) => if mb.ram_select {self.ram[self.ram_offset + (loc as usize)]}
                                       else {mb.get_rtc_reg()},
                _ => return self.ram[self.ram_offset + (loc as usize)],
            }
        }
        else {
            return 0;
        }
    }

    #[inline]
    pub fn write_ram(&mut self, loc: u16, val: u8) {
        if self.ram_enable {
            match self.mem_bank.clone() {
                MBC::_2 => self.ram[self.ram_offset + (loc as usize)] = val & 0xF,
                MBC::_3(ref mut mb) => if mb.ram_select {self.ram[self.ram_offset + (loc as usize)] = val}
                                       else {mb.set_rtc_reg(val)},
                _ => self.ram[self.ram_offset + (loc as usize)] = val,
            }
        }
    }

    // Cartridge types when writing...
    #[inline]
    fn write_mb1(&mut self, mb: MB1, loc: u16, val: u8) {
        let mut new_mb = mb;
        match loc {
            0x0000...0x1FFF => self.ram_enable = if (val & 0xF) == 0xA {true} else {false},
            0x2000...0x3FFF => new_mb.set_lower(val),
            0x4000...0x5FFF => new_mb.set_upper(val),
            _ => new_mb.mem_type_select(val),
        }

        self.swap_rom_bank(new_mb.get_rom_bank());
        self.swap_ram_bank(new_mb.get_ram_bank());

        self.mem_bank = MBC::_1(new_mb);
    }

    #[inline]
    fn write_mb2(&mut self, loc: u16, val: u8) {
        match loc {
            0x0000...0x1FFF => self.ram_enable = if (val & 1) == 1 {true} else {false},
            0x2000...0x3FFF => self.swap_rom_bank(val & 0xF),
            _ => return,
        }
    }

    #[inline]
    fn write_mb3(&mut self, mb: MB3, loc: u16, val: u8) {
        let mut new_mb = mb;
        match (loc, val) {
            (0x0000...0x1FFF, x) => self.ram_enable = if (x & 0xF) == 0xA {true} else {false},
            (0x2000...0x3FFF, 0) => self.swap_rom_bank(1),
            (0x2000...0x3FFF, x) => self.swap_rom_bank(x),
            (0x4000...0x5FFF, x @ 0...3) => self.swap_ram_bank(x),
            (0x4000...0x5FFF, x @ 8...0xC) => new_mb.select_rtc(x),
            (0x6000...0x7FFF, 1) => new_mb.latch_clock(),
            _ => return,
        }

        self.mem_bank = MBC::_3(new_mb);
    }
}

impl MemDevice for Cartridge {
    fn read(&self, loc: u16) -> u8 {
        match loc {
            0x0...0x3FFF => self.rom_bank_0[loc as usize],
            0x4000...0x7FFF => self.rom_bank_n[(loc - 0x4000) as usize],
            _ => self.read_ram(loc - 0xA000),
        }
    }

    fn write(&mut self, loc: u16, val: u8) {
        if (loc >= 0xA000) && (loc < 0xC000) {
            self.write_ram(loc - 0xA000, val);
        }
        else {
            match self.mem_bank.clone() {
                MBC::_1(mb1) => self.write_mb1(mb1, loc, val),
                MBC::_2 => self.write_mb2(loc, val),
                MBC::_3(mb3) => self.write_mb3(mb3, loc, val),
                _ => return,
            }
        }
    }
}


// Types for bank swapping

#[derive(Clone)]
struct MB1 {
    upper_select: u8,
    lower_select: u8,
    mem_type: bool,
}

impl MB1 {
    fn new() -> MB1 {
        MB1 {upper_select: 0, lower_select: 0, mem_type: false}
    }

    fn clone(&self) -> MB1 {
        MB1 {
            upper_select: self.upper_select,
            lower_select: self.lower_select,
            mem_type: self.mem_type,
        }
    }

    fn set_lower(&mut self, val: u8) {
        match val & 0x1F {
            0 => self.lower_select = 1,
            x => self.lower_select = x,
        }
    }

    fn set_upper(&mut self, val: u8) {
        self.upper_select = val & 0x03;
    }

    fn mem_type_select(&mut self, val: u8) {
        match val & 1 {
            1 => self.mem_type = true, // RAM
            _ => self.mem_type = false, //ROM
        }
    }

    fn get_rom_bank(&self) -> u8 {
        match self.mem_type {
            false => (self.upper_select << 5) | self.lower_select,
            true => self.lower_select,
        }
    }

    fn get_ram_bank(&self) -> u8 {
        match self.mem_type {
            false => 0,
            true => self.upper_select,
        }
    }
}

#[derive(Clone)]
struct MB3 {
    ram_select: bool,
    reg_select: u8,

    second_reg: u8,
    minute_reg: u8,
    hour_reg: u8,
    day_count: u16,
    day_overflow: u8,

    halt: u8,
    time: PreciseTime,
}

impl MB3 {
    fn new() -> MB3 {
        MB3 {
            ram_select: true,
            reg_select: 8,
            second_reg: 0,
            minute_reg: 0,
            hour_reg: 0,
            day_count: 0,
            day_overflow: 0,
            halt: 0,
            time: PreciseTime::now(),
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

    fn get_rtc_reg(&self) -> u8 {
        let (sec,min,hour,day,over) = self.calc_time();
        match self.reg_select {
            0x8 => sec,
            0x9 => min,
            0xA => hour,
            0xB => (day % 512) as u8,
            _ => (day >> 8) as u8 | (self.halt << 6) | (over << 7),
        }
    }

    fn set_rtc_reg(&mut self, data: u8) {
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

    fn select_rtc(&mut self, reg: u8) {
        self.reg_select = reg;
    }

    fn latch_clock(&mut self) {
        if self.halt == 0 {
            self.calc_time();
            self.halt = 1;
        }
        else {
            self.time = PreciseTime::now();
            self.halt = 0;
        }
    }
}
