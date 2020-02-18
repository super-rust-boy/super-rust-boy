// Game Boy Color 15-bit palettes.
use cgmath::{
    Vector4,
    Matrix4
};

use bitflags::bitflags;

use crate::{
    mem::MemDevice,
    video::{
        PaletteColours,
        Colour
    }
};

const MAX_COLOUR: f32 = 0x1F as f32;

bitflags! {
    #[derive(Default)]
    struct PaletteIndex: u8 {
        const AUTO_INCREMENT = bit!(7);
    }
}

/*impl Colour15 {
    fn new() -> Self {
        Colour {
            r: 0x1F,
            g: 0x1F,
            b: 0x1F
        }
    }

    fn read(&self, low_byte: bool) -> u8 {
        if low_byte {
            self.r | ((self.g & 0x7) << 5)
        } else {
            ((self.g >> 3) & 0x3) | (self.b << 2)
        }
    }

    fn write(&mut self, val: u8, low_byte: bool) {
        if low_byte {
            self.r = val & 0x1F;
            self.g &= 0x18;
            self.g |= (val >> 5) & 0x7;
        } else {
            self.g &= 0x7;
            self.g |= (val & 0x3) << 3;
            self.b = (val >> 2) & 0x1F;
        }
    }

    fn get_vector(&self) -> Vector4<u8> {
        Vector4::new(
            self.r,
            self.g,
            self.b,
            1.0
        )
    }
}*/

#[derive(Clone)]
struct DynamicPalette {
    colours: PaletteColours
}

impl DynamicPalette {
    fn new() -> Self {
        DynamicPalette {
            colours: [Colour::new(); 3]
        }
    }

    fn get_palette(&self, transparent: bool) -> PaletteColours {
        let col_0 = if transparent {
            Vector4::new(0.0, 0.0, 0.0, 0.0)
        } else {
            self.colours[0].get_vector()
        };

        Matrix4::from_cols(
            col_0,
            self.colours[1].get_vector(),
            self.colours[2].get_vector(),
            self.colours[3].get_vector()
        )
    }
}

impl MemDevice for DynamicPalette {
    fn read(&self, loc: u16) -> u8 {
        let colour = (loc / 2) as usize;
        let low_byte = (loc % 2) == 0;
        self.colours[colour].read(low_byte)
    }

    fn write(&mut self, loc: u16, val: u8) {
        let colour = (loc / 2) as usize;
        let low_byte = (loc % 2) == 0;
        self.colours[colour].write(val, low_byte);
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

    pub fn make_data(&mut self) -> Vec<PaletteColours> {
        self.bg_palettes.iter()
            .map(|p| p.get_palette(true))
            .chain(self.bg_palettes.iter().map(|p| p.get_palette(false)))
            .chain(self.obj_palettes.iter().map(|p| p.get_palette(true)))
            .collect::<Vec<_>>()
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