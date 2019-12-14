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

pub use video::UserPalette;

pub use joypad::{
    Buttons,
    Directions
};

use std::sync::mpsc::{channel, Receiver};
use winit::EventsLoop;

use cpu::CPU;
use audio::{
    AudioCommand,
    AudioDevice,
    start_audio_handler_thread
};
use mem::MemBus;

pub struct RustBoy {
    cpu: CPU,
    audio_recv: Option<Receiver<AudioCommand>>
}

impl RustBoy {
    pub fn new(cart_name: &str, save_file_name: &str, palette: UserPalette, mute: bool, events_loop: &EventsLoop) -> Box<Self> {
        let (send, recv) = channel();

        let ad = AudioDevice::new(send);
        let mem = MemBus::new(cart_name, save_file_name, palette, ad, events_loop);
    
        let cpu = CPU::new(mem);
    
        let audio_recv = if !mute {
            start_audio_handler_thread(recv);
            None
        } else {
            Some(recv)
        };

        Box::new(RustBoy {
            cpu: cpu,
            audio_recv: audio_recv
        })
    }

    pub fn step(&mut self) -> bool {
        self.cpu.step()
    }

    pub fn frame(&mut self) {
        self.cpu.frame_update();

        if let Some(recv) = &mut self.audio_recv {
            while let Ok(_) = recv.try_recv() {}
        }
    }

    pub fn set_button(&mut self, button: Buttons, val: bool) {
        self.cpu.set_button(button, val);
    }

    pub fn set_direction(&mut self, direction: Directions, val: bool) {
        self.cpu.set_direction(direction, val);
    }

    pub fn on_resize(&mut self) {
        self.cpu.on_resize();
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