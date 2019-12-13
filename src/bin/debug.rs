// Debug things
use rustboy::RustBoy;

pub fn debug_mode(cpu: &mut RustBoy) {
    // Debug mode.
    println!("Debug mode.");
    println!("Enter 'h' for help.");
    let mut breaks = std::collections::BTreeSet::new();
    let mut stack_trace = Vec::new();
    loop {
        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(_) => if input.starts_with("b:") {
                // Add breakpoint
                match u16::from_str_radix(&input[2..].trim(), 16) {
                    Ok(num) => {
                        println!("Inserted breakpoint at 0x{:X}", num);
                        breaks.insert(num);
                    },
                    Err(e) => println!("Invalid breakpoint: {}", e),
                }
            } else if input.starts_with("c:") {
                // Remove breakpoint
                match u16::from_str_radix(&input[2..].trim(), 16) {
                    Ok(num) => {
                        println!("Cleared breakpoint at 0x{:X}", num);
                        breaks.remove(&num);
                    },
                    Err(e) => println!("Invalid breakpoint: {}", e),
                }
            } else if input.starts_with("c") {
                // Remove all breakpoints
                println!("Cleared all breakpoints");
                breaks.clear();
            } else if input.starts_with("r") {
                // Run
                loop {
                    let pc = cpu.get_state().pc;
                    if breaks.contains(&pc) {
                        println!("Break at 0x{:X}", pc);
                        break;
                    } else {
                        step_and_trace(cpu, &mut stack_trace, false);
                    }
                }
            } else if input.starts_with("s") {
                // Step
                step_and_trace(cpu, &mut stack_trace, true);
            } else if input.starts_with("p:") {
                // Print cpu or mem state
                print(&input[2..].trim(), cpu);
            } else if input.starts_with("p") {
                // Print state
                println!("State:\n{}", cpu.get_state().to_string());
            } else if input.starts_with("t") {
                let trace = stack_trace.iter()
                    .map(|n| format!("{:04X}", n))
                    .collect::<Vec<_>>()
                    .join("\n");
                println!("{}", trace);
            } else if input.starts_with("h") {
                // Help
                help();
            } else if input.starts_with("q") {
                break;
            },
            Err(e) => println!("Input error: {}", e),
        }
    }
}

fn print(s: &str, cpu: &RustBoy) {
    match s {
        "a" => println!("a: 0x{:02X}", cpu.get_state().a),
        "b" => println!("b: 0x{:02X}", cpu.get_state().b),
        "c" => println!("c: 0x{:02X}", cpu.get_state().c),
        "d" => println!("d: 0x{:02X}", cpu.get_state().d),
        "e" => println!("e: 0x{:02X}", cpu.get_state().e),
        "f" => println!("f: 0x{:04b}", cpu.get_state().flags),
        "h" => println!("h: 0x{:02X}", cpu.get_state().h),
        "l" => println!("l: 0x{:02X}", cpu.get_state().l),
        "af" => println!("af: 0x{:04X}", ((cpu.get_state().a as u16) << 8) | (cpu.get_state().flags as u16)),
        "bc" => println!("bc: 0x{:04X}", ((cpu.get_state().b as u16) << 8) | (cpu.get_state().c as u16)),
        "de" => println!("de: 0x{:04X}", ((cpu.get_state().d as u16) << 8) | (cpu.get_state().e as u16)),
        "hl" => println!("hl: 0x{:04X}", ((cpu.get_state().h as u16) << 8) | (cpu.get_state().l as u16)),
        "pc" => println!("pc: 0x{:04X}", cpu.get_state().pc),
        "sp" => println!("sp: 0x{:04X}", cpu.get_state().sp),
        "(pc)" => println!("pc mem: 0x{:02X}", cpu.get_mem_at(cpu.get_state().pc)),
        "(sp)" => println!("sp mem: 0x{:02X}", cpu.get_mem_at(cpu.get_state().sp)),
        "(hl)" => println!("hl mem: 0x{:02X}", cpu.get_mem_at(((cpu.get_state().h as u16) << 8) | (cpu.get_state().l as u16))),
        s => {
            // Memory range
            if let Some(x) = s.find('-') {
                match u16::from_str_radix(&s[..x], 16) {
                    Ok(start) => match u16::from_str_radix(&s[(x+1)..], 16) {
                        Ok(end) => {
                            println!("0x{:04X} - 0x{:04X}:", start, end);
                            let mems = (start..end).map(|n| format!("{:02X}", cpu.get_mem_at(n)))
                                .collect::<Vec<_>>()
                                .join(" ");
                            println!("{}", mems);
                        },
                        Err(e) => println!("Invalid p tag: {}", e),
                    },
                    Err(e) => println!("Invalid p tag: {}", e),
                }
            } else {    // Single location
                match u16::from_str_radix(s, 16) {
                    Ok(num) => println!("0x{:04X}: 0x{:02X}", num, cpu.get_mem_at(num)),
                    Err(e) => println!("Invalid p tag: {}", e),
                }
            }
        }
    }
}

fn help() {
    println!("b:x: New breakpoint at memory location x (hex).");
    println!("c:x: Clear breakpoint at memory location x (hex).");
    println!("r: Keep running until a breakpoint is hit.");
    println!("s: Step a single instruction.");
    println!("t: Print the stack trace (all the call locations).");
    println!("p: Print the current state of the CPU.");
    println!("p:x: Print x - if x is a number, print the contents of that address, otherwise print the register.");
    println!("p:x-y: Print the memory in the range x -> y.");
    println!("q: Quit execution.");
}

// Step the CPU, and add the PC to the stack trace if it calls.
fn step_and_trace(cpu: &mut RustBoy, stack_trace: &mut Vec<u16>, print: bool) {
    let instr = cpu.get_instr();
    match instr[0] {
        0xC4 | 0xCC | 0xCD | 0xD4 | 0xDC => {
            stack_trace.push(cpu.get_state().pc);
        },
        0xC0 | 0xC8 | 0xC9 | 0xD0 | 0xD8 => {
            stack_trace.pop();
        },
        _ => {}
    }

    if print {
        let pc = cpu.get_state().pc;
        println!("0x{:04X}: 0x{:02X} ({:02X} {:02X})", pc, instr[0], instr[1], instr[2]);
    }

    cpu.step();
}