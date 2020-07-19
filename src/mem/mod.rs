// Memory device, bus and cartridges

mod bus;
mod cartridge;

pub use bus::MemBus;
pub use cartridge::ROMType;

pub trait MemDevice {
    fn read(&self, loc: u16) -> u8;
    fn write(&mut self, loc: u16, val: u8);
}

pub struct WriteableMem {
    mem: Vec<u8>,
}

impl WriteableMem {
    pub fn new(size: usize) -> WriteableMem {
        WriteableMem {mem: vec![0; size]}
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