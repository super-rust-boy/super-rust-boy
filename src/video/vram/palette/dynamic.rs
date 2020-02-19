// Game Boy Color 15-bit palettes.
use bitflags::bitflags;

use crate::{
    mem::MemDevice,
    video::{
        PaletteColours,
        Colour
    }
};

const MAX_COLOUR: u16 = 0x1F;
macro_rules! col15_to_col888 {
    ($rgb:expr) => {
        {
            let r = ($rgb & MAX_COLOUR) << 3;
            let g = (($rgb >> 5) & MAX_COLOUR) << 3;
            let b = (($rgb >> 10) & MAX_COLOUR) << 3;
            Colour::new(r as u8, g as u8, b as u8)
        }
    };
}

/*macro_rules! col888_to_col15 {
    ($colour:expr) => {
        {
            let r = colour.r >> 3;
            let g = colour.g >> 3;
            let b = colour.b >> 3;
            r | (g << 5) | (b << 5)
        }
    };
}*/

bitflags! {
    #[derive(Default)]
    struct PaletteIndex: u8 {
        const AUTO_INCREMENT = bit!(7);
    }
}

#[derive(Clone)]
struct DynamicPalette {
    colours:    PaletteColours,
    raw:        [u8; 8],
}

impl DynamicPalette {
    fn new() -> Self {
        DynamicPalette {
            colours:    [Colour::zero(); 4],
            raw:        [0; 8],
        }
    }
}

impl MemDevice for DynamicPalette {
    fn read(&self, loc: u16) -> u8 {
        self.raw[(loc % 8) as usize]
    }

    fn write(&mut self, loc: u16, val: u8) {
        let colour = (loc >> 1) as usize;
        self.raw[(loc % 8) as usize] = val;

        let raw_idx = colour << 1;
        self.colours[colour] = col15_to_col888!(make_16!(self.raw[raw_idx + 1], self.raw[raw_idx]));
    }
}

// A group of dynamic palettes
pub struct DynamicPaletteMem {
    bg_palettes:        Vec<DynamicPalette>,
    bg_palette_index:   usize,
    bg_auto_inc:        PaletteIndex,

    obj_palettes:       Vec<DynamicPalette>,
    obj_palette_index:  usize,
    obj_auto_inc:       PaletteIndex,
}

impl DynamicPaletteMem {
    pub fn new() -> Self {
        DynamicPaletteMem {
            bg_palettes:        vec![DynamicPalette::new(); 8],
            bg_palette_index:   0,
            bg_auto_inc:        PaletteIndex::default(),

            obj_palettes:       vec![DynamicPalette::new(); 8],
            obj_palette_index:  0,
            obj_auto_inc:       PaletteIndex::default(),
        }
    }

    pub fn get_bg_colour(&self, which: usize, texel: u8) -> Colour {
        self.bg_palettes[which].colours[texel as usize]
    }

    pub fn get_obj_colour(&self, which: usize, texel: u8) -> Colour {
        self.obj_palettes[which].colours[texel as usize]
    }

    pub fn read_bg_index(&self) -> u8 {
        (self.bg_palette_index as u8) | self.bg_auto_inc.bits()
    }

    pub fn write_bg_index(&mut self, val: u8) {
        self.bg_palette_index = (val & 0x3F) as usize;
        self.bg_auto_inc = PaletteIndex::from_bits_truncate(val);
    }

    pub fn read_obj_index(&self) -> u8 {
        (self.obj_palette_index as u8) | self.obj_auto_inc.bits()
    }

    pub fn write_obj_index(&mut self, val: u8) {
        self.obj_palette_index = (val & 0x3F) as usize;
        self.obj_auto_inc = PaletteIndex::from_bits_truncate(val);
    }

    pub fn read_bg(&self) -> u8 {
        let palette = self.bg_palette_index / 8;
        let colour = self.bg_palette_index % 8;
        self.bg_palettes[palette].read(colour as u16)
    }

    pub fn write_bg(&mut self, val: u8) {
        let palette = self.bg_palette_index / 8;
        let colour = self.bg_palette_index % 8;
        self.bg_palettes[palette].write(colour as u16, val);
        if self.bg_auto_inc.contains(PaletteIndex::AUTO_INCREMENT) {
            self.bg_palette_index = (self.bg_palette_index + 1) % 0x40;
        }
    }

    pub fn read_obj(&self) -> u8 {
        let palette = self.obj_palette_index / 8;
        let colour = self.obj_palette_index % 8;
        self.obj_palettes[palette].read(colour as u16)
    }

    pub fn write_obj(&mut self, val: u8) {
        let palette = self.obj_palette_index / 8;
        let colour = self.obj_palette_index % 8;
        self.obj_palettes[palette].write(colour as u16, val);
        if self.obj_auto_inc.contains(PaletteIndex::AUTO_INCREMENT) {
            self.obj_palette_index = (self.obj_palette_index + 1) % 0x40;
        }
    }
}