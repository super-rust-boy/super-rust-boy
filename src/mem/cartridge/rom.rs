// ROM sources.

use std::{
    collections::HashMap,
    io::{
        BufReader,
        Read,
        Seek,
        SeekFrom
    },
    fs::File
};

pub trait ROM {
    fn read(&self, loc: u16) -> u8;
    fn set_bank(&mut self, bank: u16);
}

// A local file.
pub struct ROMFile {
    bank_0:         [u8; 0x4000],
    bank_cache:     HashMap<usize, Vec<u8>>,
    bank_offset:    usize,

    file:           BufReader<File>,
}

impl ROMFile {
    pub fn new(file_name: &str) -> Result<Box<Self>, String> {
        let f = File::open(file_name).map_err(|e| e.to_string())?;

        let mut reader = BufReader::new(f);
        let mut buf = [0_u8; 0x4000];
        reader.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        reader.read_exact(&mut buf).map_err(|e| e.to_string())?;

        Ok(Box::new(ROMFile {
            bank_0:         buf,
            bank_cache:     HashMap::new(),
            bank_offset:    0,
            file:           reader,
        }))
    }
}

impl ROM for ROMFile {
    fn read(&self, loc: u16) -> u8 {
        match loc {
            0x0..=0x3FFF    => self.bank_0[loc as usize],
            0x4000..=0x7FFF => self.bank_cache.get(&self.bank_offset).expect("Bank not loaded!")[(loc - 0x4000) as usize],
            _ => unreachable!()
        }
    }

    fn set_bank(&mut self, bank: u16) {
        self.bank_offset = (bank as usize) * 0x4000;

        if self.bank_cache.get(&self.bank_offset).is_none() {
            let mut rom_bank = vec![0; 0x4000];

            self.file.seek(SeekFrom::Start(self.bank_offset as u64))
                .expect("Couldn't swap in bank");

            self.file.read_exact(&mut rom_bank)
                .expect(&format!("Couldn't swap in bank at pos {}-{}", self.bank_offset, self.bank_offset + 0x3FFF));

            self.bank_cache.insert(self.bank_offset, rom_bank);
        }
    }
}

// A raw blob.
pub struct ROMData {
    data:           Vec<u8>,
    bank_offset:    usize,
}

impl ROMData {
    pub fn new(data: &[u8]) -> Box<Self> {
        Box::new(ROMData {
            data:           Vec::from(data),
            bank_offset:    0,
        })
    }
}

impl ROM for ROMData {
    fn read(&self, loc: u16) -> u8 {
        match loc {
            0x0..=0x3FFF    => self.data[loc as usize],
            0x4000..=0x7FFF => self.data[self.bank_offset + (loc - 0x4000) as usize],
            _ => unreachable!()
        }
    }

    fn set_bank(&mut self, bank: u16) {
        self.bank_offset = (bank as usize) * 0x4000;
    }
}

// TODO: remote loading.