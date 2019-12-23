use winit::{
    EventsLoop,
    Window
};

use super::mem::VideoMem;

#[derive(Default, Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub data: u32
}

pub trait Renderer {
    fn frame_start(&mut self, video_mem: &mut VideoMem);
    fn frame_end(&mut self);
    fn draw_line(&mut self, y: u8, video_mem: &mut VideoMem, cgb_mode: bool);

    fn on_resize(&mut self);
}

pub enum WindowType<'a> {
    Winit(&'a EventsLoop),
    IOS {
        ui_view:    *const std::ffi::c_void,
        window:     Window
    }
}