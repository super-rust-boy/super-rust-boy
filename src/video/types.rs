use winit::EventsLoop;

#[derive(Default, Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub data: u32
}

pub enum WindowType<'a> {
    Winit(&'a EventsLoop)
}