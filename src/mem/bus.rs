// The main memory bus that connects to the CPU.

use crate::video::{
    sgbpalettes::*,
    VideoDevice
};
use crate::audio::AudioDevice;
use crate::timer::Timer;
use crate::interrupt::InterruptFlags;

use super::cartridge::Cartridge;
use super::{MemDevice, WriteableMem};

pub struct MemBus {
    cart:               Cartridge,

    ram:                WriteableMem,
    high_ram:           WriteableMem,

    interrupt_flag:     InterruptFlags,
    interrupt_enable:   InterruptFlags,

    video_device:       VideoDevice,

    audio_device:       AudioDevice,

    timer:              Timer,

    cgb_ram_offset:     u16
}

impl MemBus {
    pub fn new(rom_file: &str, save_file: &str, user_palette: UserPalette, audio_device: AudioDevice) -> MemBus {
        let rom = match Cartridge::new(rom_file, save_file) {
            Ok(r) => r,
            Err(s) => panic!("Could not construct ROM: {}", s),
        };

        let palette = match user_palette {
            UserPalette::Default => if let Some(cart_hash) = rom.cart_name_hash() {
                lookup_sgb_palette(cart_hash.0, cart_hash.1)
            } else {
                BW_PALETTE
            },
            UserPalette::Greyscale => BW_PALETTE,
            UserPalette::Classic => CLASSIC_PALETTE
        };

        let cgb_mode = (user_palette == UserPalette::Default) && rom.cgb_cart();

        MemBus {
            cart:               rom,
            ram:                WriteableMem::new(0x8000),
            high_ram:           WriteableMem::new(0x7F),
            interrupt_flag:     InterruptFlags::default(),
            interrupt_enable:   InterruptFlags::default(),
            video_device:       VideoDevice::new(palette, cgb_mode),
            audio_device:       audio_device,
            timer:              Timer::new(),
            cgb_ram_offset:     0x1000
        }
    }

    pub fn render_frame(&mut self) {
        self.audio_device.frame_update();
        self.video_device.render_frame();
    }

    // Send new audio update.
    pub fn update_audio(&mut self, clock_count: u32) {
        self.audio_device.send_update(clock_count);
    }

    // Increment timer.
    pub fn update_timer(&mut self) {
        if self.timer.update() {
            self.interrupt_flag.insert(InterruptFlags::TIMER);
        }
    }

    // Set the current video mode based on the cycle count.
    pub fn video_mode(&mut self, cycle_count: &mut u32) -> bool {
        let (ret, int) = self.video_device.video_mode(cycle_count);
        self.interrupt_flag.insert(int);
        ret
    }

    // Gets any interrupts that have been triggered and are enabled.
    pub fn get_interrupts(&self) -> InterruptFlags {
        self.interrupt_flag & self.interrupt_enable
    }

    // Clears an interrupt flag.
    pub fn clear_interrupt_flag(&mut self, flag: InterruptFlags) {
        self.interrupt_flag.remove(flag);
    }

    // Read inputs and update registers.
    pub fn read_inputs(&mut self) {
        if self.video_device.read_inputs() {
            self.interrupt_flag.insert(InterruptFlags::JOYPAD);
        }
    }

    // Flush the battery-backed RAM to disk.
    pub fn flush_cart(&mut self) {
        self.cart.flush_ram();
    }
}

// Internal functions
impl MemBus {
    // Direct memory access for palettes.
    fn dma(&mut self, val: u8) {
        let hi_byte = (val as u16) << 8;
        for lo_byte in 0_u16..=0x9F_u16 {
            let src_addr = hi_byte | lo_byte;
            let dest_addr = 0xFE00 | lo_byte;
            let byte = self.read(src_addr);
            self.video_device.write(dest_addr, byte);
        }
    }

    // Game Boy Color RAM bank.
    fn set_cgb_ram_bank(&mut self, val: u8) {
        let bank = (val & 0x7) as u16;
        self.cgb_ram_offset = if bank == 0 {
            0x1000
        } else {
            bank * 0x1000
        };
    }

    fn get_cgb_ram_bank(&self) -> u8 {
        (self.cgb_ram_offset / 0x1000) as u8
    }
}

impl MemDevice for MemBus {
    fn read(&self, loc: u16) -> u8 {
        match loc {
            0x0000..=0x7FFF => self.cart.read(loc),
            0x8000..=0x9FFF => self.video_device.read(loc),
            0xA000..=0xBFFF => self.cart.read(loc),
            0xC000..=0xCFFF => self.ram.read(loc - 0xC000),
            0xD000..=0xDFFF => self.ram.read((loc - 0xD000) + self.cgb_ram_offset),
            0xE000..=0xEFFF => self.ram.read(loc - 0xE000),
            0xF000..=0xFDFF => self.ram.read((loc - 0xF000) + self.cgb_ram_offset),
            0xFE00..=0xFE9F => self.video_device.read(loc),
            0xFF00          => self.video_device.read(loc),
            0xFF03..=0xFF07 => self.timer.read(loc),
            0xFF0F          => self.interrupt_flag.bits(),
            0xFF10..=0xFF3F => self.audio_device.read(loc),
            0xFF40..=0xFF4B => self.video_device.read(loc),
            0xFF68..=0xFF6B => self.video_device.read(loc),
            0xFF70          => self.get_cgb_ram_bank(),
            0xFF80..=0xFFFE => self.high_ram.read(loc - 0xFF80),
            0xFFFF          => self.interrupt_enable.bits(),
            _ => self.ram.read(0),
        }
    }

    fn write(&mut self, loc: u16, val: u8) {
        match loc {
            0x0000..=0x7FFF => self.cart.write(loc, val),
            0x8000..=0x9FFF => self.video_device.write(loc, val),
            0xA000..=0xBFFF => self.cart.write(loc, val),
            0xC000..=0xCFFF => self.ram.write(loc - 0xC000, val),
            0xD000..=0xDFFF => self.ram.write((loc - 0xD000) + self.cgb_ram_offset, val),
            0xE000..=0xEFFF => self.ram.write(loc - 0xE000, val),
            0xF000..=0xFDFF => self.ram.write((loc - 0xF000) + self.cgb_ram_offset, val),
            0xFE00..=0xFE9F => self.video_device.write(loc, val),
            0xFF00          => self.video_device.write(loc, val),
            0xFF03..=0xFF07 => self.timer.write(loc, val),
            0xFF0F          => self.interrupt_flag = InterruptFlags::from_bits_truncate(val),
            0xFF10..=0xFF3F => self.audio_device.write(loc, val),
            0xFF40..=0xFF45 => self.video_device.write(loc, val), 
            0xFF46          => self.dma(val),
            0xFF47..=0xFF4B => self.video_device.write(loc, val),
            0xFF68..=0xFF6B => self.video_device.write(loc, val),
            0xFF70          => self.set_cgb_ram_bank(val),
            0xFF80..=0xFFFE => self.high_ram.write(loc - 0xFF80, val),
            0xFFFF          => self.interrupt_enable = InterruptFlags::from_bits_truncate(val),
            _ => {},
        }
    }
}