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
    bounded, Receiver
};

use cpu::CPU;
use audio::{
    AudioCommand,
    AudioDevice,
    start_audio_handler_thread
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
    cpu:        CPU,
    audio_recv: Option<Receiver<AudioCommand>>,

    frame:      Arc<Mutex<[u8; FRAME_SIZE_BYTES]>>
}

impl RustBoy {
    pub fn new(cart_name: &str, save_file_name: &str, palette: UserPalette, mute: bool) -> Box<Self> {
        let (send, recv) = bounded(1);

        let ad = AudioDevice::new(send);
        let mem = MemBus::new(cart_name, save_file_name, palette, ad);

        let cpu = CPU::new(mem);

        let audio_recv = if !mute {
            start_audio_handler_thread(recv);
            None
        } else {
            Some(recv)
        };

        Box::new(RustBoy {
            cpu:        cpu,
            audio_recv: audio_recv,

            frame:      Arc::new(Mutex::new([255; FRAME_SIZE_BYTES]))
        })
    }

    // Call every 1/60 seconds.
    pub fn frame(&mut self, frame: &mut [u8]) {
        self.cpu.frame_update(self.frame.clone());    // Draw video and read inputs

        while self.cpu.step() {}    // Execute up to v-blanking

        let new_frame = self.frame.lock().unwrap();
        frame.copy_from_slice(&(*new_frame));
        
        if let Some(recv) = &mut self.audio_recv {
            while let Ok(_) = recv.try_recv() {}
        }
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

    #[cfg(feature = "debug")]
    pub fn step(&mut self) -> bool {
        self.cpu.step()
    }

    #[cfg(feature = "debug")]
    pub fn get_state(&self) -> debug::CPUState {
        self.cpu.get_state()
    }

    #[cfg(feature = "debug")]
    pub fn get_instr(&self) -> [u8; 3] {
        self.cpu.get_instr()
    }

    #[cfg(feature = "debug")]
    pub fn get_mem_at(&self, loc: u16) -> u8 {
        self.cpu.get_mem_at(loc)
    }
}
