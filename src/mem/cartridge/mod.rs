// Cartridge module.

mod ram;
mod mbc1;

use ram::*;
use mbc1::MBC1;

use std::{
    collections::BTreeMap,
    io::{
        BufReader,
        Read,
        Seek,
        SeekFrom
    },
    fs::File
};

use super::MemDevice;

// Cartridge Memory Bank type
enum MBC {
    _0,
    _1(MBC1),
    _2,
    _3,
    _5(u16),
}

pub struct Cartridge {
    rom_bank_0:         [u8; 0x4000],
    rom_bank_cache:     BTreeMap<usize, Vec<u8>>,
    rom_bank_offset:    usize,

    ram:                Box<dyn RAM>,

    rom_file:           BufReader<File>,
    mem_bank:           MBC,
    ram_enable:         bool
}

impl Cartridge {
    pub fn new(rom_file_name: &str, save_file_name: &str) -> Result<Cartridge, String> {
        let f = File::open(rom_file_name).map_err(|e| e.to_string())?;

        let mut reader = BufReader::new(f);
        let mut buf = [0_u8; 0x4000];
        reader.read(&mut buf).map_err(|e| e.to_string())?;

        let (bank_type, battery, timer) = match buf[0x147] {
            0x1 | 0x2           => (MBC::_1(MBC1::new()), false, false),
            0x3                 => (MBC::_1(MBC1::new()), true, false),
            0x5                 => (MBC::_2, false, false),
            0x6                 => (MBC::_2, true, false),
            0xF | 0x10          => (MBC::_3, true, true),
            0x11 | 0x12         => (MBC::_3, false, false),
            0x13                => (MBC::_3, true, false),
            0x19 | 0x1A | 0x1C | 0x1D => (MBC::_5(0), false, false),
            0x1B | 0x1E         => (MBC::_5(0), true, false),
            _                   => (MBC::_0, false, false)
        };

        let ram_size = match (&bank_type, buf[0x149]) {
            (MBC::_2,_)     => 0x200,
            (_,0x1)         => 0x800,
            (_,0x2)         => 0x2000,
            (_,0x3)         => 0x8000,
            (_,0x4)         => 0x20000,
            (_,0x5)         => 0x10000,
            _               => 0,
        };

        let ram = if timer {
            Box::new(ClockRAM::new(ram_size, save_file_name)?) as Box<dyn RAM>
        } else if battery {
            Box::new(BatteryRAM::new(ram_size, save_file_name)?) as Box<dyn RAM>
        } else {
            Box::new(BankedRAM::new(ram_size)) as Box<dyn RAM>
        };

        let mut ret = Cartridge {
            rom_bank_0:         buf,
            rom_bank_cache:     BTreeMap::new(),
            rom_bank_offset:    0,
            ram:                ram,
            rom_file:           reader,
            mem_bank:           bank_type,
            ram_enable:         false
        };

        ret.swap_rom_bank(1);

        Ok(ret)
    }

    pub fn flush_ram(&mut self) {
        self.ram.flush();
    }

    // Get the cart name hash values for SGB palette lookup.
    pub fn cart_name_hash(&self) -> Option<(u8, u8)> {
        let old_code = self.read(0x014B);
        let valid = if old_code == 0x33 {
            let new_code = self.read(0x0145);
            (new_code == 0x31) || (new_code == 0x01)
        } else {
            old_code == 0x01
        };
        // Get hash.
        if valid {
            let mut title_loc = 0x0134;
            let mut hash = 0_u16;
            loop {
                let byte = self.read(title_loc);
                if byte == 0 {
                    break;
                } else {
                    hash += byte as u16;
                    title_loc += 1;
                }
            }
            let char_4 = self.read(0x0137);
            Some((hash as u8, char_4))
        } else {
            None
        }
    }

    // Check cart for cgb mode.
    pub fn cgb_cart(&self) -> bool {
        let cgb_flag = self.read(0x143);
        (cgb_flag & 0x80) != 0
    }
}

