//use cpu;
mod cpu;
mod mem;
mod video;
mod timer;
mod audio;
mod interrupt;

#[cfg(feature = "debug")]
mod debug;

use time::{Duration, PreciseTime};
use std::{
    env,
    sync::mpsc::channel
};

use cpu::CPU;
use video::VideoDevice;
use audio::{
    AudioDevice,
    start_audio_handler_thread
};
use mem::MemBus;

fn main() {
    let cart = match env::args().nth(1) {
        Some(c) => c,
        None => panic!("Usage: cargo run [cart name]"),
    };

    let save_file = match env::args().nth(2) {
        Some(c) => c,
        None => "save_file.sv".to_string(),
    };

    println!("Super Rust Boy: {}", cart);

    let (send, recv) = channel();

    let vd = VideoDevice::new();
    let ad = AudioDevice::new(send);
    let mem = MemBus::new(cart.as_str(), save_file.as_str(), vd, ad);

    let mut state = CPU::new(mem);

    start_audio_handler_thread(recv);
    
    if cfg!(feature = "debug") {
        #[cfg(feature = "debug")]
        debug::debug_mode(&mut state);
    } else {
        loop {
            let frame = PreciseTime::now();

            while state.step() {}   // Execute up to v-blanking

            state.frame_update();   // Draw video and read inputs

            while frame.to(PreciseTime::now()) < Duration::microseconds(16666) {};  // Wait until next frame.
        }
    }
}
