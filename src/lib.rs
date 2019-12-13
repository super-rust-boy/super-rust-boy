#[macro_use]
mod utils;

mod cpu;
mod mem;
mod video;
mod timer;
mod audio;
mod interrupt;

#[cfg(feature = "debug")]
pub mod debug;

pub use video::UserPalette;

use std::sync::mpsc::{channel, Receiver};

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
    pub fn new(cart_name: &str, save_file_name: &str, palette: UserPalette, mute: bool) -> Box<Self> {
        let (send, recv) = channel();

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

use std::os::raw::c_char;
use std::ffi::{c_void, CStr};

#[no_mangle]
pub extern fn rustBoyCreate(cartridge_path: *const c_char, save_file_path: *const c_char) -> *const c_void {

	let save_path_result = unsafe { CStr::from_ptr(save_file_path) };
	let save_path = match save_path_result.to_str() {
		Ok(c) => c,
		Err(_) => panic!("Failed to parse save path")
	};

	let cart_path_result = unsafe { CStr::from_ptr(cartridge_path) };
	let cart_path = match cart_path_result.to_str() {
		Ok(c) => c,
		Err(_) => panic!("Failed to parse cartridge path")
	};

	let instance = RustBoy::new(cart_path, save_path, UserPalette::Default, true);

	Box::into_raw(instance) as *const c_void
}
