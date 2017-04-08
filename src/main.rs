//use cpu;
mod cpu;
mod mem;

fn main() {
    println!("Hello, world!");
    let mut state = cpu::CPU::new();
    println!("{}", state.to_string());
    state.step();
    println!("{}", state.to_string());
    println!("{}", state.test_mem(0x200));
    println!("{}", state.test_mem(0x2000));
    println!("{}", state.test_mem(0x6000));
}
