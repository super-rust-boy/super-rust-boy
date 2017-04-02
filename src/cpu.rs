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
        return z | n | h | c;
    }

    fn set_f(&mut self, f: u8) {
        self.f_z = (f & 0b10000000) != 0;
        self.f_n = (f & 0b01000000) != 0;
        self.f_h = (f & 0b00100000) != 0;
        self.f_c = (f & 0b00010000) != 0;
    }

    #[inline]
    fn get_hl(&self) -> u16 {
        return ((self.h as u16) << 8) | (self.l as u16);
    }

    #[inline]
    fn set_hl(&mut self, val: u16) {
        //TODO: remove 0xFF?
        self.h = ((val >> 8) & 0xFF) as u8;
        self.l = (val & 0xFF) as u8;
    }

    // TODO: a more complex memory system
    #[inline]
    fn read_mem(&self, loc: u16) -> u8 {
        return self.mem.read(loc);
    }

    #[inline]
    fn write_mem(&mut self, loc: u16, val: u8) {
        self.mem.write(loc, val);
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

    // Single step
    pub fn step(&mut self) {
        // read PC
        let instr = self.read_mem(self.pc);
        // read any more bytes as needed

        // call function - TODO match
        match instr {
            0x04 => self.inc(&|ref s| s.b, &|ref mut s,i| s.b=i),
            0x80 => self.add(false, &|ref s| s.b),
            _ => self.add(false, &|ref s| s.c),
        }
        // increment PC
    }
}



