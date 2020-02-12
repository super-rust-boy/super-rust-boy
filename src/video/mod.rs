mod mem;
mod types;
mod renderer;

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

use self::mem::VideoMem;
use self::sgbpalettes::SGBPalette;

pub use self::types::{
    Colour,
    PaletteColours
};

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
    mem:        Arc<Mutex<VideoMem>>,

    renderer:   Renderer,

    cgb_mode:   bool
}

impl VideoDevice {
    pub fn new(palette: SGBPalette, cgb_mode: bool) -> Self {
        let mem = Arc::new(Mutex::new(VideoMem::new(palette, cgb_mode)));

        // Spin off video thread.
        let renderer = Renderer::new(mem.clone());

        VideoDevice {
            mem:            mem,

            renderer:       renderer,

            cgb_mode:       cgb_mode
        }
    }

    // Drawing for a single frame.
    pub fn start_frame(&mut self, render_target: RenderTarget) {
        self.renderer.start_frame(render_target);
    }

    // Query to see if the video device is in H-Blank.
    pub fn is_in_hblank(&self) -> bool {
        self.mem.lock().unwrap().lcd_status.read_mode() == Mode::_0
    }

    // Set the current video mode based on the cycle count.
    // May trigger an interrupt.
    // Returns true if transitioned to V-Blank.
    pub fn video_mode(&mut self, cycles: u32) -> (bool, InterruptFlags) {
        use self::constants::*;
        //let mut mem = self.mem.lock().unwrap();
        self.mem.lock().unwrap().inc_cycle_count(cycles);

        if self.mem.lock().unwrap().display_enabled() {
            // First, calculate how many cycles into the horizontal line we are.
            let line_cycle = self.mem.lock().unwrap().get_cycle_count() % H_CYCLES;
            let mode = self.mem.lock().unwrap().lcd_status.read_mode();

            let int = match mode {
                Mode::_2 if line_cycle >= MODE_2 => self.update_mode(Mode::_3),
                Mode::_3 if line_cycle >= MODE_3 => self.update_mode(Mode::_0),
                Mode::_0 if self.mem.lock().unwrap().get_cycle_count() >= FRAME_CYCLE => {
                    self.mem.lock().unwrap().inc_lcdc_y();
                    self.update_mode(Mode::_1) | InterruptFlags::V_BLANK
                },
                Mode::_0 if line_cycle < MODE_3 => {
                    self.mem.lock().unwrap().inc_lcdc_y();
                    self.update_mode(Mode::_2)
                },
                Mode::_1 => if self.mem.lock().unwrap().get_cycle_count() >= MODE_1 {
                    //self.renderer.frame_start(&mut self.mem);
                    self.mem.lock().unwrap().set_lcdc_y(0);
                    self.mem.lock().unwrap().frame_cycle_reset();
                    self.update_mode(Mode::_2)
                } else {
                    let new_ly = (self.mem.lock().unwrap().get_cycle_count() / H_CYCLES) as u8;
                    self.mem.lock().unwrap().set_lcdc_y(new_ly);
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
            let keep_cycling = if self.mem.lock().unwrap().get_cycle_count() > MODE_1 {
                self.mem.lock().unwrap().frame_cycle_reset();
                true
            } else {
                false
            };
            (keep_cycling, InterruptFlags::default())
        }
    }

    // Update status reg, Trigger LCDC Status interrupt if necessary
    fn update_mode(&mut self, mode: Mode) -> InterruptFlags {
        use mem::LCDStatusFlags;
        //let mem = self.mem.lock().unwrap();

        self.mem.lock().unwrap().lcd_status.write_mode(mode);
        let stat_flags = self.mem.lock().unwrap().lcd_status.read_flags();

        if mode == Mode::_0 {
            self.renderer.draw_line();
        }

        // Trigger STAT interrupt
        if !stat_flags.is_empty() {
            // LY Coincidence interrupt
            if stat_flags.contains(LCDStatusFlags::COINCEDENCE_INT) {
                if self.mem.lock().unwrap().compare_ly_equal() {
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

impl MemDevice for VideoDevice {
    fn read(&self, loc: u16) -> u8 {
        self.mem.lock().unwrap().read(loc)
    }

    fn write(&mut self, loc: u16, val: u8) {
        self.mem.lock().unwrap().write(loc, val);
    }
}