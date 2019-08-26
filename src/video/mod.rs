mod mem;
mod joypad;
mod renderer;
mod shaders;
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

use winit::{
    EventsLoop,
    Event,
    WindowEvent,
    ElementState,
    VirtualKeyCode
};

use cgmath::Matrix4;

use crate::interrupt::InterruptFlags;
use crate::mem::MemDevice;

use self::joypad::{Joypad, Buttons, Directions};
use self::mem::VideoMem;
use self::renderer::Renderer;
use self::sgbpalettes::SGBPalette;

pub use sgbpalettes::UserPalette;

pub type PaletteColours = Matrix4<f32>;

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
    mem:                VideoMem,
    // joypad inputs
    joypad:             Joypad,

    renderer:           Renderer,
    events_loop:        EventsLoop,

    cgb_mode:           bool
}

impl VideoDevice {
    pub fn new(palette: SGBPalette, cgb_mode: bool) -> Self {
        let events_loop = EventsLoop::new();
        let mut renderer = Renderer::new(&events_loop);
        let mut mem = VideoMem::new(&renderer.get_device(), palette, cgb_mode);

        renderer.frame_start(&mut mem);

        VideoDevice {
            mem:            mem,
            // joypad inputs
            joypad:         Joypad::new(),

            renderer:       renderer,
            events_loop:    events_loop,

            cgb_mode:       cgb_mode
        }
    }

    // Drawing for a single frame
    pub fn frame(&mut self) {
        self.renderer.frame_end();
    }

    // Query to see if the video device is in H-Blank.
    pub fn is_in_hblank(&self) -> bool {
        self.mem.lcd_status.read_mode() == Mode::_0
    }

    // Read inputs and store, return true if joypad interrupt is triggered.
    pub fn read_inputs(&mut self) -> bool {
        let joypad = &mut self.joypad;
        let renderer = &mut self.renderer;

        self.events_loop.poll_events(|e| {
            match e {
                Event::WindowEvent {
                    window_id: _,
                    event: w,
                } => match w {
                    WindowEvent::CloseRequested => {
                        ::std::process::exit(0);
                    },
                    WindowEvent::KeyboardInput {
                        device_id: _,
                        input: k,
                    } => {
                        let pressed = match k.state {
                            ElementState::Pressed => true,
                            ElementState::Released => false,
                        };
                        match k.virtual_keycode {
                            Some(VirtualKeyCode::X)         => joypad.set_button(Buttons::A, pressed),
                            Some(VirtualKeyCode::Z)         => joypad.set_button(Buttons::B, pressed),
                            Some(VirtualKeyCode::Space)     => joypad.set_button(Buttons::SELECT, pressed),
                            Some(VirtualKeyCode::Return)    => joypad.set_button(Buttons::START, pressed),
                            Some(VirtualKeyCode::Up)        => joypad.set_direction(Directions::UP, pressed),
                            Some(VirtualKeyCode::Down)      => joypad.set_direction(Directions::DOWN, pressed),
                            Some(VirtualKeyCode::Left)      => joypad.set_direction(Directions::LEFT, pressed),
                            Some(VirtualKeyCode::Right)     => joypad.set_direction(Directions::RIGHT, pressed),
                            _ => {},
                        }
                    },
                    WindowEvent::Resized(_) => {
                        renderer.create_swapchain();
                    },
                    _ => {}
                },
                _ => {},
            }
        });

        joypad.check_interrupt()
    }

    // Set the current video mode based on the cycle count.
    // May trigger an interrupt.
    // Returns true if transitioned to V-Blank.
    pub fn video_mode(&mut self, cycles: u32) -> (bool, InterruptFlags) {
        use self::constants::*;
        self.mem.inc_cycle_count(cycles);

        if self.mem.display_enabled() {
            // First, calculate how many cycles into the horizontal line we are.
            let line_cycle = self.mem.get_cycle_count() % H_CYCLES;

            let int = match self.mem.lcd_status.read_mode() {
                Mode::_2 if line_cycle >= MODE_2 => self.update_mode(Mode::_3),
                Mode::_3 if line_cycle >= MODE_3 => self.update_mode(Mode::_0),
                Mode::_0 if self.mem.get_cycle_count() >= FRAME_CYCLE => {
                    self.mem.inc_lcdc_y();
                    self.update_mode(Mode::_1) | InterruptFlags::V_BLANK
                },
                Mode::_0 if line_cycle < MODE_3 => {
                    self.mem.inc_lcdc_y();
                    self.update_mode(Mode::_2)
                },
                Mode::_1 => if self.mem.get_cycle_count() >= MODE_1 {
                    self.renderer.frame_start(&mut self.mem);
                    self.mem.set_lcdc_y(0);
                    self.mem.frame_cycle_reset();
                    self.update_mode(Mode::_2)
                } else {
                    let new_ly = (self.mem.get_cycle_count() / H_CYCLES) as u8;
                    self.mem.set_lcdc_y(new_ly);
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
            let keep_cycling = if self.mem.get_cycle_count() > MODE_1 {
                self.mem.frame_cycle_reset();
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

        self.mem.lcd_status.write_mode(mode);
        let stat_flags = self.mem.lcd_status.read_flags();

        if mode == Mode::_0 {
            self.draw_line();
        }

        // Trigger STAT interrupt
        if !stat_flags.is_empty() {
            // LY Coincidence interrupt
            if stat_flags.contains(LCDStatusFlags::COINCEDENCE_INT) {
                if self.mem.compare_ly_equal() {
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
    fn draw_line(&mut self) {
        self.renderer.draw_line(self.mem.get_lcd_y(), &mut self.mem, self.cgb_mode);
    }
}

impl MemDevice for VideoDevice {
    fn read(&self, loc: u16) -> u8 {
        match loc {
            0xFF00 =>   self.joypad.read(),
            _ =>        self.mem.read(loc)
        }
    }

    fn write(&mut self, loc: u16, val: u8) {
        match loc {
            0xFF00 =>   self.joypad.write(val),
            _ =>        self.mem.write(loc, val)
        }
    }
}