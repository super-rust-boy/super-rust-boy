//mod cpu;

// LR35902 CPU
pub struct CPU {
    a: u8,

    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,

    f_z: bool,
    f_n: bool,
    f_h: bool,
    f_c: bool,

    sp: u16,
    pc: u16,

    mem: Vec<u8>,
}


// Internal enum for operands.
enum Operand {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}


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

    #[inline]
    fn read_mem(&self, loc: u16) -> u8 {
        return self.mem[loc as usize];
    }

    #[inline]
    fn write_mem(&mut self, loc: u16, val: u8) {
        self.mem[loc as usize] = val;
    }
}




// Instructions
impl CPU {
    // Arithmetic
    fn add(&mut self, op2: u8, carry: bool) {
        let c = if self.f_c && carry {1} else {0};
        let result = (self.a as u16) + (op2 as u16) + c;
        self.f_z = if (result & 0xFF) == 0 {true} else {false};
        self.f_n = false;
        self.f_h = if result > 0xF {true} else {false};
        self.f_c = if result > 0xFF {true} else {false};
        //TODO: remove 0xFF?
        self.a = (result & 0xFF) as u8;
    }

    fn add_16(&mut self, op2: u16) {
        let result = (self.get_hl() as u32) + (op2 as u32);
        self.f_n = false;
        self.f_h = if result > 0xF {true} else {false};
        self.f_c = if result > 0xFFFF {true} else {false};
        //TODO: remove 0xFF?
        self.set_hl((result & 0xFFFF) as u16);
    }

    fn sub(&mut self, op2: u8, carry: bool) {
        let c = if self.f_c && carry {1} else {0};
        let result = (self.a as i16) - (op2 as i16) - c;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = true;
        self.f_h = if result < 0x10 {true} else {false};
        self.f_c = if result < 0 {true} else {false};
        //TODO: remove 0xFF?
        self.a = (result & 0xFF) as u8;
    }

    fn and(&mut self, op2: u8) {
        let result = self.a & op2;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = true;
        self.f_c = false;
        self.a = result;
    }

    fn xor(&mut self, op2: u8) {
        let result = self.a ^ op2;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = false;
        self.a = result;
    }

    fn or(&mut self, op2: u8) {
        let result = self.a | op2;
        self.f_z = if result == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = false;
        self.a = result;
    }

    fn cp(&mut self, op2: u8) {
        let result = (self.a as i16) - (op2 as i16);
        self.f_z = if result == 0 {true} else {false};
        self.f_n = true;
        self.f_h = if result < 0x10 {true} else {false};
        self.f_c = if result < 0 {true} else {false};
        //TODO: remove 0xFF?
        self.a = (result & 0xFF) as u8;
    }

    // TODO: inc/dec on regs

    fn inc_mem(&mut self, loc: u16) {
        let result = (self.read_mem(loc) as u16) + 1;
        self.f_z = if (result & 0xFF) == 0 {true} else {false};
        self.f_n = false;
        self.f_h = if result > 0xF {true} else {false};
        self.write_mem(loc, (result & 0xFF) as u8);
    }

    fn dec_mem(&mut self, loc: u16) {
        let result = (self.read_mem(loc) as i16) - 1;
        self.f_z = if (result & 0xFF) == 0 {true} else {false};
        self.f_n = false;
        self.f_h = if result < 0x10 {true} else {false};
        self.write_mem(loc, (result & 0xFF) as u8);
    }

    fn daa(&mut self) {
        let lo_nib = (self.a & 0xF as u16);
        let hi_nib = (self.a & 0xF0 as u16);
        let lo_inc = match (lo_nib, self.f_n, self.f_h) {
            (_ @ 10...15,false,false) => 0x06,
            (_ @ 0...3,false,true) => 0x06,
            (_ @ 6...15,true,true) => 0x0A,
            _ => 0x00,
        }
        let hi_inc = match (hi_nib, self.f_c, self.f_n, self.f_h) {
            (_ @ 10...15,false,false,_) => 0x60,
            (_ @ 9...15,false,false,false) if lo_inc==6 => 0x60,
            (_ @ 0..2,true,false,false) => 0x60,
            (_ @ 0..3,true,false,true) => 0x60,
            (_ @ 0..8,false,true,true) => 0xF0,
            (_ @ 7..15,true,true,false) => 0xA0,
            (_ @ 6..15,true,true,true) => 0x90,
        }
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

    fn rlc(&mut self, reg_op: Operand) {
        let reg = match reg_op {
            A => &mut self.a,
            B => &mut self.b,
            C => &mut self.c,
            D => &mut self.d,
            E => &mut self.e,
            H => &mut self.h,
            L => &mut self.l,
        };
        let top_bit = (*reg >> 7) & 1;
        *reg = (*reg << 1) | top_bit;
        self.f_z = if *reg == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = if top_bit != 0 {true} else {false};
    }

    fn rl(&mut self, reg_op: Operand) {
        let reg = match reg_op {
            A => &mut self.a,
            B => &mut self.b,
            C => &mut self.c,
            D => &mut self.d,
            E => &mut self.e,
            H => &mut self.h,
            L => &mut self.l,
        };
        let carry_bit = if self.f_c {1} else {0};
        let top_bit = (*reg >> 7) & 1;
        *reg = (*reg << 1) | carry_bit;
        self.f_z = if *reg == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = if top_bit != 0 {true} else {false};
    }

    fn rrc(&mut self, reg_op: Operand) {
        let reg = match reg_op {
            A => &mut self.a,
            B => &mut self.b,
            C => &mut self.c,
            D => &mut self.d,
            E => &mut self.e,
            H => &mut self.h,
            L => &mut self.l,
        };
        let bot_bit = (*reg << 7) & 0x80;
        *reg = (*reg >> 1) | bot_bit;
        self.f_z = if *reg == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = if bot_bit != 0 {true} else {false};
    }

    fn rr(&mut self, reg_op: Operand) {
        let reg = match reg_op {
            A => &mut self.a,
            B => &mut self.b,
            C => &mut self.c,
            D => &mut self.d,
            E => &mut self.e,
            H => &mut self.h,
            L => &mut self.l,
        };
        let carry_bit = if self.f_c {0x80} else {0};
        let bot_bit = (*reg << 7) & 0x80;
        *reg = (*reg >> 1) | carry_bit;
        self.f_z = if *reg == 0 {true} else {false};
        self.f_n = false;
        self.f_h = false;
        self.f_c = if bot_bit != 0 {true} else {false};
    }


}

