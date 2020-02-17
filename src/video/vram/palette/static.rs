// Game Boy and Super Game Boy 2-bit palettes.
use crate::video::{
    PaletteColours,
    Colour,
    sgbpalettes::SGBPalette
};

// A palette with hard-coded colours.
struct StaticPalette {
    colours: PaletteColours,
    palette: PaletteColours,
    raw: u8
}

impl StaticPalette {
    pub fn new(colours: PaletteColours) -> Self {
        StaticPalette {
            colours: colours,
            palette: colours,
            raw: 0
        }
    }

    pub fn read(&self) -> u8 {
        self.raw
    }

    pub fn write(&mut self, val: u8) {
        self.raw = val;

        let colour_0 = val & 0b00000011;
        let colour_1 = (val & 0b00001100) >> 2;
        let colour_2 = (val & 0b00110000) >> 4;
        let colour_3 = (val & 0b11000000) >> 6;

        self.palette[0] = self.colours[colour_0 as usize];
        self.palette[1] = self.colours[colour_1 as usize];
        self.palette[2] = self.colours[colour_2 as usize];
        self.palette[3] = self.colours[colour_3 as usize];
    }
}

// A group of palettes
pub struct StaticPaletteMem {
    palettes:   Vec<StaticPalette>
}

impl StaticPaletteMem {
    pub fn new(colours: SGBPalette) -> Self {
        StaticPaletteMem {
            palettes: vec![
                StaticPalette::new(colours.bg),
                StaticPalette::new(colours.obj0),
                StaticPalette::new(colours.obj1)
            ]
        }
    }

    pub fn read(&self, which: usize) -> u8 {
        self.palettes[which].read()
    }

    pub fn write(&mut self, which: usize, val: u8) {
        self.palettes[which].write(val);
    }

    pub fn get_colour(&self, which: usize, texel: u8) -> Colour {
        self.palettes[which].palette[texel as usize]
    }
}