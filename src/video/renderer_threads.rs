// Pixel renderer. Makes a texture of format R8G8B8A8Unorm
use super::vram::VRAM;
use super::regs::VideoRegs;

use std::sync::{
    Arc,
    Mutex
};

use crossbeam_channel::{
    unbounded,
    Sender,
    Receiver
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
    sender:     Sender<RendererMessage>,
    receiver:   Receiver<()>,
}

impl Renderer {
    pub fn new(mem: Arc<Mutex<VRAM>>) -> Self {
        let (send_msg, recv_msg) = unbounded();
        let (send_reply, recv_reply) = unbounded();

        std::thread::spawn(move || {
            use RendererMessage::*;
            let mut target = None;

            while let Ok(msg) = recv_msg.recv() {
                match msg {
                    StartFrame(data) => {
                        target = Some(data);
                    },
                    DrawLineGB(regs) => {
                        let mut mem = mem.lock().unwrap();
                        let mut t = target.as_ref().unwrap().lock().unwrap();
                        send_reply.send(()).unwrap();
                        mem.draw_line_gb(&mut t, &regs);
                    },
                    DrawLineCGB(regs) => {
                        let mut mem = mem.lock().unwrap();
                        let mut t = target.as_ref().unwrap().lock().unwrap();
                        send_reply.send(()).unwrap();
                        mem.draw_line_cgb(&mut t, &regs);
                    }
                }
            }
        });

        Renderer {
            sender:     send_msg,
            receiver:   recv_reply,
        }
    }

    pub fn start_frame(&mut self, target: RenderTarget) {
        self.sender
            .send(RendererMessage::StartFrame(target))
            .expect("Couldn't send start frame message!");
    }

    pub fn draw_line_gb(&mut self, regs: VideoRegs) {
        self.sender
            .send(RendererMessage::DrawLineGB(regs))
            .expect("Couldn't send draw line message!");

        self.receiver
            .recv()
            .expect("GB");
    }

    pub fn draw_line_cgb(&mut self, regs: VideoRegs) {
        self.sender
            .send(RendererMessage::DrawLineCGB(regs))
            .expect("Couldn't send draw line message!");

        self.receiver
            .recv()
            .expect("CGB");
    }
}