// CPU Module
use bitflags::bitflags;

use crate::mem::{MemBus, MemDevice, InterruptFlags};

// Interrupt constants
mod int {
    // Interrupt vector locations
    pub const V_BLANK_VECT: u16  = 0x0040;
    pub const LCD_STAT_VECT: u16 = 0x0048;
    pub const TIMER_VECT: u16    = 0x0050;
    pub const SERIAL_VECT: u16   = 0x0058;
    pub const JOYPAD_VECT: u16   = 0x0060;
}

bitflags! {
    #[derive(Default)]
    pub struct CPUFlags: u8 {
        const ZERO  = 0b10000000;
        const NEG   = 0b01000000;
        const HC    = 0b00100000;
        const CARRY = 0b00010000;
    }
}

// LR35902 CPU
pub struct CPU {
    // Accumulator
    a: u8,

    // Registers
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,

    // Flags
    flags: CPUFlags,

    // Interrupts
    ime: bool,
    cont: bool,

    // Stack Pointer & PC
    sp: u16,
    pc: u16,

    // Memory Bus (ROM,RAM,Peripherals etc)
    mem: MemBus,

    // Internals
    cycle_count: u32,
}


// Conditions for Jump
#[derive(PartialEq)]
enum Cond {
    NZ,
    NC,
    Z,
    C,
    AL,
}

impl Cond {
    fn check(&self, cpu: &CPU) -> bool {
        use self::Cond::*;
        match self {
            AL  => true,
            NZ  => !cpu.flags.contains(CPUFlags::ZERO),
            NC  => !cpu.flags.contains(CPUFlags::CARRY),
            Z   => cpu.flags.contains(CPUFlags::ZERO),
            C   => cpu.flags.contains(CPUFlags::CARRY),
        }
    }
}

// Double (16-bit) registers
enum Reg {
    AF,
    BC,
    DE,
    HL,
}

// Additional commands
enum With {
    Inc,
    Dec,
    None
}

impl With {
    fn resolve(self, hl: u16, cpu: &mut CPU) {
        use self::With::*;
        match self {
            Inc => {
                let inc = (hl as u32) + 1;
                cpu.set_16(Reg::HL, inc as u16);
            },
            Dec => {
                let dec = (hl as u32) - 1;
                cpu.set_16(Reg::HL, dec as u16);
            },
            None => {}
        }
    }
}


// Public interface
impl CPU {
    // Initialise CPU
    pub fn new(mem: MemBus) -> Self {
        CPU {
            a:      0x01,
            b:      0x00,
            c:      0x13,
            d:      0x00,
            e:      0xD8,
            h:      0x01,
            l:      0x4D,
            flags:  CPUFlags::ZERO | CPUFlags::HC | CPUFlags::CARRY,
            ime:    true,
            cont:   true,
            sp:     0xFFFE,
            pc:     0x100,
            mem:    mem,
            cycle_count: 0,
        }
    }

    // Execute the next action.
    // If it returns true, keep stepping.
    // If it returns false, wait.
    pub fn step(&mut self) -> bool {
        if !self.mem.video_mode(&mut self.cycle_count) {
            return false;   // Wait until frame is ready.
        }

        if self.handle_interrupts() {
            return true;
        }

        self.mem.update_timers(self.cycle_count);

        // Keep cycling
        if !self.cont {
            self.cycle_count += 4;
            return true;
        }

        //println!("INSTR @ {:X}: ", self.pc);
        self.exec_instruction();

        return true;
    }

