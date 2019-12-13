// Interrupt flags and constants

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct InterruptFlags: u8 {
        const V_BLANK  = bit!(0);
        const LCD_STAT = bit!(1);
        const TIMER    = bit!(2);
        const SERIAL   = bit!(3);
        const JOYPAD   = bit!(4);
    }
}

// Interrupt vector locations
pub mod vector {
    pub const V_BLANK: u16  = 0x0040;
    pub const LCD_STAT: u16 = 0x0048;
    pub const TIMER: u16    = 0x0050;
    pub const SERIAL: u16   = 0x0058;
    pub const JOYPAD: u16   = 0x0060;
}