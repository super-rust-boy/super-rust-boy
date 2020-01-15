// The main memory bus that connects to the CPU.

use crate::{
    video::{
        sgbpalettes::*,
        VideoDevice,
        VulkanRenderer
    },
    audio::AudioDevice,
    timer::Timer,
    joypad::*,
    interrupt::InterruptFlags
};

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
    joypad:             Joypad,

    // DMA
    dma_addr:           u16,
    dma_active:         bool,

    // CGB
    cgb_ram_offset:     u16,
    cgb_dma_src:        u16,
    cgb_dma_dst:        u16,
    cgb_dma_len:        u16,
    cgb_dma_hblank_len: Option<u16>,

    cgb_mode:           bool
}

impl MemBus {
    pub fn new(rom_file: &str, save_file: &str, user_palette: UserPalette, audio_device: AudioDevice, renderer: Box<VulkanRenderer>) -> MemBus {
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

            video_device:       VideoDevice::new(renderer, palette, cgb_mode),
            audio_device:       audio_device,
            timer:              Timer::new(),
            joypad:             Joypad::new(),

            dma_addr:           0,
            dma_active:         false,

            cgb_ram_offset:     0x1000,
            cgb_dma_src:        0x0FF0,
            cgb_dma_dst:        0x8FF0,
            cgb_dma_len:        0,
            cgb_dma_hblank_len: None,
            cgb_mode:           cgb_mode
        }
    }

    pub fn render_frame(&mut self) {
        self.audio_device.frame_update();
        self.video_device.frame();
        if self.joypad.check_interrupt() {
            self.interrupt_flag.insert(InterruptFlags::JOYPAD);
        }
    }
    
    pub fn transfer_image(&mut self, buffer: &mut [u32]) {
        self.video_device.transfer_image(buffer);
    }

    // Send new audio update.
    pub fn update_audio(&mut self, cycles: u32) {
        self.audio_device.send_update(cycles);
    }

    // Clock memory: update timer and DMA transfers.
    // Return true if CGB DMA is active.
    pub fn clock(&mut self, cycles: u32) -> bool {
        if self.timer.update(cycles) {
            self.interrupt_flag.insert(InterruptFlags::TIMER);
        }
        if self.dma_active {
            self.dma_tick();
        }
        if self.cgb_dma_len > 0 {
            self.cgb_dma_tick();
            if cycles == 4 && self.cgb_dma_len > 0 {    // In single speed mode, transfer 2 bytes per instruction.
                self.cgb_dma_tick();
            }

            (self.cgb_dma_hblank_len.is_none() && (self.cgb_dma_len != 0)) ||
            ((self.cgb_dma_hblank_len.unwrap_or(0) > 0) && self.video_device.is_in_hblank())
        } else {
            false
        }
    }

    // Set the current video mode based on the cycle count.
    // Returns true if V-blank has been entered.
    pub fn video_mode(&mut self, cycles: u32) -> bool {
        let (ret, int) = self.video_device.video_mode(cycles);
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

    pub fn set_button(&mut self, button: Buttons, val: bool) {
        self.joypad.set_button(button, val);
    }

    pub fn set_direction(&mut self, direction: Directions, val: bool) {
        self.joypad.set_direction(direction, val);
    }

    pub fn on_resize(&mut self) {
        self.video_device.on_resize();
    }

    // Flush the battery-backed RAM to disk.
    pub fn flush_cart(&mut self) {
        self.cart.flush_ram();
    }

    // See if the memory is in CGB mode.
    pub fn is_cgb(&self) -> bool {
        self.cgb_mode
    }
}

// Internal functions
impl MemBus {
    // Direct memory access for object memory.
    fn start_dma(&mut self, val: u8) {
        self.dma_addr = make_16!(val, 0);
        self.dma_active = true;
    }

    fn dma_tick(&mut self) {
        let dest_addr = make_16!(0xFE, lo_16!(self.dma_addr));
        let byte = self.read(self.dma_addr);
        self.video_device.write(dest_addr, byte);
        self.dma_addr += 1;

        if lo_16!(self.dma_addr) >= 0xA0 {
            self.dma_active = false;
        }
    }

