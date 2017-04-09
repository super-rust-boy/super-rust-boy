//use cpu;
mod cpu;
mod mem;

extern crate time;

use time::{Duration, PreciseTime};

fn main() {
    println!("Hello, world!");
    let mut state = cpu::CPU::new();
    //let mut count = 0;
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
    //println!("count:{}", count);
}