    // Check for interrupts and handle if any are enabled.
    fn handle_interrupts(&mut self) -> bool {
        let interrupts = self.mem.get_interrupts();

        if self.ime && !interrupts.is_empty() {
            self.cycle_count += 8;
            self.ime = false;
            self.cont = true;

            if interrupts.contains(InterruptFlags::V_BLANK) {
                self.mem.clear_interrupt_flag(InterruptFlags::V_BLANK);
                self.call(Cond::AL, int::V_BLANK_VECT);

            } else if interrupts.contains(InterruptFlags::LCD_STAT) {
                self.mem.clear_interrupt_flag(InterruptFlags::LCD_STAT);
                self.call(Cond::AL, int::LCD_STAT_VECT);

            } else if interrupts.contains(InterruptFlags::TIMER) {
                self.mem.clear_interrupt_flag(InterruptFlags::TIMER);
                self.call(Cond::AL, int::TIMER_VECT);

            } else if interrupts.contains(InterruptFlags::SERIAL) {
                self.mem.clear_interrupt_flag(InterruptFlags::SERIAL);
                self.call(Cond::AL, int::SERIAL_VECT);

            } else if interrupts.contains(InterruptFlags::JOYPAD) {
                self.mem.clear_interrupt_flag(InterruptFlags::JOYPAD);
                self.call(Cond::AL, int::JOYPAD_VECT);
            }

            return true;
        }

        return false;
    }