    // Direct memory access for CGB.
    fn start_cgb_dma(&mut self, val: u8) {
        if self.cgb_dma_hblank_len.is_some() && !test_bit!(val, 7) {
            self.cgb_dma_len = 0; // This shouldn't actually set this to zero.
            self.cgb_dma_hblank_len = None;
        } else {
            self.cgb_dma_len = ((val & 0x7F) as u16 + 1) * 0x10;
            self.cgb_dma_hblank_len = if !test_bit!(val, 7) {None} else {Some(0x10)};
        }
    }

    // Get CGB DMA remaining length.
    fn get_cgb_len(&self) -> u8 {
        if self.cgb_dma_len == 0 {
            0xFF
        } else {
            ((self.cgb_dma_len - 1) / 0x10) as u8
        }
    }

    fn cgb_dma_tick(&mut self) {
        // Transfer 16 bytes each H-Blank.
        if let Some(l) = self.cgb_dma_hblank_len {
            if (l > 0) && self.video_device.is_in_hblank() {
                self.cgb_dma_transfer();
                self.cgb_dma_hblank_len = Some(l - 1);
            } else if (l == 0) && !self.video_device.is_in_hblank() {
                self.cgb_dma_hblank_len = Some(0x10);
            }
        } else {
            self.cgb_dma_transfer();
        }
    }

    // Transfer of one byte.
    fn cgb_dma_transfer(&mut self) {
        let byte = self.read(self.cgb_dma_src);
        self.write(self.cgb_dma_dst, byte);

        self.cgb_dma_src += 1;
        self.cgb_dma_dst += 1;
        self.cgb_dma_len -= 1;
    }

    // Setting CGB DMA source and destination addresses.
    fn set_cgb_dma_upper_src(&mut self, val: u8) {
        self.cgb_dma_src = (self.cgb_dma_src & 0xFF) | ((val as u16) << 8);
    }
    fn set_cgb_dma_lower_src(&mut self, val: u8) {
        self.cgb_dma_src = (self.cgb_dma_src & 0xFF00) | ((val as u16) & 0xF0);
    }

    fn set_cgb_dma_upper_dst(&mut self, val: u8) {
        self.cgb_dma_dst = (self.cgb_dma_dst & 0xFF) | (((val as u16) & 0x1F) << 8) | 0x8000;
    }
    fn set_cgb_dma_lower_dst(&mut self, val: u8) {
        self.cgb_dma_dst = (self.cgb_dma_dst & 0xFF00) | ((val as u16) & 0xF0);
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
            0xFF00          => self.joypad.read(),
            0xFF01..=0xFF02 => 0,
            0xFF03..=0xFF07 => self.timer.read(loc),
            0xFF0F          => self.interrupt_flag.bits(),
            0xFF10..=0xFF3F => self.audio_device.read(loc),
            0xFF40..=0xFF45 => self.video_device.read(loc),
            0xFF46          => (self.dma_addr >> 8) as u8,
            0xFF47..=0xFF4B => self.video_device.read(loc),
            0xFF4F          => self.video_device.read(loc),
            0xFF55          => self.get_cgb_len(),
            0xFF68..=0xFF6B => self.video_device.read(loc),
            0xFF70          => self.get_cgb_ram_bank(),
            0xFF80..=0xFFFE => self.high_ram.read(loc - 0xFF80),
            0xFFFF          => self.interrupt_enable.bits(),
            _ => 0xFF,
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
            0xFF00          => self.joypad.write(val),
            0xFF03..=0xFF07 => self.timer.write(loc, val),
            0xFF0F          => self.interrupt_flag = InterruptFlags::from_bits_truncate(val),
            0xFF10..=0xFF3F => self.audio_device.write(loc, val),
            0xFF40..=0xFF45 => self.video_device.write(loc, val), 
            0xFF46          => self.start_dma(val),
            0xFF47..=0xFF4F => self.video_device.write(loc, val),
            0xFF51          => self.set_cgb_dma_upper_src(val),
            0xFF52          => self.set_cgb_dma_lower_src(val),
            0xFF53          => self.set_cgb_dma_upper_dst(val),
            0xFF54          => self.set_cgb_dma_lower_dst(val),
            0xFF55          => self.start_cgb_dma(val),
            0xFF68..=0xFF6B => self.video_device.write(loc, val),
            0xFF70          => self.set_cgb_ram_bank(val),
            0xFF80..=0xFFFE => self.high_ram.write(loc - 0xFF80, val),
            0xFFFF          => self.interrupt_enable = InterruptFlags::from_bits_truncate(val),
            _ => {},
        }
    }
}