// Internal swapping methods.
impl Cartridge {
    fn swap_rom_bank(&mut self, bank: u16) {
        self.rom_bank_offset = (bank as usize) * 0x4000;

        if self.rom_bank_cache.get(&self.rom_bank_offset).is_none() {
            let mut rom_bank = vec![0; 0x4000];

            self.rom_file.seek(SeekFrom::Start(self.rom_bank_offset as u64))
                .expect("Couldn't swap in bank");

            self.rom_file.read_exact(&mut rom_bank)
                .expect(&format!("Couldn't swap in bank at pos {}-{}", self.rom_bank_offset, self.rom_bank_offset + 0x3FFF));

            self.rom_bank_cache.insert(self.rom_bank_offset, rom_bank);
        }
    }

    #[inline]
    fn swap_ram_bank(&mut self, bank: u8) {
        self.ram.set_bank(bank);
    }

    #[inline]
    fn read_ram(&self, loc: u16) -> u8 {
        if self.ram_enable {
            self.ram.read(loc)
        }
        else {
            0
        }
    }

    #[inline]
    fn write_ram(&mut self, loc: u16, val: u8) {
        if self.ram_enable {
            match self.mem_bank {
                MBC::_2 => self.ram.write(loc, val & 0xF),
                _ => self.ram.write(loc, val),
            }
        }
    }
}

impl MemDevice for Cartridge {
    fn read(&self, loc: u16) -> u8 {
        match loc {
            0x0..=0x3FFF    => self.rom_bank_0[loc as usize],
            0x4000..=0x7FFF => self.rom_bank_cache.get(&self.rom_bank_offset).expect("Bank not loaded!")[(loc - 0x4000) as usize],
            0xA000..=0xBFFF => self.read_ram(loc - 0xA000),
            _ => unreachable!()
        }
    }

    fn write(&mut self, loc: u16, val: u8) {
        if (loc >= 0xA000) && (loc < 0xC000) {
            self.write_ram(loc - 0xA000, val);
        } else {
            match self.mem_bank {
                MBC::_1(ref mut mb) => {
                    let old_rom_bank = mb.get_rom_bank();
                    let old_ram_bank = mb.get_ram_bank();
                    match loc {
                        0x0000..=0x1FFF => self.ram_enable = (val & 0xA) == 0xA,
                        0x2000..=0x3FFF => mb.set_lower(val),
                        0x4000..=0x5FFF => mb.set_upper(val),
                        _ => mb.mem_type_select(val),
                    }

                    let new_rom_bank = mb.get_rom_bank();
                    let new_ram_bank = mb.get_ram_bank();

                    if new_rom_bank != old_rom_bank {
                        self.swap_rom_bank(new_rom_bank as u16);
                    }
                    if new_ram_bank != old_ram_bank {
                        self.swap_ram_bank(new_ram_bank);
                    }
                },
                MBC::_2 => match loc {
                    0x0000..=0x1FFF => self.ram_enable = (loc & 0x100) == 0,
                    0x2000..=0x3FFF if (loc & 0x100) != 0 => self.swap_rom_bank((val & 0xF) as u16),
                    _ => {},
                },
                MBC::_3 => match (loc, val) {
                    (0x0000..=0x1FFF, _)    => self.ram_enable = (val & 0xF) == 0xA,
                    (0x2000..=0x3FFF, 0)    => self.swap_rom_bank(1),
                    (0x2000..=0x3FFF, _)    => self.swap_rom_bank((val & 0x7F) as u16),
                    (0x4000..=0x5FFF, _)    => self.swap_ram_bank(val),
                    _ => {},
                },
                MBC::_5(ref mut rom) => match (loc, val) {
                    (0x0000..=0x1FFF, _)    => self.ram_enable = (val & 0xF) == 0xA,
                    (0x2000..=0x2FFF, _)    => {
                        *rom &= 0xFF00;
                        *rom |= val as u16;
                        let rom_bank = *rom;
                        self.swap_rom_bank(rom_bank);
                    },
                    (0x3000..=0x3FFF, _)    => {
                        *rom &= 0xFF;
                        *rom |= 0x100;
                        let rom_bank = *rom;
                        self.swap_rom_bank(rom_bank);
                    },
                    (0x4000..=0x5FFF, _)    => self.swap_ram_bank(val),
                    _ => {},
                },
                _ => {},
            }
        }
    }
}