mod palette;
mod shaders;
mod joypad;

use glium;
use glium::{Display, Surface};
use glium::glutin::EventsLoop;
use glium::texture::texture2d::Texture2d;

use mem::MemDevice;

use self::palette::{BWPalette, Palette};
use self::joypad::Joypad;

const BG_X: u16 = 256;
const BG_Y: u16 = 256;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    texcoord: [f32; 2],
}

implement_vertex!(Vertex, position, texcoord);

fn byte_to_float(byte: u16, scale: u16) -> f32 {
    let (byte_f, scale_f) = (byte as f32, scale as f32);
    let out_f = (byte_f * 2.0) / scale_f;
    out_f - 1.0
}

pub trait VideoDevice: MemDevice {
    fn render_frame(&mut self);
    fn read_inputs(&mut self);
}

pub struct GBVideo {
    // potentially add background, sprite, window objects?
    display_enable:     bool,
    window_offset:      usize,
    window_enable:      bool,
    bg_offset:          usize,
    bg_enable:          bool,
    tile_data_select:   bool,
    sprite_size:        bool,
    sprite_enable:      bool,

    lcd_status:         u8,
    scroll_y:           u8,
    scroll_x:           u8,
    lcdc_y:             u8,
    ly_compare:         u8,
    window_y:           u8,
    window_x:           u8,
    bg_palette:         BWPalette,
    obj_palette_0:      BWPalette,
    obj_palette_1:      BWPalette,

    // joypad inputs
    joypad:             Joypad,

    // raw tiles used for background & sprites
    raw_tile_mem:       Vec<u8>,
    // map for background & window
    tile_map_mem:       Vec<u8>,
    sprite_mem:         Vec<u8>,

    display:            Display,
    events_loop:        EventsLoop,
    program:            glium::Program,
}

impl MemDevice for GBVideo {
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
            0x8000...0x97FF =>  self.raw_tile_mem[(loc - 0x8000) as usize] = val,
            0x9800...0x9FFF =>  self.tile_map_mem[(loc - 0x9800) as usize] = val,
            0xFE00...0xFE9F =>  self.sprite_mem[(loc - 0xFE00) as usize] = val,

            0xFF00 =>           self.joypad.write(val),

            0xFF40 =>           self.lcd_control_write(val),
            0xFF41 =>           self.lcd_status = val,
            0xFF42 =>           self.scroll_y = val,
            0xFF43 =>           self.scroll_x = val,
            0xFF44 =>           self.lcdc_y = val,
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

impl VideoDevice for GBVideo {
    // Drawing for a single frame
    fn render_frame(&mut self) {
        let mut target = self.display.draw();
        target.clear_color(1.0, 1.0, 1.0, 1.0);

        // render background
        if self.bg_enable {
            for y in 0..32 {
                for x in 0..32 {
                    // get tile number from background map
                    let offset = (x + (y*32)) as usize;
                    let tile = self.tile_map_mem[self.bg_offset + offset];
                    // get tile location from number & addressing mode
                    let tile_loc = if self.tile_data_select {
                        (tile as isize) * 16
                    } else {
                        0x1000 + ((tile as i8) as isize * 16)
                    } as usize;

                    let tex = {
                        let raw_tex = &self.raw_tile_mem[tile_loc..(tile_loc + 16)];
                        self.bg_palette.make_texture(&raw_tex, &self.display)
                    };
                    self.draw_square(&mut target, x*8, y*8, &tex);
                };
            };
        }

        // render sprites
        if self.sprite_enable {
            /*for s in (0..self.sprite_mem.size()).step_by(4) {
                let y_pos = self.sprite_mem[s] - 16;
                let x_pos = self.sprite_mem[s+1] - 8;
                let
            }*/
        }

        // render window
        if self.window_enable {

        }

        target.finish().unwrap();
    }

    // Read inputs and store
    fn read_inputs(&mut self) {
        use glium::glutin::{Event, WindowEvent, ElementState, VirtualKeyCode};

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
                    _ => {},
                },
                _ => {},
            }
        });
    }
}

