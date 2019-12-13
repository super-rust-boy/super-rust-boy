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
                znhc: {:08b}\n\
                pc: {:04X} sp: {:04X}",
                self.a, self.b, self.c, self.d, self.e, self.h, self.l,
                self.flags,
                self.pc, self.sp)
    }
}
