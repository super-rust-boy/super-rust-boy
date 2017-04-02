//mod cpu;
use mem::MemBus;
use common;
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

    // Stack Pointer & PC
    sp: u16,
    pc: u16,

    // Memory Bus (ROM,RAM,Peripherals etc)
    mem: MemBus,
}

// Internal enum for operands.
/*enum Operand<'a> {
    SReg(&'a mut u8),
    DReg(&'a mut u16),
    Mem(u16),
}*/


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
    fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }

    #[inline]
    fn set_hl(&mut self, val: u16) {
        //TODO: remove 0xFF?
        self.h = ((val >> 8) & 0xFF) as u8;
        self.l = (val & 0xFF) as u8;
    }

    #[inline]
    fn read_mem(&self, loc: u16) -> u8 {
        self.mem.read(loc)
    }

    #[inline]
    fn write_mem(&mut self, loc: u16, val: u8) {
        self.mem.write(loc, val);
    }

    // read and write to/from mem pointed to by hl
    fn read_hl(&self) -> u8 {
        let hl = self.get_hl();
        self.mem.read(hl)
    }

    fn write_hl(&mut self, val: u8) {
        let hl = self.get_hl();
        self.mem.write(hl, val);
    }

    // read mem pointed to by pc (and inc pc)
    fn fetch(&mut self) -> u8 {
        let result = self.mem.read(self.pc);
        self.pc += 1;
        result
    }
}

// Instructions
impl CPU {
    // Arithmetic
    fn add(&mut self, carry: bool, read: &Fn(&CPU)->u8) {
        let c = if self.f_c && carry {1} else {0};
        let result = (self.a as u16) + (read(&self) as u16) + c;
        self.f_z = if (result & 0xFF) == 0 {true} else {false};
        self.f_n = false;
        self.f_h = if result > 0xF {true} else {false};
        self.f_c = if result > 0xFF {true} else {false};
        self.a = result as u8;
    }

    fn add_16(&mut self, read: &Fn(&CPU)->u16) {
        let result = (self.get_hl() as u32) + (read(&self) as u32);
        self.f_n = false;
        self.f_h = if result > 0xF {true} else {false};
        self.f_c = if result > 0xFFFF {true} else {false};
        self.set_hl(result as u16);
    }

    fn sub(&mut self, carry: bool, read: &Fn(&CPU)->u8) {
        let c = if self.f_c && carry {1} else {0};
        let result = (self.a as i16) - (read(&self) as i16) - c;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = true;
        self.f_h = if result < 0x10 {true} else {false};
        self.f_c = if result < 0 {true} else {false};
        self.a = result as u8;
    }

    fn and(&mut self, read: &Fn(&CPU)->u8) {
        let result = self.a & read(&self);
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = true;
        self.f_c = false;
        self.a = result;
    }

    fn xor(&mut self, read: &Fn(&CPU)->u8) {
        let result = self.a ^ read(&self);
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = false;
        self.a = result;
    }

    fn or(&mut self, read: &Fn(&CPU)->u8) {
        let result = self.a | read(&self);
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = false;
        self.a = result;
    }

