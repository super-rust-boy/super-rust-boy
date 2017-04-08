// mem.rs module: Memory bus and devices

use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::fs::File;


pub struct MemBus {
    cart: Cartridge,

    vram: WriteableMem,
    ram_bank: WriteableMem,
    ram: WriteableMem,
    sprite_mem: WriteableMem,
    // IO ports
}

impl MemBus {
    pub fn new(rom_file: &str) -> MemBus {
        let rom = match Cartridge::new(rom_file) {
            Ok(r) => r,
            Err(s) => panic!("Could not construct ROM: {}", s),
        };
        MemBus {
            cart: rom,
            vram: WriteableMem::new(0x2000),
            ram_bank: WriteableMem::new(0x2000),
            ram: WriteableMem::new(0x2000),
            sprite_mem: WriteableMem::new(0xA0),
        }
    }

    pub fn read(&self, loc: u16) -> u8 {
        match loc {
            x @ 0x0000...0x7FFF => self.cart.read(x),
            x @ 0x8000...0x9FFF => self.vram.read(x - 0x8000),
            x @ 0xA000...0xBFFF => self.ram_bank.read(x - 0xA000),
            x @ 0xC000...0xDFFF => self.ram.read(x - 0xC000),
            x @ 0xE000...0xFDFF => self.ram.read(x - 0xE000),
            x @ 0xFE00...0xFE9F => self.sprite_mem.read(x - 0xFE00),
            _ => self.ram.read(0),
        }
    }

    pub fn write(&mut self, loc: u16, val: u8) {
        match loc {
            x @ 0x0000...0x7FFF => self.cart.write(x, val),
            x @ 0x8000...0x9FFF => self.vram.write(x- 0x8000, val),
            x @ 0xA000...0xBFFF => self.ram_bank.write(x - 0xA000, val),
            x @ 0xC000...0xDFFF => self.ram.write(x - 0xC000, val),
            x @ 0xE000...0xFDFF => self.ram.write(x - 0xE000, val),
            x @ 0xFE00...0xFE9F => self.sprite_mem.write(x - 0xFE00, val),
            _ => return,
        }
    }
}


trait MemDevice {
    fn read(&self, loc: u16) -> u8;
    fn write(&mut self, loc: u16, val: u8);
}


struct WriteableMem {
    mem: Vec<u8>,
}

impl WriteableMem {
    fn new(size: usize) -> WriteableMem {
        WriteableMem {mem: vec![0;size]}
    }
}

impl MemDevice for WriteableMem {
    fn read(&self, loc: u16) -> u8 {
        self.mem[loc as usize]
    }

    fn write(&mut self, loc: u16, val: u8) {
        self.mem[loc as usize] = val;
    }
}


enum MBC {
    _0,
    _1(u8, bool),
    _2(u8),
    _3(u8),
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
}

impl Cartridge {
    fn new(rom_file: &str) -> Result<Cartridge, String> {
        let f = try!(File::open(rom_file).map_err(|e| e.to_string()));

        let mut reader = BufReader::new(f);
        let mut buf: [u8; 0x4000] = [0; 0x4000];
        try!(reader.read_exact(&mut buf).map_err(|e| e.to_string()));

        let bank_type = match buf[0x147] {
            0x1...0x3 => MBC::_1(0,false),
            0x5...0x6 => MBC::_2(0),
            0xF...0x13 => MBC::_3(0),
            0x15...0x17 => MBC::_4(0),
            0x19...0x1E => MBC::_5(0),
            _ => MBC::_0,
        };

        let ram_size = match buf[0x149] {
            0x1 => 0x800,
            0x2 => 0x2000,
            0x3 => 0x8000,
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
        };
        Ok(ret)
    }

    fn swap_bank(&mut self) {
        let res = match self.mem_bank {
            MBC::_0 => self.read_bank(1),
            MBC::_1(x,_) if x % 0x20 == 0 => self.read_bank(x+1),
            MBC::_1(x,_) => self.read_bank(x),
            MBC::_2(x) => self.read_bank(x),
            MBC::_3(x) => self.read_bank(x),
            MBC::_4(x) => self.read_bank(x),
            MBC::_5(x) => self.read_bank(x),
        };
        match res {
            Ok(_) => return,
            Err(s) => panic!("Couldn't swap in bank: {}", s),
        }
    }

    fn read_bank(&mut self, bank: u8) -> Result<(), String> {
        let pos = (bank as u64) * 0x4000;
        try!(self.rom_file.seek(SeekFrom::Start(pos)).map_err(|e| e.to_string()));
        try!(self.rom_file.read_exact(&mut self.rom_bank_n).map_err(|e| e.to_string()));
        Ok(())
    }

    #[inline]
    fn read_ram(&self, loc: u16) -> u8 {
        if self.ram_enable {
            return self.ram[self.ram_offset + (loc as usize)];
        }
        else {
            return 0;
        }
    }

    #[inline]
    fn write_ram(&mut self, loc: u16, val: u8) {
        if self.ram_enable {
            self.ram[self.ram_offset + (loc as usize)] = val;
        }
    }

    // Cartridge types when writing...
    #[inline]
    fn write_mb1(&mut self, mem_bank: u8, mem_mode: bool, loc: u16, val: u8) {
        let mut b = mem_bank;
        let mut m = mem_mode;
        match loc {
            0x0000...0x1FFF => self.ram_enable = if (val & 0xF) == 0xA {true} else {false},
            0x2000...0x3FFF => b = (b & 0xE0) | (val & 0x1F),
            0x4000...0x5FFF => b = (b & 0x1F) | (val & 0x60),
            _ => m = if (val & 1) == 0 {true} else {false},
        }
        self.mem_bank = MBC::_1(b,m);
        let bank = if m {b} else {b & 0x1F};
        match self.read_bank(bank) {
            Ok(_) => return,
            Err(s) => panic!("Couldn't swap in bank: {}", s),
        }
    }

    #[inline]
    fn write_mb2(&mut self, bank_num: u8, loc: u16, val: u8) {
        match loc {
            0x0000...0x1FFF => self.ram_enable = if (val & 0xF) == 0xA {true} else {false},
            0x2000...0x3FFF => self.mem_bank = MBC::_1((bank_num & 0xE0) | (val & 0x1F), false),
            //0x4000...0x5FFF => self.
            _ => return,
        }
    }

    #[inline]
    fn write_mb3(&mut self, bank_num: u8, loc: u16, val: u8) {
        match loc {
            0x0000...0x1FFF => self.ram_enable = if (val & 0xF) == 0xA {true} else {false},
            0x2000...0x3FFF => self.mem_bank = MBC::_1((bank_num & 0xE0) | (val & 0x1F), false),
            //0x4000...0x5FFF => self.
            _ => return,
        }
    }
}

impl MemDevice for Cartridge {
    fn read(&self, loc: u16) -> u8 {
        match loc {
            0x0...0x3FFF => self.rom_bank_0[loc as usize],
            0x4000...0x7FFF => self.rom_bank_n[(loc - 0x4000) as usize],
            _ => self.read_ram(loc % 0x2000),
        }
    }

    fn write(&mut self, loc: u16, val: u8) {
        if (loc >= 0xA000) && (loc < 0xC000) {
            self.write_ram(loc % 0x2000, val);
        }
        else {
            match self.mem_bank {
                MBC::_1(x,r) => self.write_mb1(x, r, loc, val),
                MBC::_2(x) => self.write_mb2(x, loc, val),
                MBC::_3(x) => self.write_mb3(x, loc, val),
                _ => return,
            }
        }
    }
}
