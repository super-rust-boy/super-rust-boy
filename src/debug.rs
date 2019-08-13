// Debug things

pub struct CPUState {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,

    pub h: u8,
    pub l: u8,

    pub flags: u8,

    pub pc: u16,
    pub sp: u16
}

impl CPUState {
    pub fn to_string(&self) -> String {
        format!("a:{:02X} b:{:02X} c:{:02X} d:{:02X} e:{:02X} h:{:02X} l:{:02X}\n\
                zhnc: {:08b}\n\
                pc: {:04X} sp: {:04X}",
                self.a, self.b, self.c, self.d, self.e, self.h, self.l,
                self.flags,
                self.pc, self.sp)
    }
}

pub fn debug_mode(cpu: &mut crate::cpu::CPU) {
    // Debug mode.
    println!("Debug mode.");
    println!("Enter 'h' for help.");
    let mut breaks = std::collections::BTreeSet::new();
    loop {
        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(_) => if input.starts_with("b:") {
                // Breakpoint
                let num = u16::from_str_radix(&input[2..].trim(), 16).expect("Invalid breakpoint");
                println!("Inserted breakpoint at 0x{:X}", num);
                breaks.insert(num);
            } else if input.starts_with("r") {
                // Run
                loop {
                    let pc = cpu.get_state().pc;
                    if breaks.contains(&pc) {
                        println!("Break at 0x{:X}", pc);
                        break;
                    } else {
                        cpu.step();
                    }
                }
            } else if input.starts_with("s") {
                // Step
                let instr = cpu.get_instr();
                let pc = cpu.get_state().pc;
                println!("0x{:04X}: 0x{:02X} ({:02X} {:02X})", pc, instr[0], instr[1], instr[2]);
                cpu.step();
            } else if input.starts_with("p") {
                // Print state
                let state = cpu.get_state();
                println!("State:\n{}", state.to_string());
            } else if input.starts_with("h") {
                // Help
                println!("b:x: New breakpoint at memory location x (hex).");
                println!("r: Keep running until a breakpoint is hit.");
                println!("s: Step a single instruction.");
                println!("p: Print the current state of the CPU.");
                println!("q: Quit execution.");
            } else if input.starts_with("q") {
                break;
            },
            Err(e) => println!("Input error: {}", e),
        }
    }
}