// Control functions
impl GBVideo {
    pub fn new() -> GBVideo {
        let events_loop = glium::glutin::EventsLoop::new();

        // create display
        let window = glium::glutin::WindowBuilder::new();
        let context = glium::glutin::ContextBuilder::new();
        let display = glium::Display::new(window, context, &events_loop).unwrap();

        // compile program
        let program = glium::Program::from_source(&display,
                                                  shaders::VERTEX_SRC,
                                                  shaders::FRAGMENT_SRC,
                                                  None).unwrap();

        GBVideo {
            display_enable:     true,
            window_offset:      0x0,
            window_enable:      false,
            tile_data_select:   true,
            bg_offset:          0x0,
            sprite_size:        false,
            sprite_enable:      false,
            bg_enable:          true,

            lcd_status:         0, // TODO: check
            scroll_y:           0,
            scroll_x:           0,
            lcdc_y:             0,
            ly_compare:         0,
            window_y:           0,
            window_x:           0,
            bg_palette:         BWPalette::new(),
            obj_palette_0:      BWPalette::new(),
            obj_palette_1:      BWPalette::new(),

            joypad:             Joypad::new(),

            raw_tile_mem:       vec![0; 0x1800],
            tile_map_mem:       vec![0; 0x800],
            sprite_mem:         vec![0; 0x100],

            display:            display,
            events_loop:        events_loop,
            program:            program,
        }
    }

    fn lcd_control_write(&mut self, val: u8) {
        println!("LCD write: {:b}", val);
        self.display_enable     = val & 0x80 == 0x80;
        self.window_offset      = if val & 0x40 == 0x40 {0x400} else {0x0};
        self.window_enable      = val & 0x20 == 0x20;
        self.tile_data_select   = val & 0x10 == 0x10;
        self.bg_offset          = if val & 0x8 == 0x8   {0x400} else {0x0};
        self.sprite_size        = val & 0x4 == 0x4;
        self.sprite_enable      = val & 0x2 == 0x2;
        self.bg_enable          = val & 0x1 == 0x1;
    }

    fn lcd_control_read(&self) -> u8 {
        let val_7 = if self.display_enable          {0x80} else {0};
        let val_6 = if self.window_offset == 0x400  {0x40} else {0};
        let val_5 = if self.window_enable           {0x20} else {0};
        let val_4 = if self.tile_data_select        {0x10} else {0};
        let val_3 = if self.bg_offset == 0x400      {0x8} else {0};
        let val_2 = if self.sprite_size             {0x4} else {0};
        let val_1 = if self.sprite_enable           {0x2} else {0};
        let val_0 = if self.bg_enable               {0x1} else {0};
        val_7 | val_6 | val_5 | val_4 | val_3 | val_2 | val_1 | val_0
    }
}


// Internal graphics functions
impl GBVideo {

    // draw 8x8 textured square
    fn draw_square(&mut self, target: &mut glium::Frame, x: u16, y: u16, texture: &Texture2d) {
        use glium::index::{NoIndices, PrimitiveType};

        let (x_a, y_a) = (byte_to_float(x, BG_X), byte_to_float(y, BG_Y));
        let (x_b, y_b) = (byte_to_float(x + 8, BG_X), byte_to_float(y + 8, BG_Y));
        //println!("{},{}", x_a,x_b);

        let uniforms = uniform!{tex: texture};

        let tile = vec![
            Vertex { position: [x_a, y_a], texcoord: [0.0, 0.0] },
            Vertex { position: [x_b, y_a], texcoord: [1.0, 0.0] },
            Vertex { position: [x_a, y_b], texcoord: [0.0, 1.0] },
            Vertex { position: [x_b, y_b], texcoord: [1.0, 1.0] }
        ];
        let vertex_buffer = glium::VertexBuffer::new(&self.display, &tile).unwrap();

        target.draw(&vertex_buffer, NoIndices(PrimitiveType::TriangleStrip),
                    &self.program, &uniforms, &Default::default()).unwrap();
    }
}
