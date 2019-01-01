//use cpu;
mod cpu;
mod mem;
mod video;
mod timer;
mod audio;

extern crate time;
#[macro_use]
extern crate glium;

extern crate cpal;

use time::{Duration, PreciseTime};
use std::sync::mpsc::channel;

fn main() {
    let cart = match std::env::args().nth(1) {
        Some(c) => c,
        None => panic!("Usage: cargo run [cart name]"),
    };

    println!("Super Rust Boy: {}", cart);

    let (send, recv) = channel();

    let vd = video::GBVideo::new();
    let ad = audio::AudioDevice::new(send);
    let mem = mem::MemBus::new(cart.as_str(), vd, ad);

    let mut state = cpu::CPU::new(mem);

    loop {
        let frame = PreciseTime::now();

        while state.step() {}   // Execute up to v-blanking

        state.frame_update();   // Draw video and read inputs

        while frame.to(PreciseTime::now()) < Duration::microseconds(16750) {};  // Wait until next frame.
    }
}
