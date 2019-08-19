// CPU Module
use bitflags::bitflags;

use crate::mem::{MemBus, MemDevice};
use crate::interrupt::*;

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

// Additional commands for HL.
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
            a:      0x01,   // 0x11 for CGB
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
            self.mem.update_audio(self.cycle_count);
            return true;
        }

        // Keep cycling
        if !self.cont {
            self.clock_inc();
        } else {
            self.exec_instruction();
        }

        self.mem.update_audio(self.cycle_count);

        return true;
    }

    // Increment cycle count and update timer.
    #[inline]
    fn clock_inc(&mut self) {
        self.cycle_count += 4;
        self.mem.update_timer();
    }

    // Check for interrupts. Return true if they are serviced.
    fn handle_interrupts(&mut self) -> bool {
        let interrupts = self.mem.get_interrupts();

        if !interrupts.is_empty() {
            self.cont = true;

            if self.ime {
                self.clock_inc();
                self.clock_inc();
                self.ime = false;

                if interrupts.contains(InterruptFlags::V_BLANK) {
                    self.mem.clear_interrupt_flag(InterruptFlags::V_BLANK);
                    self.call(Cond::AL, vector::V_BLANK);

                } else if interrupts.contains(InterruptFlags::LCD_STAT) {
                    self.mem.clear_interrupt_flag(InterruptFlags::LCD_STAT);
                    self.call(Cond::AL, vector::LCD_STAT);

                } else if interrupts.contains(InterruptFlags::TIMER) {
                    self.mem.clear_interrupt_flag(InterruptFlags::TIMER);
                    self.call(Cond::AL, vector::TIMER);

                } else if interrupts.contains(InterruptFlags::SERIAL) {
                    self.mem.clear_interrupt_flag(InterruptFlags::SERIAL);
                    self.call(Cond::AL, vector::SERIAL);

                } else if interrupts.contains(InterruptFlags::JOYPAD) {
                    self.mem.clear_interrupt_flag(InterruptFlags::JOYPAD);
                    self.call(Cond::AL, vector::JOYPAD);
                }

                return true;
            }
        }

        return false;
    }

    // Run a single instruction.
    fn exec_instruction(&mut self) {
        let instr = self.fetch();

        let op8 = |cpu: &mut CPU| match instr % 8 {
            0 => cpu.b,
            1 => cpu.c,
            2 => cpu.d,
            3 => cpu.e,
            4 => cpu.h,
            5 => cpu.l,
            6 => cpu.read_hl(With::None),
            _ => cpu.a,
        };

        let op16 = |cpu: &mut CPU| match (instr >> 4) % 4 {
            0 => cpu.get_16(Reg::BC),
            1 => cpu.get_16(Reg::DE),
            2 => cpu.get_16(Reg::HL),
            _ => cpu.sp,
        };

        match instr {
            0x00 => self.nop(),
            0x01 => {let imm = self.fetch_16(); self.set_16(Reg::BC,imm)},
            0x02 => {let loc = self.get_16(Reg::BC); let op = self.a;
                     self.write_mem(loc,op)},
            0x03 => {let op = op16(self); let val = self.inc_16(op); self.set_16(Reg::BC,val)},
            0x04 => {let op = self.b; self.b = self.inc(op)},
            0x05 => {let op = self.b; self.b = self.dec(op)},
            0x06 => self.b = self.fetch(),
            0x07 => self.rlca(),
            0x08 => {let imm = self.fetch_16(); self.write_sp(imm)}
            0x09 => {let op = op16(self); self.add_16(op)},
            0x0A => {let bc = self.get_16(Reg::BC); self.a = self.read_mem(bc)},
            0x0B => {let op = op16(self); let val = self.dec_16(op); self.set_16(Reg::BC,val)},
            0x0C => {let op = self.c; self.c = self.inc(op)},
            0x0D => {let op = self.c; self.c = self.dec(op)},
            0x0E => self.c = self.fetch(),
            0x0F => self.rrca(),

            0x10 => {self.fetch(); self.cont = false},
            0x11 => {let imm = self.fetch_16(); self.set_16(Reg::DE,imm)},
            0x12 => {let loc = self.get_16(Reg::DE); let op = self.a;
                     self.write_mem(loc,op)},
            0x13 => {let op = op16(self); let val = self.inc_16(op); self.set_16(Reg::DE,val)},
            0x14 => {let op = self.d; self.d = self.inc(op)},
            0x15 => {let op = self.d; self.d = self.dec(op)},
            0x16 => self.d = self.fetch(),
            0x17 => self.rla(),
            0x18 => {let imm = self.fetch(); self.jr(Cond::AL, imm as i8)},
            0x19 => {let op = op16(self); self.add_16(op)},
            0x1A => {let de = self.get_16(Reg::DE); self.a = self.read_mem(de)},
            0x1B => {let op = op16(self); let val = self.dec_16(op); self.set_16(Reg::DE,val)},
            0x1C => {let op = self.e; self.e = self.inc(op)},
            0x1D => {let op = self.e; self.e = self.dec(op)},
            0x1E => self.e = self.fetch(),
            0x1F => self.rra(),

            0x20 => {let imm = self.fetch(); self.jr(Cond::NZ, imm as i8)},
            0x21 => {let imm = self.fetch_16(); self.set_16(Reg::HL,imm)},
            0x22 => {let op = self.a; self.write_hl(op, With::Inc)},
            0x23 => {let op = op16(self); let val = self.inc_16(op); self.set_16(Reg::HL,val)},
            0x24 => {let op = self.h; self.h = self.inc(op)},
            0x25 => {let op = self.h; self.h = self.dec(op)},
            0x26 => self.h = self.fetch(),
            0x27 => self.daa(),
            0x28 => {let imm = self.fetch(); self.jr(Cond::Z, imm as i8)},
            0x29 => {let op = op16(self); self.add_16(op)},
            0x2A => {self.a = self.read_hl(With::Inc)},
            0x2B => {let op = op16(self); let val = self.dec_16(op); self.set_16(Reg::HL,val)},
            0x2C => {let op = self.l; self.l = self.inc(op)},
            0x2D => {let op = self.l; self.l = self.dec(op)},
            0x2E => self.l = self.fetch(),
            0x2F => self.cpl(),

            0x30 => {let imm = self.fetch(); self.jr(Cond::NC, imm as i8)},
            0x31 => self.sp = self.fetch_16(),
            0x32 => {let op = self.a; self.write_hl(op, With::Dec)},
            0x33 => {let op = op16(self); self.sp = self.inc_16(op)},
            0x34 => {let op = self.read_hl(With::None); let res = self.inc(op); self.write_hl(res, With::None)},
            0x35 => {let op = self.read_hl(With::None); let res = self.dec(op); self.write_hl(res, With::None)},
            0x36 => {let imm = self.fetch(); self.write_hl(imm, With::None)},
            0x37 => self.scf(),
            0x38 => {let imm = self.fetch(); self.jr(Cond::C, imm as i8)},
            0x39 => {let op = op16(self); self.add_16(op)},
            0x3A => {self.a = self.read_hl(With::Dec)},
            0x3B => {let op = op16(self); self.sp = self.dec_16(op)},
            0x3C => {let op = self.a; self.a = self.inc(op)},
            0x3D => {let op = self.a; self.a = self.dec(op)},
            0x3E => self.a = self.fetch(),
            0x3F => self.ccf(),

            0x40..=0x47 => self.b = op8(self),
            0x48..=0x4F => self.c = op8(self),

            0x50..=0x57 => self.d = op8(self),
            0x58..=0x5F => self.e = op8(self),

            0x60..=0x67 => self.h = op8(self),
            0x68..=0x6F => self.l = op8(self),

            0x70..=0x75 => {let op = op8(self); self.write_hl(op, With::None)},
            0x76 => self.cont = false,
            0x77 => {let op = op8(self); self.write_hl(op, With::None)},
            0x78..=0x7F => self.a = op8(self),

            0x80..=0x87 => {let op = op8(self); self.add(false, op)},
            0x88..=0x8F => {let op = op8(self); self.add(true, op)},

            0x90..=0x97 => {let op = op8(self); self.sub(false, op)},
            0x98..=0x9F => {let op = op8(self); self.sub(true, op)},

            0xA0..=0xA7 => {let op = op8(self); self.and(op)},
            0xA8..=0xAF => {let op = op8(self); self.xor(op)},

            0xB0..=0xB7 => {let op = op8(self); self.or(op)},
            0xB8..=0xBF => {let op = op8(self); self.cp(op)},

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
            0xE8 => {let imm = self.fetch(); self.sp = self.add_sp(imm); self.clock_inc()},
            0xE9 => {self.pc = self.get_16(Reg::HL)},
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
            0xF9 => {self.sp = self.get_16(Reg::HL); self.clock_inc()},
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

            x @ 0x08..=0x0F => self.bit(x % 8, op),

            x @ 0x10..=0x17 => self.res(x % 8, op),

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
        self.mem.flush_cart();
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
        self.clock_inc();
        self.mem.read(loc)
    }

    #[inline]
    fn write_mem(&mut self, loc: u16, val: u8) {
        self.clock_inc();
        self.mem.write(loc, val);
    }

    // read mem pointed to by pc (and inc pc)
    fn fetch(&mut self) -> u8 {
        let result = self.read_mem(self.pc);
        self.pc = ((self.pc as u32) + 1) as u16;

        result
    }

    fn fetch_16(&mut self) -> u16 {
        let lo_byte = self.read_mem(self.pc) as u16;
        self.pc = ((self.pc as u32) + 1) as u16;
        let hi_byte = (self.read_mem(self.pc) as u16) << 8;
        self.pc = ((self.pc as u32) + 1) as u16;

        lo_byte | hi_byte
    }

    // read and write to/from mem pointed to by hl
    fn read_hl(&mut self, with: With) -> u8 {
        let hl = self.get_16(Reg::HL);
        let res = self.read_mem(hl);
        with.resolve(hl, self);
        res
    }

    fn write_hl(&mut self, val: u8, with: With) {
        let hl = self.get_16(Reg::HL);
        self.write_mem(hl, val);
        with.resolve(hl, self);
    }

    // increments sp - TODO: maybe improve this fn
    fn add_sp(&mut self, imm: u8) -> u16 {
        self.clock_inc();
        let offset = imm as i8;
        let result = (self.sp as i32) + (offset as i32);
        self.flags = CPUFlags::default();
        if offset >= 0 {
            self.flags.set(CPUFlags::HC, (self.sp & 0xF) + ((offset as u16) & 0xF) > 0xF);
            self.flags.set(CPUFlags::CARRY, (self.sp & 0xFF) + offset as u16 > 0xFF);
        } else {
            self.flags.set(CPUFlags::HC, (self.sp as i32) & 0xF >= result & 0xF);
            self.flags.set(CPUFlags::CARRY, (self.sp as i32) & 0xFF >= result & 0xFF);
        }
        result as u16
    }

    // writes sp to mem
    fn write_sp(&mut self, imm: u16) {
        let lo_byte = self.sp as u8;
        let hi_byte = (self.sp >> 8) as u8;
        self.write_mem(imm, lo_byte);
        self.write_mem(imm + 1, hi_byte);
    }

    #[inline]
    fn stack_push(&mut self, val: u8) {
        self.sp = ((self.sp as i32) - 1) as u16;
        self.write_mem(self.sp, val);
    }

    #[inline]
    fn stack_pop(&mut self) -> u8 {
        let ret = self.read_mem(self.sp);
        self.sp = ((self.sp as u32) + 1) as u16;
        ret
    }
}

