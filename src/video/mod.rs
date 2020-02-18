//mod mem;
mod types;
mod renderer;
mod vram;
mod regs;

pub mod sgbpalettes;

// Video mode constants
mod constants {
    // Mode cycle counts
    pub const H_CYCLES: u32     = 456;              // Cycles per Line
    pub const MODE_1: u32       = 154 * H_CYCLES;   // Mode 1: V-Blank
    pub const MODE_2: u32       = 80;               // Mode 2: Reading OAM
    pub const MODE_3: u32       = MODE_2 + 168;     // Mode 3: Reading OAM & VRAM
    pub const FRAME_CYCLE: u32  = 144 * H_CYCLES;   // Time spent cycling through modes 2,3 and 0 before V-Blank
}

use crate::interrupt::InterruptFlags;
use crate::mem::MemDevice;

//use mem::VideoMem;
use sgbpalettes::SGBPalette;
use regs::VideoRegs;

pub use types::{
    Colour,
    PaletteColours
};

use vram::VRAM;

use renderer::*;

use std::sync::{
    Arc,
    Mutex
};

pub use sgbpalettes::UserPalette;

// Modes
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Mode {
    _0 = 0, // H-blank
    _1 = 1, // V-blank
    _2 = 2, // Reading OAM
    _3 = 3  // Drawing
}

impl From<u8> for Mode {
    fn from(val: u8) -> Self {
        match val & 0b11 {
            0 => Mode::_0,
            1 => Mode::_1,
            2 => Mode::_2,
            _ => Mode::_3
        }
    }
}

pub struct VideoDevice {
    vram:           Arc<Mutex<VRAM>>,
    regs:           VideoRegs,

    renderer:       Renderer,

    // CGB things
    cgb_mode:       bool,
    vram_bank:      u8,

    // Misc
    cycle_count:    u32,
}

impl VideoDevice {
    pub fn new(palette: SGBPalette, cgb_mode: bool) -> Self {
        let vram = Arc::new(Mutex::new(VRAM::new(palette, cgb_mode)));

        // Spin off video thread.
        let renderer = Renderer::new(vram.clone());

        VideoDevice {
            vram:           vram,
            regs:           VideoRegs::new(),

            renderer:       renderer,

            // CGB things
            cgb_mode:       cgb_mode,
            vram_bank:      0,

            // Misc
            cycle_count:    0,
        }
    }

    // Drawing for a single frame.
    pub fn start_frame(&mut self, render_target: RenderTarget) {
        self.renderer.start_frame(render_target);
    }

    // Query to see if the video device is in H-Blank.
    pub fn is_in_hblank(&self) -> bool {
        self.regs.read_mode() == Mode::_0
    }

    // Set the current video mode based on the cycle count.
    // May trigger an interrupt.
    // Returns true if transitioned to V-Blank.
    pub fn video_mode(&mut self, cycles: u32) -> (bool, InterruptFlags) {
        use self::constants::*;
        //let mut mem = self.mem.lock().unwrap();
        self.inc_cycle_count(cycles);

        if self.regs.is_display_enabled() {
            // First, calculate how many cycles into the horizontal line we are.
            let line_cycle = self.get_cycle_count() % H_CYCLES;
            let mode = self.regs.read_mode();

            let int = match mode {
                Mode::_2 if line_cycle >= MODE_2 => self.update_mode(Mode::_3),
                Mode::_3 if line_cycle >= MODE_3 => self.update_mode(Mode::_0),
                Mode::_0 if self.get_cycle_count() >= FRAME_CYCLE => {
                    self.regs.inc_lcdc_y();
                    self.update_mode(Mode::_1) | InterruptFlags::V_BLANK
                },
                Mode::_0 if line_cycle < MODE_3 => {
                    self.regs.inc_lcdc_y();
                    self.update_mode(Mode::_2)
                },
                Mode::_1 => if self.get_cycle_count() >= MODE_1 {
                    //self.renderer.frame_start(&mut self.mem);
                    self.renderer.end_frame();
                    self.regs.set_lcdc_y(0);
                    self.frame_cycle_reset();
                    self.update_mode(Mode::_2)
                } else {
                    let new_ly = (self.get_cycle_count() / H_CYCLES) as u8;
                    self.regs.set_lcdc_y(new_ly);
                    InterruptFlags::default()
                },
                _ => InterruptFlags::default(),
            };

            (if int.contains(InterruptFlags::V_BLANK) {
                true
            } else {
                false
            }, int)
        } else {
            let keep_cycling = if self.get_cycle_count() > MODE_1 {
                self.frame_cycle_reset();
                true
            } else {
                false
            };
            (keep_cycling, InterruptFlags::default())
        }
    }

