// mem.rs module: Memory bus and devices

mod cartridge;

use video::VideoDevice;
use audio::AudioDevice;
use timer::Timer;
use self::cartridge::Cartridge;


pub struct MemBus<V: VideoDevice> {
    cart:           Cartridge,

    ram_bank:       WriteableMem,
    ram:            WriteableMem,
    high_ram:       WriteableMem,

    interrupt_reg:  u8,

    video_device:   V,

    audio_device:   AudioDevice,

    timer:          Timer,
}

impl<V: VideoDevice> MemBus<V> {
    pub fn new(rom_file: &str, video_device: V, audio_device: AudioDevice) -> MemBus<V> {
        let rom = match Cartridge::new(rom_file) {
            Ok(r) => r,
            Err(s) => panic!("Could not construct ROM: {}", s),
        };

        MemBus {
            cart:           rom,
            ram_bank:       WriteableMem::new(0x2000),
            ram:            WriteableMem::new(0x2000),
            high_ram:       WriteableMem::new(0x80),
            interrupt_reg:  0,
            video_device:   video_device,
            audio_device:   audio_device,
            timer:          Timer::new(),
        }
    }

    pub fn read(&self, loc: u16) -> u8 {
        match loc {
            x @ 0x0000...0x7FFF => self.cart.read(x),
            x @ 0x8000...0x9FFF => self.video_device.read(x),
            x @ 0xA000...0xBFFF => self.cart.read(x),
            x @ 0xC000...0xDFFF => self.ram.read(x - 0xC000),
            x @ 0xE000...0xFDFF => self.ram.read(x - 0xE000),
            x @ 0xFE00...0xFE9F => self.video_device.read(x),
            x @ 0xFF00          => self.video_device.read(x),
            x @ 0xFF04...0xFF07 => self.timer.read(x),
                0xFF0F          => self.interrupt_reg,
            x @ 0xFF10...0xFF3F => self.audio_device.read(x),
            x @ 0xFF40...0xFF4B => self.video_device.read(x),
            x @ 0xFF80...0xFFFF => self.high_ram.read(x - 0xFF80),
            _ => self.ram.read(0),
        }
    }

    pub fn write(&mut self, loc: u16, val: u8) {
        match loc {
            x @ 0x0000...0x7FFF => self.cart.write(x, val),
            x @ 0x8000...0x9FFF => self.video_device.write(x, val),
            x @ 0xA000...0xBFFF => self.cart.write(x, val),
            x @ 0xC000...0xDFFF => self.ram.write(x - 0xC000, val),
            x @ 0xE000...0xFDFF => self.ram.write(x - 0xE000, val),
            x @ 0xFE00...0xFE9F => self.video_device.write(x, val),
            x @ 0xFF00          => self.video_device.write(x, val),
            x @ 0xFF04...0xFF07 => self.timer.write(x, val),
                0xFF0F          => self.interrupt_reg = val,
            x @ 0xFF10...0xFF3F => self.audio_device.write(x, val),
            x @ 0xFF40...0xFF45 => self.video_device.write(x, val),
                0xFF46          => self.dma(val),
            x @ 0xFF47...0xFF4B => self.video_device.write(x, val),
            x @ 0xFF80...0xFFFF => self.high_ram.write(x - 0xFF80, val),
            _ => {},
        }

        #[cfg(feature = "test")]
        self.update_debug_string(loc);
    }

    pub fn render_frame(&mut self) {
        self.audio_device.frame_update();
        //self.video_device.render_frame();
    }

    pub fn read_inputs(&mut self) {
        self.video_device.read_inputs();
    }

    pub fn inc_lcdc_y(&mut self) {
        self.video_device.inc_lcdc_y();
    }

    pub fn set_lcdc_y(&mut self, val: u8) {
        self.video_device.set_lcdc_y(val);
    }

    pub fn update_timers(&mut self, clock_count: u32) -> bool {
        self.audio_device.send_update(clock_count);
        self.timer.update_timers(clock_count)
    }

    fn dma(&mut self, val: u8) {
        let hi_byte = (val as u16) << 8;
        for lo_byte in 0_u16..=0x9F_u16 {
            let src_addr = hi_byte | lo_byte;
            let dest_addr = 0xFE00 | lo_byte;
            let byte = self.read(src_addr);
            self.video_device.write(dest_addr, byte);
        }
    }

    #[cfg(feature = "test")]
    fn update_debug_string(&self, loc: u16) {
        if self.ram.read(0) == 0x80 {
            if (loc > 0xC003) && (loc < 0xC100) {
                for i in 0..0xFF {
                    if self.ram.read(i) != 0 {
                        print!("{}", self.ram.read(i) as char);
                    } else {
                        println!("");
                        break;
                    }
                }
            }
        }

        //println!("Writing to {:X}", loc);

        if loc == 0xC000 {
            println!("DEBUG: {:X}", self.ram.read(0));
        }
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