// Instructions
impl CPU {
    // Arithmetic
    fn add(&mut self, carry: bool, op: u8) {
        let c = if self.flags.contains(CPUFlags::CARRY) && carry {1_u8} else {0_u8};
        let result = (self.a as u16) + (op as u16) + (c as u16);
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::ZERO, (result & 0xFF) == 0);
        self.flags.set(CPUFlags::HC, ((self.a & 0xF) + (op & 0xF) + c) > 0xF);
        self.flags.set(CPUFlags::CARRY, result > 0xFF);
        self.a = result as u8;
    }

    fn add_16(&mut self, op: u16) {
        self.clock_inc();
        let hl = self.get_16(Reg::HL);
        let result = (hl as u32) + (op as u32);
        self.flags.remove(CPUFlags::NEG);
        self.flags.set(CPUFlags::HC, ((hl & 0xFFF) + (op & 0xFFF)) > 0xFFF);
        self.flags.set(CPUFlags::CARRY, result > 0xFFFF);
        self.set_16(Reg::HL, result as u16);
    }

    fn sub(&mut self, carry: bool, op: u8) {
        let c = if self.flags.contains(CPUFlags::CARRY) && carry {1_u8} else {0_u8};
        let result = (self.a as i16) - (op as i16) - (c as i16);
        self.flags = CPUFlags::NEG;
        self.flags.set(CPUFlags::ZERO, (result as u8) == 0);
        self.flags.set(CPUFlags::HC, (self.a & 0xF) < (((result as u8) & 0xF) + c));
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
        self.flags.set(CPUFlags::ZERO, (result as u8) == 0);
        self.flags.set(CPUFlags::HC, (self.a & 0xF) < (result as u8 & 0xF));
        self.flags.set(CPUFlags::CARRY, result < 0);
    }

    // inc/dec
    fn inc(&mut self, op: u8) -> u8 {
        let result = (op as u16) + 1;
        self.flags.remove(CPUFlags::NEG);
        self.flags.set(CPUFlags::ZERO, (result & 0xFF) == 0);
        self.flags.set(CPUFlags::HC, ((op & 0xF) + 1) > 0xF);
        result as u8
    }

    fn dec(&mut self, op: u8) -> u8 {
        let result = ((op as i16) - 1) as i8;
        self.flags.insert(CPUFlags::NEG);
        self.flags.set(CPUFlags::ZERO, (result as u8) == 0);
        self.flags.set(CPUFlags::HC, (op & 0xF) < (result as u8 & 0xF));
        result as u8
    }

    fn inc_16(&mut self, op: u16) -> u16 {
        self.clock_inc();
        let result = (op as u32) + 1;
        result as u16
    }

    fn dec_16(&mut self, op: u16) -> u16 {
        self.clock_inc();
        let result = (op as i32) - 1;
        result as u16
    }

    fn daa(&mut self) {
        let mut result = (self.a as u16) as i16;
        if self.flags.contains(CPUFlags::NEG) {
            // If subtract just happened:
            if self.flags.contains(CPUFlags::CARRY) {
                result -= 0x60;
            }
            if self.flags.contains(CPUFlags::HC) {
                result -= 0x06;
            }
        } else {
            // If add just happened:
            if self.flags.contains(CPUFlags::CARRY) || result > 0x99 {
                result += 0x60;
                self.flags.insert(CPUFlags::CARRY);
            }
            if self.flags.contains(CPUFlags::HC) || (result & 0xF) > 0x9 {
                result += 0x6;
            }
        }

        self.flags.remove(CPUFlags::HC);
        self.a = (result & 0xFF) as u8;
        self.flags.set(CPUFlags::ZERO, self.a == 0);
    }

    fn cpl(&mut self) {
        self.flags.insert(CPUFlags::NEG | CPUFlags::HC);
        self.a = self.a ^ 0xFF;
    }

    // Stack
    fn pop(&mut self, which: Reg) {
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
        self.clock_inc();
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
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::CARRY, top_bit != 0);
        self.a = (self.a << 1) | top_bit;
    }

    fn rla(&mut self) {
        let carry_bit = if self.flags.contains(CPUFlags::CARRY) {1} else {0};
        let top_bit = (self.a >> 7) & 1;
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::CARRY, top_bit != 0);
        self.a = (self.a << 1) | carry_bit;
    }

    fn rrca(&mut self) {
        let bot_bit = (self.a << 7) & 0x80;
        self.flags = CPUFlags::default();
        self.flags.set(CPUFlags::CARRY, bot_bit != 0);
        self.a = (self.a >> 1) | bot_bit;
    }

    fn rra(&mut self) {
        let carry_bit = if self.flags.contains(CPUFlags::CARRY) {0x80} else {0};
        let bot_bit = (self.a << 7) & 0x80;
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
        let result = op >> 1 | (op & 0x80);
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
            self.clock_inc();
            self.pc = loc
        }
    }

    fn jr(&mut self, cd: Cond, loc: i8) {
        if cd.check(&self) {
            self.clock_inc();
            self.pc = ((self.pc as i32) + (loc as i32)) as u16;
        }
    }

    fn call(&mut self, cd: Cond, loc: u16) {
        if cd.check(&self) {
            self.clock_inc();
            let hi_byte = (self.pc >> 8) as u8;
            let lo_byte = self.pc as u8;
            self.stack_push(hi_byte);
            self.stack_push(lo_byte);
            self.pc = loc;
        }
    }

    fn ret(&mut self, cd: Cond) {
        self.clock_inc();

        if cd.check(&self) {
            if cd != Cond::AL {
                self.clock_inc();
            }
            let lo_byte = self.stack_pop() as u16;
            let hi_byte = self.stack_pop() as u16;
            self.pc = (hi_byte << 8) | lo_byte;
        }
    }

    fn reti(&mut self) {
        self.clock_inc();

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