    // Update status reg, Trigger LCDC Status interrupt if necessary
    fn update_mode(&mut self, mode: Mode) -> InterruptFlags {
        use regs::LCDStatusFlags;
        //let mem = self.mem.lock().unwrap();

        self.regs.write_mode(mode);
        let stat_flags = self.regs.read_flags();

        if mode == Mode::_0 {
            self.renderer.draw_line(self.regs.clone());
        }

        // Trigger STAT interrupt
        if !stat_flags.is_empty() {
            // LY Coincidence interrupt
            if stat_flags.contains(LCDStatusFlags::COINCEDENCE_INT) {
                if self.regs.compare_ly_equal() {
                    return InterruptFlags::LCD_STAT;
                }
            } else if stat_flags.contains(LCDStatusFlags::OAM_INT) {
                if mode == Mode::_2 {
                    return InterruptFlags::LCD_STAT;
                }
            } else if stat_flags.contains(LCDStatusFlags::V_BLANK_INT) {
                if mode == Mode::_1 {
                    return InterruptFlags::LCD_STAT;
                }
            } else if stat_flags.contains(LCDStatusFlags::H_BLANK_INT) {
                if mode == Mode::_0 {
                    return InterruptFlags::LCD_STAT;
                }
            }
        }

        InterruptFlags::default()
    }

    // Draw a single line
    /*fn draw_line(&mut self) {
        self.renderer.draw_line(self.mem.get_lcd_y(), &mut self.mem, self.cgb_mode);
    }*/
}

impl VideoDevice {
    fn inc_cycle_count(&mut self, cycles: u32) {
        self.cycle_count += cycles;
    }

    fn frame_cycle_reset(&mut self) {
        self.cycle_count -= 154 * 456;
    }

    fn get_cycle_count(&self) -> u32 {
        self.cycle_count
    }
}

/*impl MemDevice for VideoDevice {
    fn read(&self, loc: u16) -> u8 {
        self.mem.read(loc)
    }

    fn write(&mut self, loc: u16, val: u8) {
        self.mem.write(loc, val);
    }
}*/

impl MemDevice for VideoDevice {
    fn read(&self, loc: u16) -> u8 {
        match loc {
            // Raw tile data
            0x8000..=0x97FF if self.regs.can_access_vram() => {
                let base = (loc - 0x8000) as usize + (self.vram_bank as usize * 0x1800);

                if base % 2 == 0 {  // Lower bit
                    self.vram.lock().unwrap().tile_mem.get_pixel_lower_row(base)
                } else {            // Upper bit
                    self.vram.lock().unwrap().tile_mem.get_pixel_upper_row(base)
                }
            },
            // Background Map A
            0x9800..=0x9BFF if self.regs.can_access_vram() => {
                /*let base = (loc - 0x9800) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                if self.vram_bank == 0 {
                    self.tile_map_0.get_tile_texture(x, y)
                } else {
                    self.tile_map_0.get_tile_attribute(x, y)
                }*/
                let index = (loc - 0x9800) as usize;
                if self.vram_bank == 0 {
                    self.vram.lock().unwrap().tile_map_0[index]
                } else {
                    self.vram.lock().unwrap().tile_attrs_0[index]
                }
            },
            // Background Map B
            0x9C00..=0x9FFF if self.regs.can_access_vram() => {
                /*let base = (loc - 0x9C00) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                if self.vram_bank == 0 {
                    self.tile_map_1.get_tile_texture(x, y)
                } else {
                    self.tile_map_1.get_tile_attribute(x, y)
                }*/
                let index = (loc - 0x9C00) as usize;
                if self.vram_bank == 0 {
                    self.vram.lock().unwrap().tile_map_1[index]
                } else {
                    self.vram.lock().unwrap().tile_attrs_1[index]
                }
            },
            // Sprite data
            0xFE00..=0xFE9F if self.regs.can_access_oam() => self.vram.lock().unwrap().object_mem.read(loc - 0xFE00),
            // Registers
            0xFF40 => self.regs.read_lcd_control(),
            0xFF41 => self.regs.read_status(),
            0xFF42 => self.regs.scroll_y,
            0xFF43 => self.regs.scroll_x,
            0xFF44 => self.regs.read_lcdc_y(),
            0xFF45 => self.regs.ly_compare,
            0xFF47 => self.vram.lock().unwrap().palettes.read(0),
            0xFF48 => self.vram.lock().unwrap().palettes.read(1),
            0xFF49 => self.vram.lock().unwrap().palettes.read(2),
            0xFF4A => self.regs.window_y,
            0xFF4B => self.regs.window_x,
            0xFF4F => self.vram_bank | 0xFE,
            // Colour palettes
            //0xFF68 => self.colour_palettes.read_bg_index(),
            //0xFF69 => self.colour_palettes.read_bg(),
            //0xFF6A => self.colour_palettes.read_obj_index(),
            //0xFF6B => self.colour_palettes.read_obj(),
            _ => 0xFF
        }
    }