    fn cp(&mut self, read: &Fn(&CPU)->u8) {
        let result = (self.a as i16) - (read(&self) as i16);
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

    // TODO: improve this
    fn daa(&mut self) {
        let lo_nib = (self.a & 0xF) as u16;
        let hi_nib = (self.a & 0xF0) as u16;
        let lo_inc = match (lo_nib, self.f_n, self.f_h) {
            // TODO: improve matches
            (x @ 10...15,false,false) => 0x06,
            (x @ 0...3,false,true) => 0x06,
            (x @ 6...15,true,true) => 0x0A,
            _ => 0x00,
        };
        let hi_inc = match (hi_nib, self.f_c, self.f_n, self.f_h) {
            // TODO: improve matches
            (x @ 10...15,false,false,_) => 0x60,
            (x @ 9...15,false,false,false) if lo_inc==6 => 0x60,
            (x @ 0...2,true,false,false) => 0x60,
            (x @ 0...3,true,false,true) => 0x60,
            (x @ 0...8,false,true,true) => 0xF0,
            (x @ 7...15,true,true,false) => 0xA0,
            (x @ 6...15,true,true,true) => 0x90,
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

    fn rlc(&mut self, read: &Fn(&CPU)->u8, write: &Fn(&mut CPU, u8)) {
        let op = read(&self);
        let top_bit = (op >> 7) & 1;
        let result = (op << 1) | top_bit;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = if top_bit != 0 {true} else {false};
        write(self, result);
    }

    fn rl(&mut self, read: &Fn(&CPU)->u8, write: &Fn(&mut CPU, u8)) {
        let op = read(&self);
        let carry_bit = if self.f_c {1} else {0};
        let top_bit = (op >> 7) & 1;
        let result = (op << 1) | carry_bit;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = if top_bit != 0 {true} else {false};
        write(self, result);
    }

    fn rrc(&mut self, read: &Fn(&CPU)->u8, write: &Fn(&mut CPU, u8)) {
        let op = read(&self);
        let bot_bit = (op << 7) & 0x80;
        let result = (op >> 1) | bot_bit;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = if bot_bit != 0 {true} else {false};
        write(self, result);
    }

    fn rr(&mut self, read: &Fn(&CPU)->u8, write: &Fn(&mut CPU, u8)) {
        let op = read(&self);
        let carry_bit = if self.f_c {0x80} else {0};
        let bot_bit = (op << 7) & 0x80;
        let result = (op >> 1) | carry_bit;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = if bot_bit != 0 {true} else {false};
        write(self, result);
    }

    fn sla(&mut self, read: &Fn(&CPU)->u8, write: &Fn(&mut CPU, u8)) {
        let op = read(&self);
        self.f_c = if (op & 0x80) != 0 {true} else {false};
        let result = op << 1;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        write(self, result);
    }

    fn sra(&mut self, read: &Fn(&CPU)->u8, write: &Fn(&mut CPU, u8)) {
        let op = read(&self) as i8;
        self.f_c = if (op & 0x1) != 0 {true} else {false};
        let result = op >> 1;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        write(self, result as u8);
    }

    fn srl(&mut self, read: &Fn(&CPU)->u8, write: &Fn(&mut CPU, u8)) {
        let op = read(&self);
        self.f_c = if (op & 0x1) != 0 {true} else {false};
        let result = op >> 1;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        write(self, result);
    }

    fn swap(&mut self, read: &Fn(&CPU)->u8, write: &Fn(&mut CPU, u8)) {
        let op = read(&self);
        let result = (op << 4) | (op >> 4);
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = false;
        write(self, result);
    }

    // Bit setting & testing
    fn set(&mut self, b: u8, read: &Fn(&CPU)->u8, write: &Fn(&mut CPU, u8)) {
        let op = read(&self);
        let result = op | (1 << b);
        write(self, result);
    }

    fn res(&mut self, b: u8, read: &Fn(&CPU)->u8, write: &Fn(&mut CPU, u8)) {
        let op = read(&self);
        let result = op & !(1 << b);
        write(self, result);
    }

    fn bit(&mut self, b: u8, read: &Fn(&CPU)->u8) {
        let op = read(&self);
        self.f_z = if (op & (1 << b)) == 0 {true} else {false};
        self.f_n = false;
        self.f_h = true;
    }


    // Other
    fn nop(&self) {
        return;
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
            sp: 0,
            pc: 0x100,
            mem: MemBus::new(),
        }
    }

    // Single instruction
    pub fn step(&mut self) {
        // read PC
        let instr = self.fetch();
        // call function - TODO match
        // TODO: add_16

        match instr {
            0x00 => self.nop(),
            0x04 => self.inc(&|ref s| s.b, &|ref mut s,i| s.b=i),
            0x05 => self.dec(&|ref s| s.b, &|ref mut s,i| s.b=i),
            0x07 => self.rlca(),
            0x0C => self.inc(&|ref s| s.c, &|ref mut s,i| s.c=i),
            0x0D => self.dec(&|ref s| s.c, &|ref mut s,i| s.c=i),
            0x0F => self.rrca(),

            0x14 => self.inc(&|ref s| s.d, &|ref mut s,i| s.d=i),
            0x15 => self.dec(&|ref s| s.d, &|ref mut s,i| s.d=i),
            0x17 => self.rla(),
            0x1C => self.inc(&|ref s| s.e, &|ref mut s,i| s.e=i),
            0x1D => self.dec(&|ref s| s.e, &|ref mut s,i| s.e=i),
            0x1F => self.rra(),

            0x24 => self.inc(&|ref s| s.h, &|ref mut s,i| s.h=i),
            0x25 => self.dec(&|ref s| s.h, &|ref mut s,i| s.h=i),
            0x27 => self.daa(),
            0x2C => self.inc(&|ref s| s.l, &|ref mut s,i| s.l=i),
            0x2D => self.dec(&|ref s| s.l, &|ref mut s,i| s.l=i),
            0x2F => self.cpl(),

            0x34 => self.inc(&|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0x35 => self.dec(&|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0x3C => self.inc(&|ref s| s.a, &|ref mut s,i| s.a=i),
            0x3D => self.dec(&|ref s| s.a, &|ref mut s,i| s.a=i),

            0x80 => self.add(false, &|ref s| s.b),
            0x81 => self.add(false, &|ref s| s.c),
            0x82 => self.add(false, &|ref s| s.d),
            0x83 => self.add(false, &|ref s| s.e),
            0x84 => self.add(false, &|ref s| s.h),
            0x85 => self.add(false, &|ref s| s.l),
            0x86 => self.add(false, &|ref s| s.read_hl()),
            0x87 => self.add(false, &|ref s| s.a),
            0x88 => self.add(true, &|ref s| s.b),
            0x89 => self.add(true, &|ref s| s.c),
            0x8A => self.add(true, &|ref s| s.d),
            0x8B => self.add(true, &|ref s| s.e),
            0x8C => self.add(true, &|ref s| s.h),
            0x8D => self.add(true, &|ref s| s.l),
            0x8E => self.add(true, &|ref s| s.read_hl()),
            0x8F => self.add(true, &|ref s| s.a),

            0x90 => self.sub(false, &|ref s| s.b),
            0x91 => self.sub(false, &|ref s| s.c),
            0x92 => self.sub(false, &|ref s| s.d),
            0x93 => self.sub(false, &|ref s| s.e),
            0x94 => self.sub(false, &|ref s| s.h),
            0x95 => self.sub(false, &|ref s| s.l),
            0x96 => self.sub(false, &|ref s| s.read_hl()),
            0x97 => self.sub(false, &|ref s| s.a),
            0x98 => self.sub(true, &|ref s| s.b),
            0x99 => self.sub(true, &|ref s| s.c),
            0x9A => self.sub(true, &|ref s| s.d),
            0x9B => self.sub(true, &|ref s| s.e),
            0x9C => self.sub(true, &|ref s| s.h),
            0x9D => self.sub(true, &|ref s| s.l),
            0x9E => self.sub(true, &|ref s| s.read_hl()),
            0x9F => self.sub(true, &|ref s| s.a),

            0xA0 => self.and(&|ref s| s.b),
            0xA1 => self.and(&|ref s| s.c),
            0xA2 => self.and(&|ref s| s.d),
            0xA3 => self.and(&|ref s| s.e),
            0xA4 => self.and(&|ref s| s.h),
            0xA5 => self.and(&|ref s| s.l),
            0xA6 => self.and(&|ref s| s.read_hl()),
            0xA7 => self.and(&|ref s| s.a),
            0xA8 => self.xor(&|ref s| s.b),
            0xA9 => self.xor(&|ref s| s.c),
            0xAA => self.xor(&|ref s| s.d),
            0xAB => self.xor(&|ref s| s.e),
            0xAC => self.xor(&|ref s| s.h),
            0xAD => self.xor(&|ref s| s.l),
            0xAE => self.xor(&|ref s| s.read_hl()),
            0xAF => self.xor(&|ref s| s.a),

            0xB0 => self.or(&|ref s| s.b),
            0xB1 => self.or(&|ref s| s.c),
            0xB2 => self.or(&|ref s| s.d),
            0xB3 => self.or(&|ref s| s.e),
            0xB4 => self.or(&|ref s| s.h),
            0xB5 => self.or(&|ref s| s.l),
            0xB6 => self.or(&|ref s| s.read_hl()),
            0xB7 => self.or(&|ref s| s.a),
            0xB8 => self.cp(&|ref s| s.b),
            0xB9 => self.cp(&|ref s| s.c),
            0xBA => self.cp(&|ref s| s.d),
            0xBB => self.cp(&|ref s| s.e),
            0xBC => self.cp(&|ref s| s.h),
            0xBD => self.cp(&|ref s| s.l),
            0xBE => self.cp(&|ref s| s.read_hl()),
            0xBF => self.cp(&|ref s| s.a),

            0xC6 => {let imm = self.fetch(); self.add(false, &|ref s| imm)},
            0xCB => {let ins = self.fetch(); self.prefix_cb(ins)},
            0xCE => {let imm = self.fetch(); self.add(true, &|ref s| imm)},

            0xD6 => {let imm = self.fetch(); self.sub(false, &|ref s| imm)},
            0xDE => {let imm = self.fetch(); self.sub(true, &|ref s| imm)},

            0xE6 => {let imm = self.fetch(); self.and(&|ref s| imm)},
            0xEE => {let imm = self.fetch(); self.xor(&|ref s| imm)},

            0xF6 => {let imm = self.fetch(); self.or(&|ref s| imm)},
            0xFE => {let imm = self.fetch(); self.cp(&|ref s| imm)},

            _ => self.add(false, &|ref s| s.c),
        }
        // increment PC
    }


    fn prefix_cb(&mut self, instr: u8) {
        match instr {
            0x00 => self.rlc(&|ref s| s.b, &|ref mut s,i| s.b=i),
            0x01 => self.rlc(&|ref s| s.c, &|ref mut s,i| s.c=i),
            0x02 => self.rlc(&|ref s| s.d, &|ref mut s,i| s.d=i),
            0x03 => self.rlc(&|ref s| s.e, &|ref mut s,i| s.e=i),
            0x04 => self.rlc(&|ref s| s.h, &|ref mut s,i| s.h=i),
            0x05 => self.rlc(&|ref s| s.l, &|ref mut s,i| s.l=i),
            0x06 => self.rlc(&|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0x07 => self.rlc(&|ref s| s.a, &|ref mut s,i| s.a=i),
            0x08 => self.rrc(&|ref s| s.b, &|ref mut s,i| s.b=i),
            0x09 => self.rrc(&|ref s| s.c, &|ref mut s,i| s.c=i),
            0x0A => self.rrc(&|ref s| s.d, &|ref mut s,i| s.d=i),
            0x0B => self.rrc(&|ref s| s.e, &|ref mut s,i| s.e=i),
            0x0C => self.rrc(&|ref s| s.h, &|ref mut s,i| s.h=i),
            0x0D => self.rrc(&|ref s| s.l, &|ref mut s,i| s.l=i),
            0x0E => self.rrc(&|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0x0F => self.rrc(&|ref s| s.a, &|ref mut s,i| s.a=i),
            0x10 => self.rl(&|ref s| s.b, &|ref mut s,i| s.b=i),
            0x11 => self.rl(&|ref s| s.c, &|ref mut s,i| s.c=i),
            0x12 => self.rl(&|ref s| s.d, &|ref mut s,i| s.d=i),
            0x13 => self.rl(&|ref s| s.e, &|ref mut s,i| s.e=i),
            0x14 => self.rl(&|ref s| s.h, &|ref mut s,i| s.h=i),
            0x15 => self.rl(&|ref s| s.l, &|ref mut s,i| s.l=i),
            0x16 => self.rl(&|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0x17 => self.rl(&|ref s| s.a, &|ref mut s,i| s.a=i),
            0x18 => self.rr(&|ref s| s.b, &|ref mut s,i| s.b=i),
            0x19 => self.rr(&|ref s| s.c, &|ref mut s,i| s.c=i),
            0x1A => self.rr(&|ref s| s.d, &|ref mut s,i| s.d=i),
            0x1B => self.rr(&|ref s| s.e, &|ref mut s,i| s.e=i),
            0x1C => self.rr(&|ref s| s.h, &|ref mut s,i| s.h=i),
            0x1D => self.rr(&|ref s| s.l, &|ref mut s,i| s.l=i),
            0x1E => self.rr(&|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0x1F => self.rr(&|ref s| s.a, &|ref mut s,i| s.a=i),
            0x20 => self.sla(&|ref s| s.b, &|ref mut s,i| s.b=i),
            0x21 => self.sla(&|ref s| s.c, &|ref mut s,i| s.c=i),
            0x22 => self.sla(&|ref s| s.d, &|ref mut s,i| s.d=i),
            0x23 => self.sla(&|ref s| s.e, &|ref mut s,i| s.e=i),
            0x24 => self.sla(&|ref s| s.h, &|ref mut s,i| s.h=i),
            0x25 => self.sla(&|ref s| s.l, &|ref mut s,i| s.l=i),
            0x26 => self.sla(&|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0x27 => self.sla(&|ref s| s.a, &|ref mut s,i| s.a=i),
            0x28 => self.sra(&|ref s| s.b, &|ref mut s,i| s.b=i),
            0x29 => self.sra(&|ref s| s.c, &|ref mut s,i| s.c=i),
            0x2A => self.sra(&|ref s| s.d, &|ref mut s,i| s.d=i),
            0x2B => self.sra(&|ref s| s.e, &|ref mut s,i| s.e=i),
            0x2C => self.sra(&|ref s| s.h, &|ref mut s,i| s.h=i),
            0x2D => self.sra(&|ref s| s.l, &|ref mut s,i| s.l=i),
            0x2E => self.sra(&|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0x2F => self.sra(&|ref s| s.a, &|ref mut s,i| s.a=i),
            0x30 => self.swap(&|ref s| s.b, &|ref mut s,i| s.b=i),
            0x31 => self.swap(&|ref s| s.c, &|ref mut s,i| s.c=i),
            0x32 => self.swap(&|ref s| s.d, &|ref mut s,i| s.d=i),
            0x33 => self.swap(&|ref s| s.e, &|ref mut s,i| s.e=i),
            0x34 => self.swap(&|ref s| s.h, &|ref mut s,i| s.h=i),
            0x35 => self.swap(&|ref s| s.l, &|ref mut s,i| s.l=i),
            0x36 => self.swap(&|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0x37 => self.swap(&|ref s| s.a, &|ref mut s,i| s.a=i),
            0x38 => self.srl(&|ref s| s.b, &|ref mut s,i| s.b=i),
            0x39 => self.srl(&|ref s| s.c, &|ref mut s,i| s.c=i),
            0x3A => self.srl(&|ref s| s.d, &|ref mut s,i| s.d=i),
            0x3B => self.srl(&|ref s| s.e, &|ref mut s,i| s.e=i),
            0x3C => self.srl(&|ref s| s.h, &|ref mut s,i| s.h=i),
            0x3D => self.srl(&|ref s| s.l, &|ref mut s,i| s.l=i),
            0x3E => self.srl(&|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0x3F => self.srl(&|ref s| s.a, &|ref mut s,i| s.a=i),

            0x40 => self.bit(0, &|ref s| s.b),
            0x41 => self.bit(0, &|ref s| s.c),
            0x42 => self.bit(0, &|ref s| s.d),
            0x43 => self.bit(0, &|ref s| s.e),
            0x44 => self.bit(0, &|ref s| s.h),
            0x45 => self.bit(0, &|ref s| s.l),
            0x46 => self.bit(0, &|ref s| s.read_hl()),
            0x47 => self.bit(0, &|ref s| s.a),
            0x48 => self.bit(1, &|ref s| s.b),
            0x49 => self.bit(1, &|ref s| s.c),
            0x4A => self.bit(1, &|ref s| s.d),
            0x4B => self.bit(1, &|ref s| s.e),
            0x4C => self.bit(1, &|ref s| s.h),
            0x4D => self.bit(1, &|ref s| s.l),
            0x4E => self.bit(1, &|ref s| s.read_hl()),
            0x4F => self.bit(1, &|ref s| s.a),
            0x50 => self.bit(2, &|ref s| s.b),
            0x51 => self.bit(2, &|ref s| s.c),
            0x52 => self.bit(2, &|ref s| s.d),
            0x53 => self.bit(2, &|ref s| s.e),
            0x54 => self.bit(2, &|ref s| s.h),
            0x55 => self.bit(2, &|ref s| s.l),
            0x56 => self.bit(2, &|ref s| s.read_hl()),
            0x57 => self.bit(2, &|ref s| s.a),
            0x58 => self.bit(3, &|ref s| s.b),
            0x59 => self.bit(3, &|ref s| s.c),
            0x5A => self.bit(3, &|ref s| s.d),
            0x5B => self.bit(3, &|ref s| s.e),
            0x5C => self.bit(3, &|ref s| s.h),
            0x5D => self.bit(3, &|ref s| s.l),
            0x5E => self.bit(3, &|ref s| s.read_hl()),
            0x5F => self.bit(3, &|ref s| s.a),
            0x60 => self.bit(4, &|ref s| s.b),
            0x61 => self.bit(4, &|ref s| s.c),
            0x62 => self.bit(4, &|ref s| s.d),
            0x63 => self.bit(4, &|ref s| s.e),
            0x64 => self.bit(4, &|ref s| s.h),
            0x65 => self.bit(4, &|ref s| s.l),
            0x66 => self.bit(4, &|ref s| s.read_hl()),
            0x67 => self.bit(4, &|ref s| s.a),
            0x68 => self.bit(5, &|ref s| s.b),
            0x69 => self.bit(5, &|ref s| s.c),
            0x6A => self.bit(5, &|ref s| s.d),
            0x6B => self.bit(5, &|ref s| s.e),
            0x6C => self.bit(5, &|ref s| s.h),
            0x6D => self.bit(5, &|ref s| s.l),
            0x6E => self.bit(5, &|ref s| s.read_hl()),
            0x6F => self.bit(5, &|ref s| s.a),
            0x70 => self.bit(6, &|ref s| s.b),
            0x71 => self.bit(6, &|ref s| s.c),
            0x72 => self.bit(6, &|ref s| s.d),
            0x73 => self.bit(6, &|ref s| s.e),
            0x74 => self.bit(6, &|ref s| s.h),
            0x75 => self.bit(6, &|ref s| s.l),
            0x76 => self.bit(6, &|ref s| s.read_hl()),
            0x77 => self.bit(6, &|ref s| s.a),
            0x78 => self.bit(7, &|ref s| s.b),
            0x79 => self.bit(7, &|ref s| s.c),
            0x7A => self.bit(7, &|ref s| s.d),
            0x7B => self.bit(7, &|ref s| s.e),
            0x7C => self.bit(7, &|ref s| s.h),
            0x7D => self.bit(7, &|ref s| s.l),
            0x7E => self.bit(7, &|ref s| s.read_hl()),
            0x7F => self.bit(7, &|ref s| s.a),

            0x80 => self.res(0, &|ref s| s.b, &|ref mut s,i| s.b=i),
            0x81 => self.res(0, &|ref s| s.c, &|ref mut s,i| s.c=i),
            0x82 => self.res(0, &|ref s| s.d, &|ref mut s,i| s.d=i),
            0x83 => self.res(0, &|ref s| s.e, &|ref mut s,i| s.e=i),
            0x84 => self.res(0, &|ref s| s.h, &|ref mut s,i| s.h=i),
            0x85 => self.res(0, &|ref s| s.l, &|ref mut s,i| s.l=i),
            0x86 => self.res(0, &|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0x87 => self.res(0, &|ref s| s.a, &|ref mut s,i| s.a=i),
            0x88 => self.res(1, &|ref s| s.b, &|ref mut s,i| s.b=i),
            0x89 => self.res(1, &|ref s| s.c, &|ref mut s,i| s.c=i),
            0x8A => self.res(1, &|ref s| s.d, &|ref mut s,i| s.d=i),
            0x8B => self.res(1, &|ref s| s.e, &|ref mut s,i| s.e=i),
            0x8C => self.res(1, &|ref s| s.h, &|ref mut s,i| s.h=i),
            0x8D => self.res(1, &|ref s| s.l, &|ref mut s,i| s.l=i),
            0x8E => self.res(1, &|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0x8F => self.res(1, &|ref s| s.a, &|ref mut s,i| s.a=i),
            0x90 => self.res(2, &|ref s| s.b, &|ref mut s,i| s.b=i),
            0x91 => self.res(2, &|ref s| s.c, &|ref mut s,i| s.c=i),
            0x92 => self.res(2, &|ref s| s.d, &|ref mut s,i| s.d=i),
            0x93 => self.res(2, &|ref s| s.e, &|ref mut s,i| s.e=i),
            0x94 => self.res(2, &|ref s| s.h, &|ref mut s,i| s.h=i),
            0x95 => self.res(2, &|ref s| s.l, &|ref mut s,i| s.l=i),
            0x96 => self.res(2, &|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0x97 => self.res(2, &|ref s| s.a, &|ref mut s,i| s.a=i),
            0x98 => self.res(3, &|ref s| s.b, &|ref mut s,i| s.b=i),
            0x99 => self.res(3, &|ref s| s.c, &|ref mut s,i| s.c=i),
            0x9A => self.res(3, &|ref s| s.d, &|ref mut s,i| s.d=i),
            0x9B => self.res(3, &|ref s| s.e, &|ref mut s,i| s.e=i),
            0x9C => self.res(3, &|ref s| s.h, &|ref mut s,i| s.h=i),
            0x9D => self.res(3, &|ref s| s.l, &|ref mut s,i| s.l=i),
            0x9E => self.res(3, &|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0x9F => self.res(3, &|ref s| s.a, &|ref mut s,i| s.a=i),
            0xA0 => self.res(4, &|ref s| s.b, &|ref mut s,i| s.b=i),
            0xA1 => self.res(4, &|ref s| s.c, &|ref mut s,i| s.c=i),
            0xA2 => self.res(4, &|ref s| s.d, &|ref mut s,i| s.d=i),
            0xA3 => self.res(4, &|ref s| s.e, &|ref mut s,i| s.e=i),
            0xA4 => self.res(4, &|ref s| s.h, &|ref mut s,i| s.h=i),
            0xA5 => self.res(4, &|ref s| s.l, &|ref mut s,i| s.l=i),
            0xA6 => self.res(4, &|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0xA7 => self.res(4, &|ref s| s.a, &|ref mut s,i| s.a=i),
            0xA8 => self.res(5, &|ref s| s.b, &|ref mut s,i| s.b=i),
            0xA9 => self.res(5, &|ref s| s.c, &|ref mut s,i| s.c=i),
            0xAA => self.res(5, &|ref s| s.d, &|ref mut s,i| s.d=i),
            0xAB => self.res(5, &|ref s| s.e, &|ref mut s,i| s.e=i),
            0xAC => self.res(5, &|ref s| s.h, &|ref mut s,i| s.h=i),
            0xAD => self.res(5, &|ref s| s.l, &|ref mut s,i| s.l=i),
            0xAE => self.res(5, &|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0xAF => self.res(5, &|ref s| s.a, &|ref mut s,i| s.a=i),
            0xB0 => self.res(6, &|ref s| s.b, &|ref mut s,i| s.b=i),
            0xB1 => self.res(6, &|ref s| s.c, &|ref mut s,i| s.c=i),
            0xB2 => self.res(6, &|ref s| s.d, &|ref mut s,i| s.d=i),
            0xB3 => self.res(6, &|ref s| s.e, &|ref mut s,i| s.e=i),
            0xB4 => self.res(6, &|ref s| s.h, &|ref mut s,i| s.h=i),
            0xB5 => self.res(6, &|ref s| s.l, &|ref mut s,i| s.l=i),
            0xB6 => self.res(6, &|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0xB7 => self.res(6, &|ref s| s.a, &|ref mut s,i| s.a=i),
            0xB8 => self.res(7, &|ref s| s.b, &|ref mut s,i| s.b=i),
            0xB9 => self.res(7, &|ref s| s.c, &|ref mut s,i| s.c=i),
            0xBA => self.res(7, &|ref s| s.d, &|ref mut s,i| s.d=i),
            0xBB => self.res(7, &|ref s| s.e, &|ref mut s,i| s.e=i),
            0xBC => self.res(7, &|ref s| s.h, &|ref mut s,i| s.h=i),
            0xBD => self.res(7, &|ref s| s.l, &|ref mut s,i| s.l=i),
            0xBE => self.res(7, &|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0xBF => self.res(7, &|ref s| s.a, &|ref mut s,i| s.a=i),

            0xC0 => self.set(0, &|ref s| s.b, &|ref mut s,i| s.b=i),
            0xC1 => self.set(0, &|ref s| s.c, &|ref mut s,i| s.c=i),
            0xC2 => self.set(0, &|ref s| s.d, &|ref mut s,i| s.d=i),
            0xC3 => self.set(0, &|ref s| s.e, &|ref mut s,i| s.e=i),
            0xC4 => self.set(0, &|ref s| s.h, &|ref mut s,i| s.h=i),
            0xC5 => self.set(0, &|ref s| s.l, &|ref mut s,i| s.l=i),
            0xC6 => self.set(0, &|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0xC7 => self.set(0, &|ref s| s.a, &|ref mut s,i| s.a=i),
            0xC8 => self.set(1, &|ref s| s.b, &|ref mut s,i| s.b=i),
            0xC9 => self.set(1, &|ref s| s.c, &|ref mut s,i| s.c=i),
            0xCA => self.set(1, &|ref s| s.d, &|ref mut s,i| s.d=i),
            0xCB => self.set(1, &|ref s| s.e, &|ref mut s,i| s.e=i),
            0xCC => self.set(1, &|ref s| s.h, &|ref mut s,i| s.h=i),
            0xCD => self.set(1, &|ref s| s.l, &|ref mut s,i| s.l=i),
            0xCE => self.set(1, &|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0xCF => self.set(1, &|ref s| s.a, &|ref mut s,i| s.a=i),
            0xD0 => self.set(2, &|ref s| s.b, &|ref mut s,i| s.b=i),
            0xD1 => self.set(2, &|ref s| s.c, &|ref mut s,i| s.c=i),
            0xD2 => self.set(2, &|ref s| s.d, &|ref mut s,i| s.d=i),
            0xD3 => self.set(2, &|ref s| s.e, &|ref mut s,i| s.e=i),
            0xD4 => self.set(2, &|ref s| s.h, &|ref mut s,i| s.h=i),
            0xD5 => self.set(2, &|ref s| s.l, &|ref mut s,i| s.l=i),
            0xD6 => self.set(2, &|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0xD7 => self.set(2, &|ref s| s.a, &|ref mut s,i| s.a=i),
            0xD8 => self.set(3, &|ref s| s.b, &|ref mut s,i| s.b=i),
            0xD9 => self.set(3, &|ref s| s.c, &|ref mut s,i| s.c=i),
            0xDA => self.set(3, &|ref s| s.d, &|ref mut s,i| s.d=i),
            0xDB => self.set(3, &|ref s| s.e, &|ref mut s,i| s.e=i),
            0xDC => self.set(3, &|ref s| s.h, &|ref mut s,i| s.h=i),
            0xDD => self.set(3, &|ref s| s.l, &|ref mut s,i| s.l=i),
            0xDE => self.set(3, &|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0xDF => self.set(3, &|ref s| s.a, &|ref mut s,i| s.a=i),
            0xE0 => self.set(4, &|ref s| s.b, &|ref mut s,i| s.b=i),
            0xE1 => self.set(4, &|ref s| s.c, &|ref mut s,i| s.c=i),
            0xE2 => self.set(4, &|ref s| s.d, &|ref mut s,i| s.d=i),
            0xE3 => self.set(4, &|ref s| s.e, &|ref mut s,i| s.e=i),
            0xE4 => self.set(4, &|ref s| s.h, &|ref mut s,i| s.h=i),
            0xE5 => self.set(4, &|ref s| s.l, &|ref mut s,i| s.l=i),
            0xE6 => self.set(4, &|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0xE7 => self.set(4, &|ref s| s.a, &|ref mut s,i| s.a=i),
            0xE8 => self.set(5, &|ref s| s.b, &|ref mut s,i| s.b=i),
            0xE9 => self.set(5, &|ref s| s.c, &|ref mut s,i| s.c=i),
            0xEA => self.set(5, &|ref s| s.d, &|ref mut s,i| s.d=i),
            0xEB => self.set(5, &|ref s| s.e, &|ref mut s,i| s.e=i),
            0xEC => self.set(5, &|ref s| s.h, &|ref mut s,i| s.h=i),
            0xED => self.set(5, &|ref s| s.l, &|ref mut s,i| s.l=i),
            0xEE => self.set(5, &|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0xEF => self.set(5, &|ref s| s.a, &|ref mut s,i| s.a=i),
            0xF0 => self.set(6, &|ref s| s.b, &|ref mut s,i| s.b=i),
            0xF1 => self.set(6, &|ref s| s.c, &|ref mut s,i| s.c=i),
            0xF2 => self.set(6, &|ref s| s.d, &|ref mut s,i| s.d=i),
            0xF3 => self.set(6, &|ref s| s.e, &|ref mut s,i| s.e=i),
            0xF4 => self.set(6, &|ref s| s.h, &|ref mut s,i| s.h=i),
            0xF5 => self.set(6, &|ref s| s.l, &|ref mut s,i| s.l=i),
            0xF6 => self.set(6, &|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
            0xF7 => self.set(6, &|ref s| s.a, &|ref mut s,i| s.a=i),
            0xF8 => self.set(7, &|ref s| s.b, &|ref mut s,i| s.b=i),
            0xF9 => self.set(7, &|ref s| s.c, &|ref mut s,i| s.c=i),
            0xFA => self.set(7, &|ref s| s.d, &|ref mut s,i| s.d=i),
            0xFB => self.set(7, &|ref s| s.e, &|ref mut s,i| s.e=i),
            0xFC => self.set(7, &|ref s| s.h, &|ref mut s,i| s.h=i),
            0xFD => self.set(7, &|ref s| s.l, &|ref mut s,i| s.l=i),
            0xFE => self.set(7, &|ref s| s.read_hl(), &|ref mut s,i| s.write_hl(i)),
       /*0xFF*/_ => self.set(7, &|ref s| s.a, &|ref mut s,i| s.a=i),
        }
    }
}
