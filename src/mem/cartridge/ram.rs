// RAM

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
    fn set_offset(&mut self, offset: usize);
    fn flush(&mut self) {}
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
            save_reader.read(&mut ram).map_err(|e| e.to_string())?;
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
    fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }

    fn flush(&mut self) {
        if self.dirty {
            let save_f = OpenOptions::new()
                .write(true)
                .open(self.save_file.as_str())
                .expect("Couldn't open file");

            let mut bufwriter = BufWriter::new(save_f);

            bufwriter.write_all(&self.ram).expect("Couldn't write to file");
        }
    }
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
    fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }
}