    // Run a single instruction.
    fn exec_instruction(&mut self) {
        let instr = self.fetch();

        let op8 = match instr % 8 {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => self.read_hl(With::None),
            _ => self.a,
        };

        let op16 = match (instr >> 4) % 4 {
            0 => self.get_16(Reg::BC),
            1 => self.get_16(Reg::DE),
            2 => self.get_16(Reg::HL),
            _ => self.sp,
        };

        match instr {
            0x00 => self.nop(),
            0x01 => {let imm = self.fetch_16(); self.set_16(Reg::BC,imm)},
            0x02 => {let loc = self.get_16(Reg::BC); let op = self.a;
                     self.write_mem(loc,op)}, // +4?
            0x03 => {let val = self.inc_16(op16); self.set_16(Reg::BC,val)},
            0x04 => {let op = self.b; self.b = self.inc(op)},
            0x05 => {let op = self.b; self.b = self.dec(op)},
            0x06 => self.b = self.fetch(),
            0x07 => self.rlca(),
            0x08 => {let imm = self.fetch_16(); self.write_sp(imm)}
            0x09 => self.add_16(op16),
            0x0A => {let bc = self.get_16(Reg::BC); self.a = self.read_mem(bc)},
            0x0B => {let val = self.dec_16(op16); self.set_16(Reg::BC,val)},
            0x0C => {let op = self.c; self.c = self.inc(op)},
            0x0D => {let op = self.c; self.c = self.dec(op)},
            0x0E => self.c = self.fetch(),
            0x0F => self.rrca(),

            0x10 => {},//self.int_stat = IntType::Stop,
            0x11 => {let imm = self.fetch_16(); self.set_16(Reg::DE,imm)},
            0x12 => {let loc = self.get_16(Reg::DE); let op = self.a;
                     self.write_mem(loc,op)},
            0x13 => {let val = self.inc_16(op16); self.set_16(Reg::DE,val)},
            0x14 => {let op = self.d; self.d = self.inc(op)},
            0x15 => {let op = self.d; self.d = self.dec(op)},
            0x16 => self.d = self.fetch(),
            0x17 => self.rla(),
            0x18 => {let imm = self.fetch(); self.jr(Cond::AL, imm as i8)},
            0x19 => self.add_16(op16),
            0x1A => {let de = self.get_16(Reg::DE); self.a = self.read_mem(de)},
            0x1B => {let val = self.dec_16(op16); self.set_16(Reg::DE,val)},
            0x1C => {let op = self.e; self.e = self.inc(op)},
            0x1D => {let op = self.e; self.e = self.dec(op)},
            0x1E => self.e = self.fetch(),
            0x1F => self.rra(),

            0x20 => {let imm = self.fetch(); self.jr(Cond::NZ, imm as i8)},
            0x21 => {let imm = self.fetch_16(); self.set_16(Reg::HL,imm)},
            0x22 => {let op = self.a; self.write_hl(op, With::Inc)},
            0x23 => {let val = self.inc_16(op16); self.set_16(Reg::HL,val)},
            0x24 => {let op = self.h; self.h = self.inc(op)},
            0x25 => {let op = self.h; self.h = self.dec(op)},
            0x26 => self.h = self.fetch(),
            0x27 => self.daa(),
            0x28 => {let imm = self.fetch(); self.jr(Cond::Z, imm as i8)},
            0x29 => self.add_16(op16),
            0x2A => {self.a = self.read_hl(With::Inc)},
            0x2B => {let val = self.dec_16(op16); self.set_16(Reg::HL,val)},
            0x2C => {let op = self.l; self.l = self.inc(op)},
            0x2D => {let op = self.l; self.l = self.dec(op)},
            0x2E => self.l = self.fetch(),
            0x2F => self.cpl(),

            0x30 => {let imm = self.fetch(); self.jr(Cond::NC, imm as i8)},
            0x31 => self.sp = self.fetch_16(),
            0x32 => {let op = self.a; self.write_hl(op, With::Dec)},
            0x33 => self.sp = self.inc_16(op16),
            0x34 => {let op = self.read_hl(With::None); let res = self.inc(op); self.write_hl(res, With::None)},
            0x35 => {let op = self.read_hl(With::None); let res = self.dec(op); self.write_hl(res, With::None)},
            0x36 => {let imm = self.fetch(); self.write_hl(imm, With::None)},
            0x37 => self.scf(),
            0x38 => {let imm = self.fetch(); self.jr(Cond::C, imm as i8)},
            0x39 => self.add_16(op16),
            0x3A => {self.a = self.read_hl(With::Dec)},
            0x3B => self.sp = self.dec_16(op16),
            0x3C => {let op = self.a; self.a = self.inc(op)},
            0x3D => {let op = self.a; self.a = self.dec(op)},
            0x3E => self.a = self.fetch(),
            0x3F => self.ccf(),

            0x40...0x47 => self.b = op8,
            0x48...0x4F => self.c = op8,

            0x50...0x57 => self.d = op8,
            0x58...0x5F => self.e = op8,

            0x60...0x67 => self.h = op8,
            0x68...0x6F => self.l = op8,

            0x70...0x75 => self.write_hl(op8, With::None),
            0x76 => self.cont = false,
            0x77 => self.write_hl(op8, With::None),
            0x78...0x7F => self.a = op8,

            0x80...0x87 => self.add(false, op8),
            0x88...0x8F => self.add(true, op8),

            0x90...0x97 => self.sub(false, op8),
            0x98...0x9F => self.sub(true, op8),

            0xA0...0xA7 => self.and(op8),
            0xA8...0xAF => self.xor(op8),

            0xB0...0xB7 => self.or(op8),
            0xB8...0xBF => self.cp(op8),

            0xC0 => self.ret(Cond::NZ),
            0xC1 => self.pop(Reg::BC),
            0xC2 => {let imm = self.fetch_16(); self.jp(Cond::NZ, imm)},
            0xC3 => {let imm = self.fetch_16(); self.jp(Cond::AL, imm)},
            0xC4 => {let imm = self.fetch_16(); self.call(Cond::NZ, imm)},
            0xC5 => self.push(Reg::BC),
            0xC6 => {let imm = self.fetch(); self.add(false, imm)},
            0xC7 => self.call(Cond::AL, 0x00),
            0xC8 => self.ret(Cond::Z),
            0xC9 => self.ret(Cond::AL),
            0xCA => {let imm = self.fetch_16(); self.jp(Cond::Z, imm)},
            0xCB => {let ins = self.fetch(); self.prefix_cb(ins)},
            0xCC => {let imm = self.fetch_16(); self.call(Cond::Z, imm)},
            0xCD => {let imm = self.fetch_16(); self.call(Cond::AL, imm)},
            0xCE => {let imm = self.fetch(); self.add(true, imm)},
            0xCF => self.call(Cond::AL, 0x08),

            0xD0 => self.ret(Cond::NC),
            0xD1 => self.pop(Reg::DE),
            0xD2 => {let imm = self.fetch_16(); self.jp(Cond::NC, imm)},
            0xD4 => {let imm = self.fetch_16(); self.call(Cond::NC, imm)},
            0xD5 => self.push(Reg::DE),
            0xD6 => {let imm = self.fetch(); self.sub(false, imm)},
            0xD7 => self.call(Cond::AL, 0x10),
            0xD8 => self.ret(Cond::C),
            0xD9 => self.reti(),
            0xDA => {let imm = self.fetch_16(); self.jp(Cond::C, imm)},
            0xDC => {let imm = self.fetch_16(); self.call(Cond::C, imm)},
            0xDE => {let imm = self.fetch(); self.sub(true, imm)},
            0xDF => self.call(Cond::AL, 0x18),

            0xE0 => {let imm = (self.fetch() as u16) + 0xFF00;
                     let op = self.a;
                     self.write_mem(imm,op)},
            0xE1 => self.pop(Reg::HL),
            0xE2 => {let loc = (self.c as u16) + 0xFF00;
                     let op = self.a;
                     self.write_mem(loc,op)},
            0xE5 => self.push(Reg::HL),
            0xE6 => {let imm = self.fetch(); self.and(imm)},
            0xE7 => self.call(Cond::AL, 0x20),
            0xE8 => {let imm = self.fetch(); self.sp = self.add_sp(imm)},
            0xE9 => {let loc = self.get_16(Reg::HL); self.jp(Cond::AL, loc)}, // jpHL
            0xEA => {let loc = self.fetch_16();
                     let op = self.a;
                     self.write_mem(loc, op)},
            0xEE => {let imm = self.fetch(); self.xor(imm)},
            0xEF => self.call(Cond::AL, 0x28),

            0xF0 => {let imm = (self.fetch() as u16) + 0xFF00;
                     self.a = self.read_mem(imm)},
            0xF1 => self.pop(Reg::AF),
            0xF2 => {let loc = (self.c as u16) + 0xFF00;
                     self.a = self.read_mem(loc)},
            0xF3 => self.di(),
            0xF5 => self.push(Reg::AF),
            0xF6 => {let imm = self.fetch(); self.or(imm)},
            0xF7 => self.call(Cond::AL, 0x30),
            0xF8 => {let imm = self.fetch(); let val = self.add_sp(imm); self.set_16(Reg::HL, val)},
            0xF9 => self.sp = self.get_16(Reg::HL),
            0xFA => {let loc = self.fetch_16();
                     self.a = self.read_mem(loc)},
            0xFB => self.ei(),
            0xFE => {let imm = self.fetch(); self.cp(imm)},
            0xFF => self.call(Cond::AL, 0x38),

            _ => {},
        }
    }

