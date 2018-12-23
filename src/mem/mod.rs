// mem.rs module: Memory bus and devices

mod cartridge;

use video::VideoDevice;
use self::cartridge::Cartridge;


pub struct MemBus<V: VideoDevice> {
    cart:           Cartridge,

    ram_bank:       WriteableMem,
    ram:            WriteableMem,
    video_device:   V,
    // IO ports
}

impl<V: VideoDevice> MemBus<V> {
    pub fn new(rom_file: &str, video_device: V) -> MemBus<V> {
        let rom = match Cartridge::new(rom_file) {
            Ok(r) => r,
            Err(s) => panic!("Could not construct ROM: {}", s),
        };

        MemBus {
            cart:           rom,
            ram_bank:       WriteableMem::new(0x2000),
            ram:            WriteableMem::new(0x2000),
            video_device:   video_device,
        }
    }

    pub fn read(&self, loc: u16) -> u8 {
        match loc {
            x @ 0x0000...0x7FFF => self.cart.read(x),
            x @ 0x8000...0x9FFF => self.video_device.read(x),
            x @ 0xA000...0xBFFF => self.ram_bank.read(x - 0xA000),
            x @ 0xC000...0xDFFF => self.ram.read(x - 0xC000),
            x @ 0xE000...0xFDFF => self.ram.read(x - 0xE000),
            x @ 0xFE00...0xFE9F => self.video_device.read(x),
            x @ 0xFF40...0xFF4B => self.video_device.read(x),
            _ => self.ram.read(0),
        }
    }

    pub fn write(&mut self, loc: u16, val: u8) {
        match loc {
            x @ 0x0000...0x7FFF => self.cart.write(x, val),
            x @ 0x8000...0x9FFF => self.video_device.write(x, val),
            x @ 0xA000...0xBFFF => self.ram_bank.write(x - 0xA000, val),
            x @ 0xC000...0xDFFF => self.ram.write(x - 0xC000, val),
            x @ 0xE000...0xFDFF => self.ram.write(x - 0xE000, val),
            x @ 0xFE00...0xFE9F => self.video_device.write(x, val),
            x @ 0xFF40...0xFF4B => self.video_device.write(x, val),
            _ => return,
        }
    }

    pub fn trigger_frame(&mut self) {
        self.video_device.render_frame();
    }
}


pub trait MemDevice {
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
