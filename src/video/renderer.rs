// Pixel renderer. Makes a texture of format R8G8B8A8Unorm
use super::mem::VideoMem;

use std::sync::{
    Arc,
    mpsc::{
        channel,
        Sender,
        Receiver
    },
    Mutex
};

pub type RenderTarget = Arc<Mutex<[u8]>>;

// Messages to send to the render thread.
enum RendererMessage {
    StartFrame(RenderTarget),   // Begin frame, and target the provided byte array.
    EndFrame,                   // End frame.
    DrawLine
}

// Messages to receive from the render thread.
#[derive(PartialEq)]
enum RendererReply {
    StartedFrame,
    DrawingLine,
    FinishedFrame
}

// Renderer for video that spawns a thread to render on.
pub struct Renderer {
    sender: Sender<RendererMessage>,
    receiver: Receiver<RendererReply>
}

impl Renderer {
    pub fn new(mem: Arc<Mutex<VideoMem>>) -> Self {
        let (send_msg, recv_msg) = channel::<RendererMessage>();
        let (send_reply, recv_reply) = channel::<RendererReply>();

        std::thread::spawn(move || {
            use RendererMessage::*;
            let mut target = None;

            while let Ok(msg) = recv_msg.recv() {
                match msg {
                    StartFrame(data) => {
                        target = Some(data);
                        //send_reply.send(RendererReply::StartedFrame).unwrap();
                    },
                    DrawLine => {
                        let mut mem = mem.lock().unwrap();
                        let mut t = target.as_ref().unwrap().lock().unwrap();
                        //send_reply.send(RendererReply::DrawingLine).unwrap();
                        mem.draw_line_gb(&mut t);
                    },
                    EndFrame => {
                        target = None;
                        //send_reply.send(RendererReply::FinishedFrame).unwrap();
                    }
                }
            }
        });

        Renderer {
            sender: send_msg,
            receiver: recv_reply
        }
    }

    pub fn start_frame(&mut self, target: RenderTarget) {
        self.sender
            .send(RendererMessage::StartFrame(target))
            .expect("Couldn't send start frame message!");

        /*let msg = self.receiver.recv().unwrap();
        if msg != RendererReply::StartedFrame {
            panic!("Wrong reply received for start frame!");
        }*/
    }

    pub fn draw_line(&mut self) {
        self.sender
            .send(RendererMessage::DrawLine)
            .expect("Couldn't send draw line message!");

        /*let msg = self.receiver.recv().unwrap();
        if msg != RendererReply::DrawingLine {
            panic!("Wrong reply received for draw line!");
        }*/
    }

    pub fn end_frame(&mut self) {
        self.sender
            .send(RendererMessage::EndFrame)
            .expect("Couldn't send end frame message!");

        /*let msg = self.receiver.recv().unwrap();
        if msg != RendererReply::FinishedFrame {
            panic!("Wrong reply received for end frame!");
        }*/
    }
}