    // Run an instruction with "0xCB" as the first byte.
    fn prefix_cb(&mut self, instr: u8) {
        let op = match instr % 0x8 {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => self.read_hl(With::None),
            _ => self.a,
        };

        let result = match instr >> 3 {
            0x00 => self.rlc(op),
            0x01 => self.rrc(op),
            0x02 => self.rl(op),
            0x03 => self.rr(op),
            0x04 => self.sla(op),
            0x05 => self.sra(op),
            0x06 => self.swap(op),
            0x07 => self.srl(op),

            x @ 0x08...0x0F => self.bit(x % 8, op),

            x @ 0x10...0x17 => self.res(x % 8, op),

            x => self.set(x % 8, op),
        };

        match (result, instr % 0x08) {
            (Some(x), 0) => self.b = x,
            (Some(x), 1) => self.c = x,
            (Some(x), 2) => self.d = x,
            (Some(x), 3) => self.e = x,
            (Some(x), 4) => self.h = x,
            (Some(x), 5) => self.l = x,
            (Some(x), 6) => self.write_hl(x, With::None),
            (Some(x), 7) => self.a = x,
            _ => {},
        }
    }

    pub fn frame_update(&mut self) {
        self.mem.render_frame();
        self.mem.read_inputs();
    }
}

// Internal
impl CPU {
    // Special access registers
    #[inline]
    fn get_f(&self) -> u8 {
        self.flags.bits()
    }

