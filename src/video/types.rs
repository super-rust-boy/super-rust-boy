#[derive(Clone, Copy, Debug)]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8
}

impl Colour {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Colour {
            r: r,
            g: g,
            b: b
        }
    }

    pub fn zero() -> Colour {
        Colour {
            r: 255,
            g: 255,
            b: 255
        }
    }
}

pub type PaletteColours = [Colour; 4];