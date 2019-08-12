mod mem;
mod joypad;
mod renderer;
mod shaders;

use winit::{
    EventsLoop,
    Event,
    WindowEvent,
    KeyboardInput,
    ElementState,
    ControlFlow,
    VirtualKeyCode
};

use crate::mem::MemDevice;

use self::joypad::Joypad;
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
            0x8000...0x97FF =>  self.raw_tile_mem[(loc - 0x8000) as usize],
            0x9800...0x9FFF =>  self.tile_map_mem[(loc - 0x9800) as usize],
            0xFE00...0xFE9F =>  self.sprite_mem[(loc - 0xFE00) as usize],

            0xFF00 =>           self.joypad.read(),

            0xFF40 =>           self.lcd_control_read(),
            0xFF41 =>           self.lcd_status,
            0xFF42 =>           self.scroll_y,
            0xFF43 =>           self.scroll_x,
            0xFF44 =>           self.lcdc_y,
            0xFF45 =>           self.ly_compare,
            0xFF47 =>           self.bg_palette.read(),
            0xFF48 =>           self.obj_palette_0.read(),
            0xFF49 =>           self.obj_palette_1.read(),
            0xFF4A =>           self.window_y,
            0xFF4B =>           self.window_x,
            _ => 0,
        }
    }

    fn write(&mut self, loc: u16, val: u8) {
        match loc {
            0x8000...0x97FF =>  self.write_raw_tile(loc, val),
            0x9800...0x9FFF =>  self.tile_map_mem[(loc - 0x9800) as usize] = val,
            0xFE00...0xFE9F =>  self.sprite_mem[(loc - 0xFE00) as usize] = val,

            0xFF00 =>           self.joypad.write(val),

            0xFF40 =>           self.lcd_control_write(val),
            0xFF41 =>           self.lcd_status = val,
            0xFF42 =>           self.scroll_y = val,
            0xFF43 =>           self.scroll_x = val,
            0xFF44 =>           self.lcdc_y = 0,
            0xFF45 =>           self.ly_compare = val,
            0xFF47 =>           self.bg_palette.write(val),
            0xFF48 =>           self.obj_palette_0.write(val),
            0xFF49 =>           self.obj_palette_1.write(val),
            0xFF4A =>           self.window_y = val,
            0xFF4B =>           self.window_x = val,
            _ => return,
        }
    }
}

impl VideoDevice {
    // Drawing for a single frame
    pub fn render_frame(&mut self) {
        self.renderer.render(self.video_mem);
    }

    // Read inputs and store
    pub fn read_inputs(&mut self) {
        let joypad = &mut self.joypad;

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
                            Some(VirtualKeyCode::Z)         => joypad.a = pressed,
                            Some(VirtualKeyCode::X)         => joypad.b = pressed,
                            Some(VirtualKeyCode::Space)     => joypad.select = pressed,
                            Some(VirtualKeyCode::Return)    => joypad.start = pressed,
                            Some(VirtualKeyCode::Up)        => joypad.up = pressed,
                            Some(VirtualKeyCode::Down)      => joypad.down = pressed,
                            Some(VirtualKeyCode::Left)      => joypad.left = pressed,
                            Some(VirtualKeyCode::Right)     => joypad.right = pressed,
                            _ => {},
                        }
                    },
                    WindowEvent::Resized(_) => {
                        self.renderer.create_swapchain();
                    },
                    _ => {}
                },
                _ => {},
            }
        });
    }

    pub fn inc_lcdc_y(&mut self) {
        self.lcdc_y += 1;
    }

    pub fn set_lcdc_y(&mut self, val: u8) {
        self.lcdc_y = val;
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