    #[inline]
    fn set_f(&mut self, f: u8) {
        self.flags = CPUFlags::from_bits_truncate(f);
    }

    #[inline]
    fn get_16(&self, which: Reg) -> u16 {
        match which {
            Reg::HL => ((self.h as u16) << 8) | (self.l as u16),
            Reg::AF => ((self.a as u16) << 8) | (self.get_f() as u16),
            Reg::BC => ((self.b as u16) << 8) | (self.c as u16),
            Reg::DE => ((self.d as u16) << 8) | (self.e as u16),
        }
    }

    #[inline]
    fn set_16(&mut self, which: Reg, val: u16) {
        match which {
            Reg::HL => {self.h = (val >> 8) as u8;
                        self.l = val as u8;},
            Reg::AF => {self.a = (val >> 8) as u8;
                        self.set_f(val as u8);},
            Reg::BC => {self.b = (val >> 8) as u8;
                        self.c = val as u8;},
            Reg::DE => {self.d = (val >> 8) as u8;
                        self.e = val as u8;},
        }
    }

    // Memory access
    #[inline]
    fn read_mem(&mut self, loc: u16) -> u8 {
        self.cycle_count += 4;
        self.mem.read(loc)
    }

    #[inline]
    fn write_mem(&mut self, loc: u16, val: u8) {
        self.cycle_count += 4;
        self.mem.write(loc, val);
    }

    // read mem pointed to by pc (and inc pc)
    fn fetch(&mut self) -> u8 {
        self.cycle_count += 4;
        let result = self.mem.read(self.pc);
        self.pc = ((self.pc as u32) + 1) as u16;
        //println!("{:X}", result);

        result
    }

    fn fetch_16(&mut self) -> u16 {
        self.cycle_count += 8;
        let lo_byte = self.mem.read(self.pc) as u16;
        self.pc = ((self.pc as u32) + 1) as u16;
        let hi_byte = (self.mem.read(self.pc) as u16) << 8;
        self.pc = ((self.pc as u32) + 1) as u16;
        //println!("{:X}", lo_byte | hi_byte);

        lo_byte | hi_byte
    }

    // read and write to/from mem pointed to by hl
    fn read_hl(&mut self, with: With) -> u8 {
        self.cycle_count += 4;
        let hl = self.get_16(Reg::HL);
        let res = self.mem.read(hl);
        with.resolve(hl, self);
        res
    }

    fn write_hl(&mut self, val: u8, with: With) {
        self.cycle_count += 4;
        let hl = self.get_16(Reg::HL);
        self.mem.write(hl, val);
        with.resolve(hl, self);
    }

    // increments sp - TODO: maybe improve this fn
    fn add_sp(&mut self, imm: u8) -> u16 {
        self.cycle_count += 8;
        let offset = imm as i8;
        let result = (self.sp as i32) + (offset as i32);
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::HC, result > 0xF);
        self.flags.set(CPUFlags::CARRY, result > 0xFFFF);
        result as u16
    }

    // writes sp to mem
    fn write_sp(&mut self, imm: u16) {
        let lo_byte = self.sp as u8;
        let hi_byte = (self.sp >> 8) as u8;
        self.write_mem(imm, lo_byte);
        self.write_mem(imm, hi_byte);
    }

    #[inline]
    fn stack_push(&mut self, val: u8) {
        self.sp = ((self.sp as i32) - 1) as u16;
        self.mem.write(self.sp, val);
    }

    #[inline]
    fn stack_pop(&mut self) -> u8 {
        let ret = self.mem.read(self.sp);
        self.sp = ((self.sp as u32) + 1) as u16;
        ret
    }
}

