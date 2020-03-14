#[macro_use]
mod utils;

mod cpu;
mod mem;
mod video;
mod timer;
mod audio;
mod interrupt;
mod joypad;

#[cfg(feature = "debug")]
pub mod debug;

pub use video::{
    UserPalette
};

use joypad::{
    Buttons,
    Directions
};

use std::sync::{
    Arc,
    Mutex
};

use crossbeam_channel::{
    unbounded, Receiver
};

use cpu::CPU;
use audio::{
    AudioCommand,
    AudioHandler
};
use mem::MemBus;

pub const FRAME_SIZE_BYTES: usize = 160 * 144 * 4;

pub enum Button {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Start,
    Select
}

pub struct RustBoy {
    cpu:            CPU,

    frame:          Arc<Mutex<[u8; FRAME_SIZE_BYTES]>>,
}

impl RustBoy {
    pub fn new(cart_name: &str, save_file_name: &str, palette: UserPalette) -> Box<Self> {
        //let ad = AudioDevice::new(audio_send, audio_reply_recv);
        let mem = MemBus::new(cart_name, save_file_name, palette);

        let cpu = CPU::new(mem);

        //let audio_packet = Arc::new(Mutex::new(vec![0.0; sample_rate / 60]));

        //start_audio_handler_thread(audio_recv, audio_reply, sample_rate, audio_packet.clone());

        Box::new(RustBoy {
            cpu:            cpu,

            frame:          Arc::new(Mutex::new([255; FRAME_SIZE_BYTES])),
        })
    }

    pub fn enable_audio(&mut self, sample_rate: usize) -> RustBoyAudioHandle {
        let (audio_send, audio_recv) = unbounded();

        self.cpu.enable_audio(audio_send);

        RustBoyAudioHandle {
            handler: AudioHandler::new(audio_recv, sample_rate)
        }
    }

    // Call every 1/60 seconds.
    pub fn frame(&mut self, frame: &mut [u8]) {
        self.cpu.frame_update(self.frame.clone());    // Draw video and read inputs

        while self.cpu.step() {}    // Execute up to v-blanking

        let new_frame = self.frame.lock().unwrap();
        frame.copy_from_slice(&(*new_frame));
    }

    pub fn set_button(&mut self, button: Button, val: bool) {
        use Button::*;

        match button {
            Up      => self.cpu.set_direction(Directions::UP, val),
            Down    => self.cpu.set_direction(Directions::DOWN, val),
            Left    => self.cpu.set_direction(Directions::LEFT, val),
            Right   => self.cpu.set_direction(Directions::RIGHT, val),
            A       => self.cpu.set_button(Buttons::A, val),
            B       => self.cpu.set_button(Buttons::B, val),
            Start   => self.cpu.set_button(Buttons::START, val),
            Select  => self.cpu.set_button(Buttons::SELECT, val),
        }
    }
}

pub struct RustBoyAudioHandle {
    handler: AudioHandler
}

impl RustBoyAudioHandle {
    pub fn get_audio_packet(&mut self, packet: &mut [f32]) {
        self.handler.fill_buffer(packet);
    }
}

#[cfg(feature = "debug")]
impl RustBoy {
    pub fn step(&mut self) -> bool {
        self.cpu.step()
    }

    pub fn get_state(&self) -> debug::CPUState {
        self.cpu.get_state()
    }

    pub fn get_instr(&self) -> [u8; 3] {
        self.cpu.get_instr()
    }

    pub fn get_mem_at(&self, loc: u16) -> u8 {
        self.cpu.get_mem_at(loc)
    }
}