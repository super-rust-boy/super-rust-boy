// Pixel renderer. Makes a texture of format R8G8B8A8Unorm
use super::mem::VideoMem;

use std::sync::{
    Arc,
    mpsc::{
        channel,
        Sender,
    },
    Mutex
};

pub type RenderTarget = Arc<Mutex<[u8]>>;

enum RendererMessage {
    StartFrame(RenderTarget),   // Begin frame, and target the provided byte array.
    EndFrame,                   // End frame.
    DrawLine
}

// Renderer for video that spawns a thread to render on.
pub struct Renderer {
    sender: Sender<RendererMessage>
}

impl Renderer {
    pub fn new(mem: Arc<Mutex<VideoMem>>) -> Self {
        let (send, recv) = channel::<RendererMessage>();

        std::thread::spawn(move || {
            use RendererMessage::*;
            let mut target = None;

            while let Ok(msg) = recv.recv() {
                match msg {
                    StartFrame(data) => {
                        target = Some(data);
                    },
                    DrawLine => {
                        let mut mem = mem.lock().unwrap();
                        let mut t = target.as_ref().unwrap().lock().unwrap();
                        mem.draw_line_gb(&mut t);
                    },
                    EndFrame => {
                        target = None;
                            //.expect("End frame was called before start frame!");
                        //callback(renderer.finish());
                    }
                }
            }
        });

        Renderer {
            sender: send
        }
    }

    pub fn start_frame(&mut self, target: RenderTarget) {
        self.sender
            .send(RendererMessage::StartFrame(target))
            .expect("Couldn't send start frame message!");
    }

    pub fn draw_line(&mut self) {
        self.sender
            .send(RendererMessage::DrawLine)
            .expect("Couldn't send draw line message!");
    }

    pub fn end_frame(&mut self) {
        self.sender
            .send(RendererMessage::EndFrame)
            .expect("Couldn't send end frame message!");
    }
}