// Instructions
impl CPU {
    // Arithmetic
    fn add(&mut self, carry: bool, op: u8) {
        let c = if self.flags.contains(CPUFlags::CARRY) && carry {1} else {0};
        let result = (self.a as u16) + (op as u16) + c;
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::ZERO, (result & 0xFF) == 0);
        self.flags.set(CPUFlags::HC, result > 0xF);
        self.flags.set(CPUFlags::CARRY, result > 0xFF);
        self.a = result as u8;
    }

    fn add_16(&mut self, op: u16) {
        self.cycle_count += 4;
        let result = (self.get_16(Reg::HL) as u32) + (op as u32);
        self.flags.remove(CPUFlags::NEG);
        self.flags.set(CPUFlags::HC, result > 0xF);
        self.flags.set(CPUFlags::CARRY, result > 0xFFFF);
        self.set_16(Reg::HL, result as u16);
    }

    fn sub(&mut self, carry: bool, op: u8) {
        let c = if self.flags.contains(CPUFlags::CARRY) && carry {1} else {0};
        let result = (self.a as i16) - (op as i16) - c;
        self.flags = CPUFlags::NEG;
        self.flags.set(CPUFlags::ZERO, result == 0);
        self.flags.set(CPUFlags::HC, result < 0x10);
        self.flags.set(CPUFlags::CARRY, result < 0);
        self.a = result as u8;
    }

    fn and(&mut self, op: u8) {
        let result = self.a & op;
        self.flags = CPUFlags::HC;
        self.flags.set(CPUFlags::ZERO, result == 0);
        self.a = result;
    }

    fn xor(&mut self, op: u8) {
        let result = self.a ^ op;
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::ZERO, result == 0);
        self.a = result;
    }

    fn or(&mut self, op: u8) {
        let result = self.a | op;
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::ZERO, result == 0);
        self.a = result;
    }

    fn cp(&mut self, op: u8) {
        let result = (self.a as i16) - (op as i16);
        self.flags = CPUFlags::NEG;
        self.flags.set(CPUFlags::ZERO, result == 0);
        self.flags.set(CPUFlags::HC, result < 0x10);
        self.flags.set(CPUFlags::CARRY, result < 0);
        self.a = result as u8;
    }

    // inc/dec
    fn inc(&mut self, op: u8) -> u8 {
        let result = (op as u16) + 1;
        self.flags.remove(CPUFlags::NEG);
        self.flags.set(CPUFlags::ZERO, (result & 0xFF) == 0);
        self.flags.set(CPUFlags::HC, result > 0xF);
        result as u8
    }

    fn dec(&mut self, op: u8) -> u8 {
        let result = ((op as i16) - 1) as i8;
        self.flags.insert(CPUFlags::NEG);
        self.flags.set(CPUFlags::ZERO, result == 0);
        self.flags.set(CPUFlags::HC, result < 0x10);
        result as u8
    }

    fn inc_16(&mut self, op: u16) -> u16 {
        self.cycle_count += 4;
        let result = (op as u32) + 1;
        result as u16
    }

    fn dec_16(&mut self, op: u16) -> u16 {
        self.cycle_count += 4;
        let result = (op as i32) - 1;
        result as u16
    }

    // TODO: improve this
    fn daa(&mut self) {
        let lo_nib = (self.a & 0xF) as u16;
        let hi_nib = (self.a & 0xF0) as u16;
        let lo_inc = match (lo_nib, self.flags.contains(CPUFlags::NEG), self.flags.contains(CPUFlags::HC)) {
            // TODO: improve matches
            (10...15,false,false) => 0x06,
            (0...3,false,true) => 0x06,
            (6...15,true,true) => 0x0A,
            _ => 0x00,
        };
        let hi_inc = match (hi_nib, self.flags.contains(CPUFlags::CARRY), self.flags.contains(CPUFlags::NEG), self.flags.contains(CPUFlags::HC)) {
            // TODO: improve matches
            (10...15,false,false,_) => 0x60,
            (9...15,false,false,false) if lo_inc==6 => 0x60,
            (0...2,true,false,false) => 0x60,
            (0...3,true,false,true) => 0x60,
            (0...8,false,true,true) => 0xF0,
            (7...15,true,true,false) => 0xA0,
            (6...15,true,true,true) => 0x90,
            _ => 0x00,
        };
        let result = (hi_nib | lo_nib) + lo_inc + hi_inc;
        self.flags.set(CPUFlags::ZERO, (result & 0xFF) == 0);
        self.flags.remove(CPUFlags::HC);
        self.flags.set(CPUFlags::CARRY, result > 0xFF);
        self.a = (result & 0xFF) as u8;
    }

    fn cpl(&mut self) {
        self.flags.insert(CPUFlags::NEG | CPUFlags::HC);
        self.a = self.a ^ 0xFF;
    }

    // Stack
    fn pop(&mut self, which: Reg) {
        self.cycle_count += 8;
        let lo_byte = self.stack_pop();
        let hi_byte = self.stack_pop();
        match which {
            Reg::AF => {self.a = hi_byte; self.set_f(lo_byte);},
            Reg::BC => {self.b = hi_byte; self.c = lo_byte;},
            Reg::DE => {self.d = hi_byte; self.e = lo_byte;},
            Reg::HL => {self.h = hi_byte; self.l = lo_byte;},
        }
    }

    fn push(&mut self, which: Reg) {
        self.cycle_count += 12;
        let (hi_byte, lo_byte) = match which {
            Reg::AF => (self.a, self.get_f()),
            Reg::BC => (self.b, self.c),
            Reg::DE => (self.d, self.e),
            Reg::HL => (self.h, self.l),
        };
        self.stack_push(hi_byte);
        self.stack_push(lo_byte);
    }


    // Shift/Rotate
    fn rlca(&mut self) {
        let top_bit = (self.a >> 7) & 1;
        // TODO: check if z is set false here
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::CARRY, top_bit != 0);
        self.a = (self.a << 1) | top_bit;
    }

    fn rla(&mut self) {
        let carry_bit = if self.flags.contains(CPUFlags::CARRY) {1} else {0};
        let top_bit = (self.a >> 7) & 1;
        // TODO: check if z is set false here
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::CARRY, top_bit != 0);
        self.a = (self.a << 1) | carry_bit;
    }

    fn rrca(&mut self) {
        let bot_bit = (self.a << 7) & 0x80;
        // TODO: check if z is set false here
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::CARRY, bot_bit != 0);
        self.a = (self.a >> 1) | bot_bit;
    }

    fn rra(&mut self) {
        let carry_bit = if self.flags.contains(CPUFlags::CARRY) {0x80} else {0};
        let bot_bit = (self.a << 7) & 0x80;
        // TODO: check if z is set false here
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::CARRY, bot_bit != 0);
        self.a = (self.a >> 1) | carry_bit;
    }

    fn rlc(&mut self, op: u8) -> Option<u8> {
        let top_bit = (op >> 7) & 1;
        let result = (op << 1) | top_bit;
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::ZERO, result == 0);
        self.flags.set(CPUFlags::CARRY, top_bit != 0);
        Some(result)
    }

    fn rl(&mut self, op: u8) -> Option<u8> {
        let carry_bit = if self.flags.contains(CPUFlags::CARRY) {1} else {0};
        let top_bit = (op >> 7) & 1;
        let result = (op << 1) | carry_bit;
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::ZERO, result == 0);
        self.flags.set(CPUFlags::CARRY, top_bit != 0);
        Some(result)
    }

    fn rrc(&mut self, op: u8) -> Option<u8> {
        let bot_bit = (op << 7) & 0x80;
        let result = (op >> 1) | bot_bit;
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::ZERO, result == 0);
        self.flags.set(CPUFlags::CARRY, bot_bit != 0);
        Some(result)
    }

    fn rr(&mut self, op: u8) -> Option<u8> {
        let carry_bit = if self.flags.contains(CPUFlags::CARRY) {0x80} else {0};
        let bot_bit = (op << 7) & 0x80;
        let result = (op >> 1) | carry_bit;
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::ZERO, result == 0);
        self.flags.set(CPUFlags::CARRY, bot_bit != 0);
        Some(result)
    }

    fn sla(&mut self, op: u8) -> Option<u8> {
        let result = op << 1;
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::ZERO, result == 0);
        self.flags.set(CPUFlags::CARRY, (op & 0x80) != 0);
        Some(result)
    }

    fn sra(&mut self, op: u8) -> Option<u8> {
        let result = op >> 1;
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::ZERO, result == 0);
        self.flags.set(CPUFlags::CARRY, (op & 0x1) != 0);
        Some(result)
    }

    fn srl(&mut self, op: u8) -> Option<u8> {
        let result = op >> 1;
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::ZERO, result == 0);
        self.flags.set(CPUFlags::CARRY, (op & 0x1) != 0);
        Some(result)
    }

    fn swap(&mut self, op: u8) -> Option<u8> {
        let result = (op << 4) | (op >> 4);
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::ZERO, result == 0);
        Some(result)
    }

    // Bit setting & testing
    fn set(&mut self, b: u8, op: u8) -> Option<u8> {
        let result = op | (1 << b);
        Some(result)
    }

    fn res(&mut self, b: u8, op: u8) -> Option<u8> {
        let result = op & !(1 << b);
        Some(result)
    }

    fn bit(&mut self, b: u8, op: u8) -> Option<u8> {
        self.flags.set(CPUFlags::ZERO, (op & (1 << b)) == 0);
        self.flags.remove(CPUFlags::NEG);
        self.flags.insert(CPUFlags::HC);
        None
    }

    // Control commands
    fn scf(&mut self) {
        self.flags.remove(CPUFlags::NEG | CPUFlags::HC);
        self.flags.insert(CPUFlags::CARRY);
    }

    fn ccf(&mut self) {
        self.flags.remove(CPUFlags::NEG | CPUFlags::HC);
        self.flags.toggle(CPUFlags::CARRY);
    }

    fn nop(&self) {
        return;
    }

    // halt, stop
    fn di(&mut self) {
        self.ime = false;
    }

    fn ei(&mut self) {
        self.ime = true;
    }

    // Jump
    fn jp(&mut self, cd: Cond, loc: u16) {
        if cd.check(&self) {
            self.cycle_count += 4;
            self.pc = loc
        }
    }

    fn jr(&mut self, cd: Cond, loc: i8) {
        if cd.check(&self) {
            self.cycle_count += 4;
            self.pc = ((self.pc as i32) + (loc as i32)) as u16;
        }
    }

    fn call(&mut self, cd: Cond, loc: u16) {
        if cd.check(&self) {
            self.cycle_count += 12;
            let hi_byte = (self.pc >> 8) as u8;
            let lo_byte = self.pc as u8;
            self.stack_push(hi_byte);
            self.stack_push(lo_byte);
            self.pc = loc;
        }
    }

    fn ret(&mut self, cd: Cond) {
        self.cycle_count += 4;

        if cd.check(&self) {
            if cd == Cond::AL {
                self.cycle_count += 8;
            } else {
                self.cycle_count += 12;
            }
            let lo_byte = self.stack_pop() as u16;
            let hi_byte = self.stack_pop() as u16;
            self.pc = (hi_byte << 8) | lo_byte;
        }
    }

    fn reti(&mut self) {
        self.cycle_count += 12;

        self.ime = true;
        let lo_byte = self.stack_pop() as u16;
        let hi_byte = self.stack_pop() as u16;
        self.pc = (hi_byte << 8) | lo_byte;
    }
}

impl CPU {
    #[cfg(feature = "debug")]
    pub fn get_state(&self) -> crate::debug::CPUState {
        crate::debug::CPUState {
            a: self.a,
            b: self.b,
            c: self.c,
            d: self.d,
            e: self.e,
            h: self.h,
            l: self.l,
            flags: self.flags.bits(),
            pc: self.pc,
            sp: self.sp
        }
    }

    #[cfg(feature = "debug")]
    pub fn get_instr(&self) -> [u8; 3] {
        [
            self.mem.read(self.pc),
            self.mem.read(self.pc + 1),
            self.mem.read(self.pc + 2)
        ]
    }

    #[cfg(feature = "debug")]
    pub fn get_mem_at(&self, loc: u16) -> u8 {
        self.mem.read(loc)
    }
}