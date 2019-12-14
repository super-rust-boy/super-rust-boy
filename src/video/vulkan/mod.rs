mod renderer;
mod shaders;

pub use renderer::Renderer;

use winit::EventsLoop;

pub enum WindowType<'a> {
    Winit(&'a EventsLoop)
}