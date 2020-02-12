use super::mem::VideoMem;

use std::sync::Arc;
use std::cell::RefCell;

#[derive(Default, Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub data: u32
}

#[derive(Clone, Copy)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8
}

impl Colour {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Colour {
            r: r,
            g: g,
            b: b
        }
    }

    pub fn zero() -> Colour {
        Colour {
            r: 255,
            g: 255,
            b: 255
        }
    }
}

pub type PaletteColours = [Colour; 4];

/*pub struct Renderer<'a> {
    mem: Arc<RefCell<VideoMem>>,

    frame: Option<[u8]>
}

impl<'a> Renderer<'a> {
    // Begin the process of rendering a frame.
    fn frame_start(&mut self, frame: [u8]) {
        self.frame = Some(frame);
    }

    // End the process of rendering a frame.
    fn frame_end(&mut self) -> [u8] {
        let frame = std::mem::replace(self.frame, None);
        frame.expect("Call frame_start before frame_end")
    }

    // Draw a line, based on the current state.
    fn draw_line(&mut self, cgb_mode: bool) {
        let mem = self.mem.borrow_mut();


    }

    fn copy_image(&self, out: &[u8]) {

    }
}*/