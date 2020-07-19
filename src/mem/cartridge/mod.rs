// Cartridge module.

mod ram;
mod rom;
mod mbc1;

use ram::*;
use rom::*;
use mbc1::MBC1;

use super::MemDevice;

pub enum ROMType {
    File(String),
    Data(Vec<u8>),
}

// Cartridge Memory Bank type
enum MBC {
    _0,
    _1(MBC1),
    _2,
    _3,
    _5(u16),
}

// Cartridge extra features
enum CartFeatures {
    None,
    Battery,
    Timer
}

pub struct Cartridge {
    rom:        Box<dyn ROM>,
    ram:        Box<dyn RAM>,

    mem_bank:   MBC,
    ram_enable: bool
}

impl Cartridge {
    pub fn new(rom_type: ROMType, save_file_name: &str) -> Result<Cartridge, String> {
        let rom = match rom_type {
            ROMType::File(file_name) => ROMFile::new(&file_name)? as Box<dyn ROM>,
            ROMType::Data(data) => ROMData::new(&data) as Box<dyn ROM>,
        };

        let (bank_type, features) = match rom.read(0x147) {
            0x1 | 0x2           => (MBC::_1(MBC1::new()), CartFeatures::None),
            0x3                 => (MBC::_1(MBC1::new()), CartFeatures::Battery),
            0x5                 => (MBC::_2,              CartFeatures::None),
            0x6                 => (MBC::_2,              CartFeatures::Battery),
            0xF | 0x10          => (MBC::_3,              CartFeatures::Timer),
            0x11 | 0x12         => (MBC::_3,              CartFeatures::None),
            0x13                => (MBC::_3,              CartFeatures::Battery),
            0x19 | 0x1A | 0x1C | 0x1D => (MBC::_5(0),     CartFeatures::None),
            0x1B | 0x1E         => (MBC::_5(0),           CartFeatures::Battery),
            _                   => (MBC::_0,              CartFeatures::None)
        };

        let ram_size = match (&bank_type, rom.read(0x149)) {
            (MBC::_2,_)     => 0x200,
            (_,0x1)         => 0x800,
            (_,0x2)         => 0x2000,
            (_,0x3)         => 0x8000,
            (_,0x4)         => 0x20000,
            (_,0x5)         => 0x10000,
            _               => 0,
        };

        let ram: Box<dyn RAM> = match features {
            CartFeatures::None      => Box::new(BankedRAM::new(ram_size)),
            CartFeatures::Battery   => Box::new(BatteryRAM::new(ram_size, save_file_name)?),
            CartFeatures::Timer     => Box::new(ClockRAM::new(ram_size, save_file_name)?)
        };

        let mut ret = Cartridge {
            rom:                rom,
            ram:                ram,
            mem_bank:           bank_type,
            ram_enable:         false
        };

        ret.swap_rom_bank(1);

        Ok(ret)
    }

    pub fn flush_ram(&mut self) {
        self.ram.flush();
    }

    // Get the ROM name.
    pub fn name(&self) -> String {
        use std::str::FromStr;

        let old_code = self.read(0x014B);
        let title_end = if old_code == 0x33 {
            0x13E
        } else {
            0x143
        };

        let mut name_bytes = Vec::new();
        for title_loc in 0x134..=title_end {
            let byte = self.read(title_loc);
            if byte == 0 {
                break;
            } else {
                name_bytes.push(byte);
            }
        }

        String::from_str(std::str::from_utf8(&name_bytes).unwrap()).unwrap()
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
        test_bit!(cgb_flag, 7)
    }
}

// Internal swapping methods.
impl Cartridge {
    fn swap_rom_bank(&mut self, bank: u16) {
        self.rom.set_bank(bank);
    }

    #[inline]
    fn swap_ram_bank(&mut self, bank: u8) {
        self.ram.set_bank(bank, 0);
    }

    #[inline]
    fn read_ram(&self, loc: u16) -> u8 {
        if self.ram_enable {
            self.ram.read(loc)
        } else {
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
            0x0..=0x7FFF    => self.rom.read(loc),
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
                    (0x4000..=0x7FFF, _)    => self.ram.set_bank(val, loc),
                    _ => unreachable!(),
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
                        *rom |= ((val & 1) as u16) << 8;
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