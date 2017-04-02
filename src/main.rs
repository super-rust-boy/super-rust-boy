//use cpu;
mod cpu;
mod mem;
mod common;

fn main() {
    println!("Hello, world!");
    let state = cpu::CPU::new();
}
