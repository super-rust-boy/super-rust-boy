// Palettes used by Super Gameboy for GB games.
macro_rules! make_colour {
    ($r: expr, $g: expr, $b: expr) => {
        {
            Vector4::new(($r as f32) / 255.0, ($g as f32) / 255.0, ($b as f32) / 255.0, 1.0)
        }
    };
}

use cgmath::{
    Matrix4,
    Vector4
};

use super::PaletteColours;

// Which palette the user specified.
pub enum UserPalette {
    Default,
    Greyscale,
    Classic
}

// Palette for use with super game boy.
#[derive(Clone, Copy)]
pub struct SGBPalette {
    pub bg: PaletteColours,
    pub obj0: PaletteColours,
    pub obj1: PaletteColours
}

impl SGBPalette {
    #[inline]
    pub fn get_colour_0(&self) -> Vector4<f32> {
        self.bg[0]
    }
}

// GB greyscale palette
const BW_COLOURS: PaletteColours = Matrix4::from_cols(
    Vector4::new(1.0, 1.0, 1.0, 1.0),
    Vector4::new(0.65, 0.65, 0.65, 1.0),
    Vector4::new(0.33, 0.33, 0.33, 1.0),
    Vector4::new(0.0, 0.0, 0.0, 1.0)
);

pub const BW_PALETTE: SGBPalette = SGBPalette {
    bg: BW_COLOURS,
    obj0: BW_COLOURS,
    obj1: BW_COLOURS
};

// GB classic (green) palette
const CLASSIC_COLOURS: PaletteColours = Matrix4::from_cols(
    Vector4::new(0.647, 0.765, 0.086, 1.0),
    Vector4::new(0.596, 0.702, 0.165, 1.0),
    Vector4::new(0.184, 0.388, 0.145, 1.0),
    Vector4::new(0.055, 0.208, 0.059, 1.0)
);

pub const CLASSIC_PALETTE: SGBPalette = SGBPalette {
    bg: CLASSIC_COLOURS,
    obj0: CLASSIC_COLOURS,
    obj1: CLASSIC_COLOURS
};

// CGB/SGB palette lookup table.
pub fn lookup_sgb_palette(hash_in: u8, char_4_in: u8) -> SGBPalette {
    for (hash, char_4, palette) in SGB_PALETTES {
        if (*hash == hash_in) && ((*char_4 == 0) || (*char_4 == char_4_in)) {
            return *palette;
        }
    }

    BW_PALETTE
}

const SGB_PALETTES: &[(u8, u8, SGBPalette)] = &[
    (0x46, 0x52, SGBPalette {
            bg: Matrix4::from_cols(
                make_colour!(0xFF, 0xFF, 0xFF),
                make_colour!(0x63, 0xA5, 0xFF),
                make_colour!(0x00, 0x00, 0xFF),
                make_colour!(0x00, 0x00, 0x00)
            ),
            obj0: Matrix4::from_cols(
                make_colour!(0xFF, 0xFF, 0x00),
                make_colour!(0xFF, 0x00, 0x00),
                make_colour!(0x63, 0x00, 0x00),
                make_colour!(0x00, 0x00, 0x00)
            ),
            obj1: Matrix4::from_cols(
                make_colour!(0xFF, 0xFF, 0xFF),
                make_colour!(0x7B, 0xFF, 0x31),
                make_colour!(0x00, 0x84, 0x00),
                make_colour!(0x00, 0x00, 0x00)
            )
        }
    )
];