// mem.rs module: Memory bus and devices

use std::io::BufReader;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::fs::File;


pub struct MemBus {
    // TODO: Cart type
    cart: WriteableMem,

    vram: WriteableMem,
    ram_bank: WriteableMem,
    ram: WriteableMem,
    sprite_mem: WriteableMem,
    // IO ports
}

impl MemBus {
    pub fn new() -> MemBus {
        MemBus {
            cart: WriteableMem::new(0x8000),
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
        WriteableMem {mem: Vec::with_capacity(size)}
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
    _1(u8),
    _2(u8),
    _3(u8),
    _4(u8),
    _5(u8),
}

struct ReadOnlyMem {
    bank_0: [u8; 0x4000],
    bank_n: [u8; 0x4000],
    rom_file: BufReader<File>,
    mem_bank: MBC,
}

impl ReadOnlyMem {
    fn new(rom_file: String) -> Result<ReadOnlyMem, String> {
        let mut f = try!(File::open(rom_file).map_err(|e| e.to_string()));

        let mut reader = BufReader::new(f);
        let mut buf: [u8; 0x4000] = [0; 0x4000];
        try!(reader.read_exact(&mut buf).map_err(|e| e.to_string()));
        let bank_type = match buf[0x147] {
            0x1...0x3 => MBC::_1(0),
            0x5...0x6 => MBC::_2(0),
            0xF...0x13 => MBC::_3(0),
            0x15...0x17 => MBC::_4(0),
            0x19...0x1E => MBC::_5(0),
            _ => MBC::_0,
        };

        let ret = ReadOnlyMem {
            bank_0: buf,
            bank_n: [0; 0x4000],
            rom_file: reader,
            mem_bank: bank_type,
        };
        Ok(ret)
    }

    fn swap_bank(&mut self) {
        let res = match self.mem_bank {
            MBC::_0 => self.read_bank(1),
            MBC::_1(x) if x % 0x20 == 0 => self.read_bank(x+1),
            MBC::_1(x) => self.read_bank(x),
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
        try!(self.rom_file.read_exact(&mut self.bank_n).map_err(|e| e.to_string()));
        Ok(())
    }
}

impl MemDevice for ReadOnlyMem {
    fn read(&self, loc: u16) -> u8 {
        self.bank_0[loc as usize]
    }

    fn write(&mut self, loc: u16, val: u8) {
        return;
    }
}
