mod mem;
mod joypad;
mod renderer;
mod shaders;

// Video mode constants
mod constants {
    // Mode cycle counts
    pub const H_CYCLES: u32     = 456;
    pub const MODE_1: u32       = 154 * H_CYCLES;
    pub const MODE_2: u32       = 80;
    pub const MODE_3: u32       = MODE_2 + 172;
    pub const FRAME_CYCLE: u32  = 144 * H_CYCLES;
}

// Modes
#[derive(PartialEq, Debug, Clone)]
pub enum Mode {
    _0 = 0, // H-blank
    _1 = 1, // V-blank
    _2 = 2, // Reading
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

use winit::{
    EventsLoop,
    Event,
    WindowEvent,
    ElementState,
    ControlFlow,
    VirtualKeyCode
};

use crate::mem::{InterruptFlags, MemDevice};

use self::joypad::{Joypad, Buttons, Directions};
use self::mem::VideoMem;
use self::renderer::Renderer;

pub struct VideoDevice {
    mem:                VideoMem,
    // joypad inputs
    joypad:             Joypad,

    renderer:           Renderer,
    events_loop:        EventsLoop
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

impl VideoDevice {
    // Drawing for a single frame
    pub fn render_frame(&mut self) {
        self.renderer.render(&mut self.mem);
    }

    // Read inputs and store
    pub fn read_inputs(&mut self) {
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
                            Some(VirtualKeyCode::X)         => joypad.buttons.set(Buttons::A, pressed),
                            Some(VirtualKeyCode::Z)         => joypad.buttons.set(Buttons::B, pressed),
                            Some(VirtualKeyCode::Space)     => joypad.buttons.set(Buttons::SELECT, pressed),
                            Some(VirtualKeyCode::Return)    => joypad.buttons.set(Buttons::START, pressed),
                            Some(VirtualKeyCode::Up)        => joypad.directions.set(Directions::UP, pressed),
                            Some(VirtualKeyCode::Down)      => joypad.directions.set(Directions::DOWN, pressed),
                            Some(VirtualKeyCode::Left)      => joypad.directions.set(Directions::LEFT, pressed),
                            Some(VirtualKeyCode::Right)     => joypad.directions.set(Directions::RIGHT, pressed),
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
    }

    // Set the current video mode based on the cycle count.
    // May trigger an interrupt.
    pub fn video_mode(&mut self, cycle_count: &mut u32) -> (bool, InterruptFlags) {
        use self::constants::*;

        let frame_cycle = *cycle_count % H_CYCLES;
        let int = match self.mem.lcd_status.read_mode() {
            Mode::_2 => if frame_cycle >= MODE_2 {
                self.update_mode(Mode::_3)
            } else { InterruptFlags::default() },
            Mode::_3 => if frame_cycle >= MODE_3 {
                self.update_mode(Mode::_0)
            } else { InterruptFlags::default() },
            Mode::_0 => if *cycle_count >= FRAME_CYCLE {
                self.mem.inc_lcdc_y();
                self.update_mode(Mode::_1) | InterruptFlags::V_BLANK
            } else if frame_cycle < MODE_3 {
                self.mem.inc_lcdc_y();
                self.update_mode(Mode::_2)
            } else { InterruptFlags::default() },
            Mode::_1 => if *cycle_count >= MODE_1 {
                self.mem.set_lcdc_y(0);
                *cycle_count -= MODE_1;
                self.update_mode(Mode::_2)
            } else {
                let new_ly = (*cycle_count / H_CYCLES) as u8;
                self.mem.set_lcdc_y(new_ly);
                InterruptFlags::default()
            },
        };

        (if int.contains(InterruptFlags::V_BLANK) {
            false
        } else {
            true
        }, int)
    }

    // Update status reg, Trigger LCDC Status interrupt if necessary
    fn update_mode(&mut self, mode: Mode) -> InterruptFlags {
        use mem::LCDStatusFlags;

        self.mem.lcd_status.write_mode(mode.clone());
        let stat_flags = self.mem.lcd_status.read_flags();

        // Trigger STAT interrupt
        if !stat_flags.is_empty() {
            // LY Coincidence interrupt
            if stat_flags.contains(LCDStatusFlags::COINCEDENCE_INT) {
                if stat_flags.contains(LCDStatusFlags::COINCEDENCE_FLAG) == self.mem.compare_ly_equal() {
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
}

// Control functions
impl VideoDevice {
    pub fn new() -> Self {
        let events_loop = EventsLoop::new();
        let renderer = Renderer::new(&events_loop);
        let mem = VideoMem::new(&renderer.get_device());

        VideoDevice {
            mem:            mem,
            // joypad inputs
            joypad:         Joypad::new(),

            renderer:       renderer,
            events_loop:    events_loop
        }
    }

}