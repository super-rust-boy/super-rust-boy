// RAM

use std::{
    io::{
        BufReader,
        BufWriter,
        Read,
        Write,
        Seek,
        SeekFrom
    },
    fs::{
        File,
        OpenOptions
    }
};

use crate::mem::MemDevice;

pub trait RAM: MemDevice {
    fn set_offset(&mut self, offset: usize);
}

// Battery backed RAM
pub struct BatteryRAM {
    save_file:  BufWriter<File>,
    offset:     usize,
    ram:        Vec<u8>
}

impl BatteryRAM {
    pub fn new(ram_size: usize, save_file_name: &str) -> Result<Self, String> {
        let mut ram = vec![0; ram_size];

        if let Ok(file) = File::open(save_file_name) {
            //file.set_len(ram_size as u64).map_err(|e| e.to_string())?;
            let mut save_reader = BufReader::new(file);
            save_reader.read(&mut ram).map_err(|e| e.to_string())?;
        } else {
            let file = File::create(save_file_name).map_err(|e| e.to_string())?;
            file.set_len(ram_size as u64).map_err(|e| e.to_string())?;
        }

        let save_f = OpenOptions::new()
            .write(true)
            .open(save_file_name)
            .map_err(|e| e.to_string())?;

        Ok(BatteryRAM {
            save_file: BufWriter::new(save_f),
            offset: 0,
            ram: ram
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

        self.save_file.seek(SeekFrom::Start(pos as u64))
            .expect("Couldn't write to ram (seek)");

        self.save_file.write(&[val])
            .expect("Couldn't write to ram (write)");
    }
}

impl RAM for BatteryRAM {
    fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
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