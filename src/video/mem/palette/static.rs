// Game Boy and Super Game Boy 2-bit palettes.
use cgmath::{
    Matrix4,
    Vector4
};

use crate::video::{
    PaletteColours,
    sgbpalettes::SGBPalette
};

// A palette with hard-coded colours.
struct StaticPalette {
    colours: PaletteColours,
    raw: u8
}

impl StaticPalette {
    pub fn new(colours: PaletteColours) -> Self {
        StaticPalette {
            colours: colours,
            raw: 0
        }
    }

    pub fn get_palette(&self, transparent: bool) -> PaletteColours {
        let colour_1 = (self.raw & 0b00001100) >> 2;
        let colour_2 = (self.raw & 0b00110000) >> 4;
        let colour_3 = (self.raw & 0b11000000) >> 6;

        let col_0 = if transparent {
            Vector4::new(0.0, 0.0, 0.0, 0.0)
        } else {
            let colour_0 = self.raw & 0b00000011;
            self.colours[colour_0 as usize]
        };

        Matrix4::from_cols(
            col_0,
            self.colours[colour_1 as usize],
            self.colours[colour_2 as usize],
            self.colours[colour_3 as usize]
        )
    }

    pub fn get_colour_0(&self) -> Vector4<f32> {
        let colour_0 = (self.raw & 0b00000011) as usize;
        self.colours[colour_0]
    }

    pub fn read(&self) -> u8 {
        self.raw
    }

    pub fn write(&mut self, val: u8) {
        self.raw = val;
    }
}

// A group of palettes
pub struct StaticPaletteMem {
    palettes:   Vec<StaticPalette>,

    dirty:      bool
}

impl StaticPaletteMem {
    pub fn new(colours: SGBPalette) -> Self {
        StaticPaletteMem {
            palettes: vec![
                StaticPalette::new(colours.bg),
                StaticPalette::new(colours.obj0),
                StaticPalette::new(colours.obj1)
            ],

            dirty:  true
        }
    }

    pub fn make_data(&mut self) -> Vec<PaletteColours> {
        self.dirty = false;
        vec![
            self.palettes[0].get_palette(true),       // BG
            self.palettes[0].get_palette(false),    // Window
            self.palettes[1].get_palette(true),     // Sprite 0
            self.palettes[2].get_palette(true)      // Sprite 1
        ]
    }

    pub fn get_colour_0(&self) -> Vector4<f32> {
        self.palettes[0].get_colour_0()
    }

    pub fn read(&self, which: usize) -> u8 {
        self.palettes[which].read()
    }

    pub fn write(&mut self, which: usize, val: u8) {
        self.palettes[which].write(val);

        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
}