    fn write(&mut self, loc: u16, val: u8) {
        match loc {
            // Raw tile data
            0x8000..=0x97FF if self.regs.can_access_vram() => {
                let base = (loc - 0x8000) as usize + (self.vram_bank as usize * 0x1800);

                let mut vram = self.vram.lock().unwrap();

                if base % 2 == 0 {  // Lower bit
                    vram.tile_mem.set_pixel_lower_row(base, val);
                } else {            // Upper bit
                    vram.tile_mem.set_pixel_upper_row(base, val);
                }

                vram.map_cache_0_dirty = true;
                vram.map_cache_1_dirty = true;
            },
            // Background Map A
            0x9800..=0x9BFF if self.regs.can_access_vram() => {
                /*let base = (loc - 0x9800) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                if self.vram_bank == 0 {
                    self.tile_map_0.set_tile_texture(x, y, val);
                } else {
                    self.tile_map_0.set_tile_attribute(x, y, val);
                }*/
                let index = (loc - 0x9800) as usize;
                if self.vram_bank == 0 {
                    self.vram.lock().unwrap().tile_map_0[index] = val;
                } else {
                    self.vram.lock().unwrap().tile_attrs_0[index] = val;
                }

                self.vram.lock().unwrap().map_cache_0_dirty = true;
            },
            // Background Map B
            0x9C00..=0x9FFF if self.regs.can_access_vram() => {
                /*let base = (loc - 0x9C00) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                if self.vram_bank == 0 {
                    self.tile_map_1.set_tile_texture(x, y, val);
                } else {
                    self.tile_map_1.set_tile_attribute(x, y, val);
                }*/
                let index = (loc - 0x9C00) as usize;
                if self.vram_bank == 0 {
                    self.vram.lock().unwrap().tile_map_1[index] = val;
                } else {
                    self.vram.lock().unwrap().tile_attrs_1[index] = val;
                }
                
                self.vram.lock().unwrap().map_cache_1_dirty = true;
            },
            // Sprite data
            0xFE00..=0xFE9F if self.regs.can_access_oam() => self.vram.lock().unwrap().object_mem.write(loc - 0xFE00, val),
            0xFF40 => {
                if self.regs.write_lcd_control(val) {
                    self.cycle_count = 0;
                }
                self.vram.lock().unwrap().map_cache_0_dirty = true;
                self.vram.lock().unwrap().map_cache_1_dirty = true;
            },
            0xFF41 => self.regs.write_status(val),
            0xFF42 => self.regs.scroll_y = val,
            0xFF43 => self.regs.scroll_x = val,
            0xFF44 => self.regs.set_lcdc_y(0),
            0xFF45 => self.regs.ly_compare = val,
            0xFF47 => self.vram.lock().unwrap().palettes.write(0, val),
            0xFF48 => self.vram.lock().unwrap().palettes.write(1, val),
            0xFF49 => self.vram.lock().unwrap().palettes.write(2, val),
            0xFF4A => self.regs.window_y = val,
            0xFF4B => self.regs.window_x = val,
            0xFF4F => self.vram_bank = val & 1,
            // Colour palettes
            //0xFF68 => self.colour_palettes.write_bg_index(val),
            //0xFF69 => self.colour_palettes.write_bg(val),
            //0xFF6A => self.colour_palettes.write_obj_index(val),
            //0xFF6B => self.colour_palettes.write_obj(val),
            _ => {}//unreachable!()
        }
    }
}