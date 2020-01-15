use winit::{
    EventsLoop,
    Window
};

use std::ffi::c_void;

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

    fn transfer_image(&mut self, buffer: &mut [u32]);
}

pub enum WindowType<'a> {
    Winit(&'a EventsLoop),
    IOS {
        ui_view:    *const c_void,
        window:     Window
    },
    MacOS {
        ns_view:    *const c_void,
        window:     Window
    }
}