//use cpu;
mod cpu;
mod mem;
mod video;
mod timer;

extern crate time;
#[macro_use]
extern crate glium;

use time::{Duration, PreciseTime};

fn main() {
    let cart = match std::env::args().nth(1) {
        Some(c) => c,
        None => panic!("Usage: cargo run [cart name]"),
    };

    println!("Super Rust Boy: {}", cart);

    let vd = video::GBVideo::new();
    let mem = mem::MemBus::new(cart.as_str(), vd);

    let mut state = cpu::CPU::new(mem);
    /*let mut count = 0;
    println!("{}", state.to_string());

    let start = PreciseTime::now();
    while start.to(PreciseTime::now()) < Duration::seconds(1) {
        let frame = PreciseTime::now();
        let mut count = 0;
        while frame.to(PreciseTime::now()) < Duration::microseconds(16750) {
            state.step();
            count += 1;
        }
        //state.v_blank();
        println!("frame:{}", count);
    }
    println!("{}", state.to_string());
    //println!("count:{}", count);*/

    loop {
        let frame = PreciseTime::now();
        while state.step() {}   // Execute up to v-blanking
        state.frame_update();   // Draw video and read inputs
        while frame.to(PreciseTime::now()) < Duration::microseconds(16750) {};  // Wait until next frame.
    }
}
