// mem.rs module: Memory bus and devices



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
            x @ 0x0000...0x7FFF => return,
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
        self.mem[loc as usize] = val
    }
}
