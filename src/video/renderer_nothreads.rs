// Pixel renderer. Makes a texture of format R8G8B8A8Unorm
use super::vram::VRAM;
use super::regs::VideoRegs;

use std::sync::{
    Arc,
    Mutex
};

pub type RenderTarget = Arc<Mutex<[u8]>>;

// Messages to send to the render thread.
enum RendererMessage {
    StartFrame(RenderTarget),   // Begin frame, and target the provided byte array.
    DrawLineGB(VideoRegs),
    DrawLineCGB(VideoRegs)
}

// Renderer for video that spawns a thread to render on.
pub struct Renderer {
    mem:    Arc<Mutex<VRAM>>,
    target: Option<RenderTarget>
}

impl Renderer {
    pub fn new(mem: Arc<Mutex<VRAM>>) -> Self {
        Renderer {
            mem:        mem,
            target:     None,
        }
    }

    pub fn start_frame(&mut self, target: RenderTarget) {
        self.target = Some(target);
    }

    pub fn draw_line_gb(&mut self, regs: VideoRegs) {
        let mut mem = self.mem.lock().unwrap();
        let mut t = self.target.as_ref().unwrap().lock().unwrap();
        mem.draw_line_gb(&mut t, &regs);
    }

    pub fn draw_line_cgb(&mut self, regs: VideoRegs) {
        let mut mem = self.mem.lock().unwrap();
        let mut t = self.target.as_ref().unwrap().lock().unwrap();
        mem.draw_line_cgb(&mut t, &regs);
    }
}