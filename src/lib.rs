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

use crossbeam_channel::unbounded;

use audio::Resampler;
use cpu::CPU;
use mem::MemBus;
pub use mem::ROMType;

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
    pub fn new(rom: ROMType, save_file_name: &str, palette: UserPalette) -> Box<Self> {
        let mem = MemBus::new(rom, save_file_name, palette);
        let cpu = CPU::new(mem);

        Box::new(RustBoy {
            cpu:            cpu,

            frame:          Arc::new(Mutex::new([255; FRAME_SIZE_BYTES])),
        })
    }

    pub fn enable_audio(&mut self, sample_rate: usize) -> RustBoyAudioHandle {
        let (audio_send, audio_recv) = unbounded();

        self.cpu.enable_audio(audio_send);

        RustBoyAudioHandle {
            resampler: Resampler::new(audio_recv, sample_rate as f64)
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

    pub fn cart_name(&self) -> String {
        self.cpu.cart_name()
    }
}

pub struct RustBoyAudioHandle {
    resampler: Resampler,
}

impl RustBoyAudioHandle {
    pub fn get_audio_packet(&mut self, packet: &mut [f32]) {
        for (o_frame, i_frame) in packet.chunks_exact_mut(2).zip(&mut self.resampler) {
            for (o, i) in o_frame.iter_mut().zip(i_frame.iter()) {
                *o = *i;
            }
        }
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