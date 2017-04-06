// CPU Module

use mem::MemBus;
use std::collections::HashMap;

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
    f_z: bool,
    f_n: bool,
    f_h: bool,
    f_c: bool,

    // Interrupts
    ime: bool,

    // Stack Pointer & PC
    sp: u16,
    pc: u16,

    // Memory Bus (ROM,RAM,Peripherals etc)
    mem: MemBus,
}


// Conditions for Jump
enum Cond {
    NZ,
    NC,
    Z,
    C,
    AL,
}

// Double registers
enum Reg {
    AF,
    BC,
    DE,
    HL,
}


// Internal
impl CPU {
    // Special access registers
    fn get_f(&self) -> u8 {
        let z = if self.f_z {0b10000000} else {0};
        let n = if self.f_n {0b01000000} else {0};
        let h = if self.f_h {0b00100000} else {0};
        let c = if self.f_c {0b00010000} else {0};
        z | n | h | c
    }

    fn set_f(&mut self, f: u8) {
        self.f_z = (f & 0b10000000) != 0;
        self.f_n = (f & 0b01000000) != 0;
        self.f_h = (f & 0b00100000) != 0;
        self.f_c = (f & 0b00010000) != 0;
    }

    #[inline]
    fn get_16(&self, which: Reg) -> u16 {
        match which {
            Reg::HL => ((self.h as u16) << 8) | (self.l as u16),
            Reg::AF => ((self.a as u16) << 8) | (self.get_f() as u16),
            Reg::BC => ((self.b as u16) << 8) | (self.b as u16),
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
    fn read_mem(&self, loc: u16) -> u8 {
        self.mem.read(loc)
    }

    #[inline]
    fn write_mem(&mut self, loc: u16, val: u8) {
        self.mem.write(loc, val);
    }

    // read mem pointed to by pc (and inc pc)
    fn fetch(&mut self) -> u8 {
        let result = self.mem.read(self.pc);
        self.pc += 1;
        result
    }

    fn fetch_16(&mut self) -> u16 {
        let result = (self.mem.read(self.pc) as u16)
                     | ((self.mem.read(self.pc + 1) as u16) << 8);
        self.pc += 2;
        result
    }

    // read and write to/from mem pointed to by hl
    fn read_hl(&self) -> u8 {
        let hl = self.get_16(Reg::HL);
        self.mem.read(hl)
    }

    fn write_hl(&mut self, val: u8) {
        let hl = self.get_16(Reg::HL);
        self.mem.write(hl, val);
    }

    // increments sp - TODO: maybe improve this fn
    fn add_sp(&mut self, imm: u8) -> u16 {
        let offset = imm as i8;
        let result = (self.sp as i32) + (offset as i32);
        self.f_z = false;
        self.f_n = false;
        self.f_h = if result > 0xF {true} else {false};
        self.f_c = if result > 0xFFFF {true} else {false};
        result as u16
    }
}

// Instructions
impl CPU {
    // Arithmetic
    fn add(&mut self, carry: bool, op: u8) {
        let c = if self.f_c && carry {1} else {0};
        let result = (self.a as u16) + (op as u16) + c;
        self.f_z = if (result & 0xFF) == 0 {true} else {false};
        self.f_n = false;
        self.f_h = if result > 0xF {true} else {false};
        self.f_c = if result > 0xFF {true} else {false};
        self.a = result as u8;
    }

    fn add_16(&mut self, op: u16) {
        let result = (self.get_16(Reg::HL) as u32) + (op as u32);
        self.f_n = false;
        self.f_h = if result > 0xF {true} else {false};
        self.f_c = if result > 0xFFFF {true} else {false};
        self.set_16(Reg::HL, result as u16);
    }

    fn sub(&mut self, carry: bool, op: u8) {
        let c = if self.f_c && carry {1} else {0};
        let result = (self.a as i16) - (op as i16) - c;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = true;
        self.f_h = if result < 0x10 {true} else {false};
        self.f_c = if result < 0 {true} else {false};
        self.a = result as u8;
    }

    fn and(&mut self, op: u8) {
        let result = self.a & op;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = true;
        self.f_c = false;
        self.a = result;
    }

    fn xor(&mut self, op: u8) {
        let result = self.a ^ op;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = false;
        self.a = result;
    }

    fn or(&mut self, op: u8) {
        let result = self.a | op;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = false;
        self.a = result;
    }

    fn cp(&mut self, op: u8) {
        let result = (self.a as i16) - (op as i16);
        self.f_z = if result == 0 {true} else {false};
        self.f_n = true;
        self.f_h = if result < 0x10 {true} else {false};
        self.f_c = if result < 0 {true} else {false};
        self.a = result as u8;
    }

    // inc/dec
    fn inc(&mut self, read: &Fn(&CPU)->u8, write: &Fn(&mut CPU, u8)) {
        let result = (read(&self) as u16) + 1;
        self.f_z = if (result & 0xFF) == 0 {true} else {false};
        self.f_n = false;
        self.f_h = if result > 0xF {true} else {false};
        write(self, result as u8);
    }

    fn dec(&mut self, read: &Fn(&CPU)->u8, write: &Fn(&mut CPU, u8)) {
        let result = (read(&self) as i16) - 1;
        self.f_z = if (result & 0xFF) == 0 {true} else {false};
        self.f_n = false;
        self.f_h = if result < 0x10 {true} else {false};
        write(self, result as u8);
    }

    fn inc_16(&mut self, read: &Fn(&CPU)->u16, write: &Fn(&mut CPU, u16)) {
        let result = (read(&self) as u32) + 1;
        write(self, result as u16);
    }

    fn dec_16(&mut self, read: &Fn(&CPU)->u16, write: &Fn(&mut CPU, u16)) {
        let result = (read(&self) as i32) - 1;
        write(self, result as u16);
    }

    // TODO: improve this
    fn daa(&mut self) {
        let lo_nib = (self.a & 0xF) as u16;
        let hi_nib = (self.a & 0xF0) as u16;
        let lo_inc = match (lo_nib, self.f_n, self.f_h) {
            // TODO: improve matches
            (10...15,false,false) => 0x06,
            (0...3,false,true) => 0x06,
            (6...15,true,true) => 0x0A,
            _ => 0x00,
        };
        let hi_inc = match (hi_nib, self.f_c, self.f_n, self.f_h) {
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
        self.f_z = if (result & 0xFF) == 0 {true} else {false};
        self.f_h = false;
        self.f_c = if result > 0xFF {true} else {false};
        self.a = (result & 0xFF) as u8;
    }

    fn cpl(&mut self) {
        self.f_n = true;
        self.f_h = true;
        self.a = self.a ^ 0xFF;
    }

    // Load
    fn ld(&mut self, read: &Fn(&CPU)->u8, write: &Fn(&mut CPU, u8)) {
        let data = read(self);
        write(self, data);
    }

    fn ld_16(&mut self, read: &Fn(&CPU)->u16, write: &Fn(&mut CPU, u16)) {
        let data = read(self);
        write(self, data);
    }

    fn pop(&mut self, which: Reg) {
        let hi_byte = self.mem.read(self.sp+1);
        let lo_byte = self.mem.read(self.sp+2);
        self.sp += 2;
        match which {
            Reg::AF => {self.a = hi_byte; self.set_f(lo_byte);},
            Reg::BC => {self.b = hi_byte; self.c = lo_byte;},
            Reg::DE => {self.d = hi_byte; self.e = lo_byte;},
            Reg::HL => {self.h = hi_byte; self.l = lo_byte;},
        }
    }

    fn push(&mut self, which: Reg) {
        let (lo_byte, hi_byte) = match which {
            Reg::AF => (self.a, self.get_f()),
            Reg::BC => (self.b, self.c),
            Reg::DE => (self.d, self.e),
            Reg::HL => (self.h, self.l),
        };
        self.mem.write(self.sp, lo_byte);
        self.mem.write(self.sp-1, hi_byte);
        self.sp -= 2;
    }


    // Shift/Rotate
    fn rlca(&mut self) {
        let top_bit = (self.a >> 7) & 1;
        // TODO: check if z is set false here
        self.f_z = false;
        self.f_n = false;
        self.f_h = false;
        self.f_c = if top_bit != 0 {true} else {false};
        self.a = (self.a << 1) | top_bit;
    }

    fn rla(&mut self) {
        let carry_bit = if self.f_c {1} else {0};
        let top_bit = (self.a >> 7) & 1;
        // TODO: check if z is set false here
        self.f_z = false;
        self.f_n = false;
        self.f_h = false;
        self.f_c = if top_bit != 0 {true} else {false};
        self.a = (self.a << 1) | carry_bit;
    }

    fn rrca(&mut self) {
        let bot_bit = (self.a << 7) & 0x80;
        // TODO: check if z is set false here
        self.f_z = false;
        self.f_n = false;
        self.f_h = false;
        self.f_c = if bot_bit != 0 {true} else {false};
        self.a = (self.a >> 1) | bot_bit;
    }

    fn rra(&mut self) {
        let carry_bit = if self.f_c {0x80} else {0};
        let bot_bit = (self.a << 7) & 0x80;
        // TODO: check if z is set false here
        self.f_z = false;
        self.f_n = false;
        self.f_h = false;
        self.f_c = if bot_bit != 0 {true} else {false};
        self.a = (self.a >> 1) | carry_bit;
    }

    fn rlc(&mut self, op: u8) -> Option<u8> {
        let top_bit = (op >> 7) & 1;
        let result = (op << 1) | top_bit;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = if top_bit != 0 {true} else {false};
        Some(result)
    }

    fn rl(&mut self, op: u8) -> Option<u8> {
        let carry_bit = if self.f_c {1} else {0};
        let top_bit = (op >> 7) & 1;
        let result = (op << 1) | carry_bit;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = if top_bit != 0 {true} else {false};
        Some(result)
    }

    fn rrc(&mut self, op: u8) -> Option<u8> {
        let bot_bit = (op << 7) & 0x80;
        let result = (op >> 1) | bot_bit;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = if bot_bit != 0 {true} else {false};
        Some(result)
    }

    fn rr(&mut self, op: u8) -> Option<u8> {
        let carry_bit = if self.f_c {0x80} else {0};
        let bot_bit = (op << 7) & 0x80;
        let result = (op >> 1) | carry_bit;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = if bot_bit != 0 {true} else {false};
        Some(result)
    }

    fn sla(&mut self, op: u8) -> Option<u8> {
        self.f_c = if (op & 0x80) != 0 {true} else {false};
        let result = op << 1;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        Some(result)
    }

    fn sra(&mut self, op: u8) -> Option<u8> {
        self.f_c = if (op & 0x1) != 0 {true} else {false};
        let result = op >> 1;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        Some(result)
    }

    fn srl(&mut self, op: u8) -> Option<u8> {
        self.f_c = if (op & 0x1) != 0 {true} else {false};
        let result = op >> 1;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        Some(result)
    }

    fn swap(&mut self, op: u8) -> Option<u8> {
        let result = (op << 4) | (op >> 4);
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = false;
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
        self.f_z = if (op & (1 << b)) == 0 {true} else {false};
        self.f_n = false;
        self.f_h = true;
        None
    }

    // Control commands
    fn scf(&mut self) {
        self.f_c = true;
        self.f_n = false;
        self.f_h = false;
    }

    fn ccf(&mut self) {
        self.f_c = if self.f_c {false} else {true};
        self.f_n = false;
        self.f_h = false;
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
        match cd {
            Cond::AL => self.pc = loc,
            Cond::NZ => if self.f_n && self.f_z {self.pc = loc},
            Cond::NC => if self.f_n && self.f_c {self.pc = loc},
            Cond::Z => if self.f_z {self.pc = loc},
            Cond::C => if self.f_c {self.pc = loc},
        }
    }

    fn jr(&mut self, cd: Cond, loc: i8) {
        match cd {
            Cond::AL => {},
            Cond::NZ => if !self.f_n || !self.f_z {return},
            Cond::NC => if !self.f_n || !self.f_c {return},
            Cond::Z => if !self.f_z {return},
            Cond::C => if !self.f_c {return},
        }
        self.pc = ((self.pc as i32) + (loc as i32)) as u16;
    }

    fn call(&mut self, cd: Cond, loc: u16) {
        match cd {
            Cond::AL => {},
            Cond::NZ => if !self.f_n || !self.f_z {return},
            Cond::NC => if !self.f_n || !self.f_c {return},
            Cond::Z => if !self.f_z {return},
            Cond::C => if !self.f_c {return},
        }
        self.mem.write(self.sp, self.pc as u8);
        self.mem.write(self.sp-1, (self.pc >> 8) as u8);
        self.sp -= 2;
        self.pc = loc;
    }

    fn ret(&mut self, cd: Cond) {
        match cd {
            Cond::AL => {},
            Cond::NZ => if !self.f_n || !self.f_z {return},
            Cond::NC => if !self.f_n || !self.f_c {return},
            Cond::Z => if !self.f_z {return},
            Cond::C => if !self.f_c {return},
        }
        let hi_byte = self.mem.read(self.sp+1) as u16;
        let lo_byte = self.mem.read(self.sp+2) as u16;
        self.sp += 2;
        self.pc = (hi_byte << 8) | lo_byte;
    }

    fn reti(&mut self) {
        self.ime = true;
        let hi_byte = self.mem.read(self.sp+1) as u16;
        let lo_byte = self.mem.read(self.sp+2) as u16;
        self.sp += 2;
        self.pc = (hi_byte << 8) | lo_byte;
    }

}


// Public interface
impl CPU {
    // Initialise CPU
    pub fn new() -> CPU {
        CPU {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            f_z: false,
            f_n: false,
            f_h: false,
            f_c: false,
            ime: true,
            sp: 0,
            pc: 0x100,
            mem: MemBus::new(),
        }
    }

    // Single instruction
    pub fn step(&mut self) {
        // TODO: check for interrupt? (should that happen here?)
        let instr = self.fetch();

        let op8 = match instr % 8 {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => self.read_hl(),
            _ => self.a,
        };

        let op16 = match (instr >> 4) % 2 {
            0 => self.get_16(Reg::BC),
            1 => self.get_16(Reg::DE),
            2 => self.get_16(Reg::HL),
            _ => self.sp,
        };

        match instr {
            0x00 => self.nop(),
            0x01 => {let imm = self.fetch_16();
                     self.ld_16(&|ref s| imm, &|ref mut s,i| s.set_16(Reg::BC,i))},
            0x02 => {let loc = self.get_16(Reg::BC);
                     self.ld(&|ref s| s.a, &|ref mut s,i| s.write_mem(loc,i))},
            0x03 => self.inc_16(&|ref s| s.get_16(Reg::BC), &|ref mut s,i| s.set_16(Reg::BC,i)),
            0x04 => self.inc(&|ref s| s.b, &|ref mut s,i| s.b=i),
            0x05 => self.dec(&|ref s| s.b, &|ref mut s,i| s.b=i),
            0x06 => {let imm = self.fetch(); self.ld(&|ref s| imm, &|ref mut s,i| s.b=i)},
            0x07 => self.rlca(),
            0x08 => {}, // LOAD 16-bits into mem (how??) /FOR BC:/ low byte(C) to n, high byte(B) to n+1 
            0x09 => self.add_16(op16),
            0x0A => {let loc = self.get_16(Reg::BC);
                     self.ld(&|ref s| s.read_mem(loc), &|ref mut s,i| s.a = i)},
            0x0B => self.dec_16(&|ref s| s.get_16(Reg::BC), &|ref mut s,i| s.set_16(Reg::BC,i)),
            0x0C => self.inc(&|ref s| s.c, &|ref mut s,i| s.c=i),
            0x0D => self.dec(&|ref s| s.c, &|ref mut s,i| s.c=i),
            0x0E => {let imm = self.fetch(); self.ld(&|ref s| imm, &|ref mut s,i| s.c=i)},
            0x0F => self.rrca(),

            0x10 => {}, // TODO stop
            0x11 => {let imm = self.fetch_16();
                     self.ld_16(&|ref s| imm, &|ref mut s,i| s.set_16(Reg::DE,i))},
            0x12 => {let loc = self.get_16(Reg::DE);
                     self.ld(&|ref s| s.a, &|ref mut s,i| s.write_mem(loc,i))},
            0x13 => self.inc_16(&|ref s| s.get_16(Reg::DE), &|ref mut s,i| s.set_16(Reg::DE,i)),
            0x14 => self.inc(&|ref s| s.d, &|ref mut s,i| s.d=i),
            0x15 => self.dec(&|ref s| s.d, &|ref mut s,i| s.d=i),
            0x16 => {let imm = self.fetch(); self.ld(&|ref s| imm, &|ref mut s,i| s.d=i)},
            0x17 => self.rla(),
            0x18 => {let imm = self.fetch(); self.jr(Cond::AL, imm as i8)},
            0x19 => self.add_16(op16),
            0x1A => {let loc = self.get_16(Reg::DE);
                     self.ld(&|ref s| s.read_mem(loc), &|ref mut s,i| s.a = i)},
            0x1B => self.dec_16(&|ref s| s.get_16(Reg::DE), &|ref mut s,i| s.set_16(Reg::DE,i)),
            0x1C => self.inc(&|ref s| s.e, &|ref mut s,i| s.e=i),
            0x1D => self.dec(&|ref s| s.e, &|ref mut s,i| s.e=i),
            0x1E => {let imm = self.fetch(); self.ld(&|ref s| imm, &|ref mut s,i| s.e=i)},
            0x1F => self.rra(),

            0x20 => {let imm = self.fetch(); self.jr(Cond::NZ, imm as i8)},
            0x21 => {let imm = self.fetch_16();
                     self.ld_16(&|ref s| imm, &|ref mut s,i| s.set_16(Reg::HL,i))},
            0x22 => {self.ld(&|ref s| s.a, &|ref mut s,i| s.write_hl(i));
                     self.inc_16(&|ref s| s.get_16(Reg::HL), &|ref mut s,i| s.set_16(Reg::HL,i))},
            0x23 => self.inc_16(&|ref s| s.get_16(Reg::HL), &|ref mut s,i| s.set_16(Reg::HL,i)),
            0x24 => self.inc(&|ref s| s.h, &|ref mut s,i| s.h=i),
            0x25 => self.dec(&|ref s| s.h, &|ref mut s,i| s.h=i),
            0x26 => {let imm = self.fetch(); self.ld(&|ref s| imm, &|ref mut s,i| s.h=i)},
            0x27 => self.daa(),
            0x28 => {let imm = self.fetch(); self.jr(Cond::Z, imm as i8)},
            0x29 => self.add_16(op16),
            0x2A => {self.ld(&|ref s| s.read_hl(), &|ref mut s,i| s.a=i);
                     self.inc_16(&|ref s| s.get_16(Reg::HL), &|ref mut s,i| s.set_16(Reg::HL,i))},
            0x2B => self.dec_16(&|ref s| s.get_16(Reg::HL), &|ref mut s,i| s.set_16(Reg::HL,i)),
            0x2C => self.inc(&|ref s| s.l, &|ref mut s,i| s.l=i),
            0x2D => self.dec(&|ref s| s.l, &|ref mut s,i| s.l=i),
            0x2E => {let imm = self.fetch(); self.ld(&|ref s| imm, &|ref mut s,i| s.l=i)},
            0x2F => self.cpl(),

            0x30 => {let imm = self.fetch(); self.jr(Cond::NC, imm as i8)},
            0x31 => {let imm = self.fetch_16(); self.ld_16(&|ref s| imm, &|ref mut s,i| s.sp = i)},
            0x32 => {self.ld(&|ref s| s.a, &|ref mut s,i| s.write_hl(i));
                     self.dec_16(&|ref s| s.get_16(Reg::HL), &|ref mut s,i| s.set_16(Reg::HL,i))},
            0x33 => self.inc_16(&|ref s| s.sp, &|ref mut s,i| s.sp=i),
            0x34 => self.inc(&|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0x35 => self.dec(&|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0x36 => {let imm = self.fetch(); self.ld(&|ref s| imm, &|ref mut s,i| s.write_hl(i))},
            0x37 => self.scf(),
            0x38 => {let imm = self.fetch(); self.jr(Cond::C, imm as i8)},
            0x39 => self.add_16(op16),
            0x3A => {self.ld(&|ref s| s.read_hl(), &|ref mut s,i| s.a=i);
                     self.dec_16(&|ref s| s.get_16(Reg::HL), &|ref mut s,i| s.set_16(Reg::HL,i))},
            0x3B => self.dec_16(&|ref s| s.sp, &|ref mut s,i| s.sp=i),
            0x3C => self.inc(&|ref s| s.a, &|ref mut s,i| s.a=i),
            0x3D => self.dec(&|ref s| s.a, &|ref mut s,i| s.a=i),
            0x3E => {let imm = self.fetch(); self.ld(&|ref s| imm, &|ref mut s,i| s.a=i)},
            0x3F => self.ccf(),

            0x40 => self.ld(&|ref s| s.b, &|ref mut s,i| s.b=i),
            0x41 => self.ld(&|ref s| s.c, &|ref mut s,i| s.b=i),
            0x42 => self.ld(&|ref s| s.d, &|ref mut s,i| s.b=i),
            0x43 => self.ld(&|ref s| s.e, &|ref mut s,i| s.b=i),
            0x44 => self.ld(&|ref s| s.h, &|ref mut s,i| s.b=i),
            0x45 => self.ld(&|ref s| s.l, &|ref mut s,i| s.b=i),
            0x46 => self.ld(&|ref s| s.read_hl(), &|ref mut s,i| s.b=i),
            0x47 => self.ld(&|ref s| s.a, &|ref mut s,i| s.b=i),
            0x48 => self.ld(&|ref s| s.b, &|ref mut s,i| s.c=i),
            0x49 => self.ld(&|ref s| s.c, &|ref mut s,i| s.c=i),
            0x4A => self.ld(&|ref s| s.d, &|ref mut s,i| s.c=i),
            0x4B => self.ld(&|ref s| s.e, &|ref mut s,i| s.c=i),
            0x4C => self.ld(&|ref s| s.h, &|ref mut s,i| s.c=i),
            0x4D => self.ld(&|ref s| s.l, &|ref mut s,i| s.c=i),
            0x4E => self.ld(&|ref s| s.read_hl(), &|ref mut s,i| s.c=i),
            0x4F => self.ld(&|ref s| s.a, &|ref mut s,i| s.c=i),

            0x50 => self.ld(&|ref s| s.b, &|ref mut s,i| s.d=i),
            0x51 => self.ld(&|ref s| s.c, &|ref mut s,i| s.d=i),
            0x52 => self.ld(&|ref s| s.d, &|ref mut s,i| s.d=i),
            0x53 => self.ld(&|ref s| s.e, &|ref mut s,i| s.d=i),
            0x54 => self.ld(&|ref s| s.h, &|ref mut s,i| s.d=i),
            0x55 => self.ld(&|ref s| s.l, &|ref mut s,i| s.d=i),
            0x56 => self.ld(&|ref s| s.read_hl(), &|ref mut s,i| s.d=i),
            0x57 => self.ld(&|ref s| s.a, &|ref mut s,i| s.d=i),
            0x58 => self.ld(&|ref s| s.b, &|ref mut s,i| s.e=i),
            0x59 => self.ld(&|ref s| s.c, &|ref mut s,i| s.e=i),
            0x5A => self.ld(&|ref s| s.d, &|ref mut s,i| s.e=i),
            0x5B => self.ld(&|ref s| s.e, &|ref mut s,i| s.e=i),
            0x5C => self.ld(&|ref s| s.h, &|ref mut s,i| s.e=i),
            0x5D => self.ld(&|ref s| s.l, &|ref mut s,i| s.e=i),
            0x5E => self.ld(&|ref s| s.read_hl(), &|ref mut s,i| s.e=i),
            0x5F => self.ld(&|ref s| s.a, &|ref mut s,i| s.e=i),

            0x60 => self.ld(&|ref s| s.b, &|ref mut s,i| s.h=i),
            0x61 => self.ld(&|ref s| s.c, &|ref mut s,i| s.h=i),
            0x62 => self.ld(&|ref s| s.d, &|ref mut s,i| s.h=i),
            0x63 => self.ld(&|ref s| s.e, &|ref mut s,i| s.h=i),
            0x64 => self.ld(&|ref s| s.h, &|ref mut s,i| s.h=i),
            0x65 => self.ld(&|ref s| s.l, &|ref mut s,i| s.h=i),
            0x66 => self.ld(&|ref s| s.read_hl(), &|ref mut s,i| s.h=i),
            0x67 => self.ld(&|ref s| s.a, &|ref mut s,i| s.h=i),
            0x68 => self.ld(&|ref s| s.b, &|ref mut s,i| s.l=i),
            0x69 => self.ld(&|ref s| s.c, &|ref mut s,i| s.l=i),
            0x6A => self.ld(&|ref s| s.d, &|ref mut s,i| s.l=i),
            0x6B => self.ld(&|ref s| s.e, &|ref mut s,i| s.l=i),
            0x6C => self.ld(&|ref s| s.h, &|ref mut s,i| s.l=i),
            0x6D => self.ld(&|ref s| s.l, &|ref mut s,i| s.l=i),
            0x6E => self.ld(&|ref s| s.read_hl(), &|ref mut s,i| s.l=i),
            0x6F => self.ld(&|ref s| s.a, &|ref mut s,i| s.l=i),

            0x70 => self.ld(&|ref s| s.b, &|ref mut s,i| s.write_hl(i)),
            0x71 => self.ld(&|ref s| s.c, &|ref mut s,i| s.write_hl(i)),
            0x72 => self.ld(&|ref s| s.d, &|ref mut s,i| s.write_hl(i)),
            0x73 => self.ld(&|ref s| s.e, &|ref mut s,i| s.write_hl(i)),
            0x74 => self.ld(&|ref s| s.h, &|ref mut s,i| s.write_hl(i)),
            0x75 => self.ld(&|ref s| s.l, &|ref mut s,i| s.write_hl(i)),
            0x76 => {}, // TODO halt
            0x77 => self.ld(&|ref s| s.a, &|ref mut s,i| s.write_hl(i)),
            0x78 => self.ld(&|ref s| s.b, &|ref mut s,i| s.a=i),
            0x79 => self.ld(&|ref s| s.c, &|ref mut s,i| s.a=i),
            0x7A => self.ld(&|ref s| s.d, &|ref mut s,i| s.a=i),
            0x7B => self.ld(&|ref s| s.e, &|ref mut s,i| s.a=i),
            0x7C => self.ld(&|ref s| s.h, &|ref mut s,i| s.a=i),
            0x7D => self.ld(&|ref s| s.l, &|ref mut s,i| s.a=i),
            0x7E => self.ld(&|ref s| s.read_hl(), &|ref mut s,i| s.a=i),
            0x7F => self.ld(&|ref s| s.a, &|ref mut s,i| s.a=i),

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
                                self.ld(&|ref s| s.a, &|ref mut s,i| s.write_mem(imm,i))},
            0xE1 => self.pop(Reg::HL),
            0xE2 => {let loc = (self.c as u16) + 0xFF00;
                                self.ld(&|ref s| s.a, &|ref mut s,i| s.write_mem(loc,i))},
            0xE5 => self.push(Reg::HL),
            0xE6 => {let imm = self.fetch(); self.and(imm)},
            0xE7 => self.call(Cond::AL, 0x20),
            0xE8 => {let imm = self.fetch(); self.sp = self.add_sp(imm)},
            0xE9 => {let loc = self.get_16(Reg::HL); self.jp(Cond::C, loc)},
            0xEA => {let loc = self.fetch_16();
                               self.ld(&|ref s| s.a, &|ref mut s,i| s.write_mem(loc,i))},
            0xEE => {let imm = self.fetch(); self.xor(imm)},
            0xEF => self.call(Cond::AL, 0x28),

            0xF0 => {let imm = (self.fetch() as u16) + 0xFF00;
                                self.ld(&|ref s| s.read_mem(imm), &|ref mut s,i| s.a=i)},
            0xF1 => self.pop(Reg::AF),
            0xF2 => {let imm = (self.c as u16) + 0xFF00;
                                self.ld(&|ref s| s.read_mem(imm), &|ref mut s,i| s.a=i)},
            0xF3 => self.di(),
            0xF5 => self.push(Reg::AF),
            0xF6 => {let imm = self.fetch(); self.or(imm)},
            0xF7 => self.call(Cond::AL, 0x30),
            0xF8 => {let imm = self.fetch(); let val = self.add_sp(imm); self.set_16(Reg::HL, val)},
            0xF9 => self.ld_16(&|ref s| s.get_16(Reg::HL), &|ref mut s,i| s.sp=i),
            0xFA => {let loc = self.fetch_16();
                               self.ld(&|ref s| s.read_mem(loc), &|ref mut s,i| s.a=i)},
            0xFB => self.ei(),
            0xFE => {let imm = self.fetch(); self.cp(imm)},
            0xFF => self.call(Cond::AL, 0x38),

            _ => {},
        }
    }


    fn prefix_cb(&mut self, instr: u8) {
        let op = match instr % 0x8 {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => self.read_hl(),
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
            (Some(x), 6) => self.write_hl(x),
            (Some(x), 7) => self.a = x,
            _ => {},
        }
    }
}
