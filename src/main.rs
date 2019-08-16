//use cpu;
mod cpu;
mod mem;
mod video;
mod timer;
mod audio;

#[cfg(feature = "debug")]
mod debug;

use time::{Duration, PreciseTime};
use std::sync::mpsc::channel;

fn main() {
    let cart = match std::env::args().nth(1) {
        Some(c) => c,
        None => panic!("Usage: cargo run [cart name]"),
    };

    let save_file = match std::env::args().nth(2) {
        Some(c) => c,
        None => "save_file.sv".to_string(),
    };

    println!("Super Rust Boy: {}", cart);

    let (send, recv) = channel();

    let vd = video::VideoDevice::new();
    let ad = audio::AudioDevice::new(send);
    let mem = mem::MemBus::new(cart.as_str(), save_file.as_str(), vd, ad);

    let mut state = cpu::CPU::new(mem);

    audio::start_audio_handler_thread(recv);
    
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
