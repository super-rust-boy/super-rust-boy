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
#[derive(PartialEq)]
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
    for (hash, char_4, palette) in SGB_PALETTE_TABLE {
        if (*hash == hash_in) && ((*char_4 == 0) || (*char_4 == char_4_in)) {
            return *palette;
        }
    }

    BW_PALETTE
}

const SGB_PALETTE_TABLE: &[(u8, u8, SGBPalette)] = &[
    (0x14, 0x00, PKMN_RED),
    (0x15, 0x00, PKMN_YELLOW),
    (0x18, 0x4B, DK_LAND_2),    // DK Land JP
    (0x46, 0x45, MARIO_LAND),
    (0x46, 0x52, METROID_2),
    (0x59, 0x00, WARIO_LAND),
    (0x61, 0x45, PKMN_BLUE),
    (0x6A, 0x4B, DK_LAND_2),    // DK Land 2
    (0x6B, 0x00, DK_LAND_2),    // DK Land 3
    (0x70, 0x00, ZELDA),        // Link's Awakening
    (0x86, 0x00, DK_LAND),      // DK Land US/EU
    (0xA8, 0x00, DK_LAND),      // Super Donkey Kong GB
    (0xC6, 0x41, WARIO_LAND),   // Game Boy Wars
    (0xDB, 0x00, PKMN_YELLOW),  // Tetris
];

// Game-specific palettes.
const PKMN_YELLOW: SGBPalette = SGBPalette {
    bg: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0xFF, 0xFF, 0x00),
        make_colour!(0xFF, 0x00, 0x00),
        make_colour!(0x00, 0x00, 0x00)
    ),
    obj0: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0xFF, 0xFF, 0x00),
        make_colour!(0xFF, 0x00, 0x00),
        make_colour!(0x00, 0x00, 0x00)
    ),
    obj1: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0xFF, 0xFF, 0x00),
        make_colour!(0xFF, 0x00, 0x00),
        make_colour!(0x00, 0x00, 0x00)
    )
};

const PKMN_BLUE: SGBPalette = SGBPalette {
    bg: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0x63, 0xA5, 0xFF),
        make_colour!(0x00, 0x00, 0xFF),
        make_colour!(0x00, 0x00, 0x00)
    ),
    obj0: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0xFF, 0x84, 0x84),
        make_colour!(0x94, 0x3A, 0x3A),
        make_colour!(0x00, 0x00, 0x00)
    ),
    obj1: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0x63, 0xA5, 0xFF),
        make_colour!(0x00, 0x00, 0xFF),
        make_colour!(0x00, 0x00, 0x00)
    )
};

const PKMN_RED: SGBPalette = SGBPalette {
    bg: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0xFF, 0x84, 0x84),
        make_colour!(0x94, 0x3A, 0x3A),
        make_colour!(0x00, 0x00, 0x00)
    ),
    obj0: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0x7B, 0xFF, 0x31),
        make_colour!(0x00, 0x84, 0x00),
        make_colour!(0x00, 0x00, 0x00)
    ),
    obj1: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0xFF, 0x84, 0x84),
        make_colour!(0x94, 0x3A, 0x3A),
        make_colour!(0x00, 0x00, 0x00)
    )
};

const METROID_2: SGBPalette = SGBPalette {
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
};

const ZELDA: SGBPalette = SGBPalette {
    bg: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0xFF, 0x84, 0x84),
        make_colour!(0x94, 0x3A, 0x3A),
        make_colour!(0x00, 0x00, 0x00)
    ),
    obj0: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0x00, 0xFF, 0x00),
        make_colour!(0x31, 0x84, 0x00),
        make_colour!(0x00, 0x4A, 0x00)
    ),
    obj1: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0x63, 0xA5, 0xFF),
        make_colour!(0x00, 0x00, 0xFF),
        make_colour!(0x00, 0x00, 0x00)
    )
};

const WARIO_LAND: SGBPalette = SGBPalette {
    bg: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0xAD, 0xAD, 0x84),
        make_colour!(0x42, 0x73, 0x7B),
        make_colour!(0x00, 0x00, 0x00)
    ),
    obj0: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0xFF, 0x73, 0x00),
        make_colour!(0x94, 0x42, 0x00),
        make_colour!(0x00, 0x4A, 0x00)
    ),
    obj1: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0x5A, 0xBD, 0xFF),
        make_colour!(0xFF, 0x00, 0x00),
        make_colour!(0x00, 0x00, 0xFF)
    )
};

const DK_LAND: SGBPalette = SGBPalette {
    bg: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0x9C),
        make_colour!(0x94, 0xB5, 0xFF),
        make_colour!(0x63, 0x94, 0x73),
        make_colour!(0x00, 0x3A, 0x3A)
    ),
    obj0: Matrix4::from_cols(
        make_colour!(0xFF, 0xC5, 0x42),
        make_colour!(0xFF, 0xD6, 0x00),
        make_colour!(0x94, 0x3A, 0x00),
        make_colour!(0x4A, 0x00, 0x00)
    ),
    obj1: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0xFF, 0x84, 0x84),
        make_colour!(0x94, 0x3A, 0x3A),
        make_colour!(0x00, 0x00, 0x00)
    )
};

const DK_LAND_2: SGBPalette = SGBPalette {
    bg: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0x8C, 0x8C, 0xDE),
        make_colour!(0x52, 0x52, 0x8C),
        make_colour!(0x00, 0x00, 0x00)
    ),
    obj0: Matrix4::from_cols(
        make_colour!(0xFF, 0xC5, 0x42),
        make_colour!(0xFF, 0xD6, 0x00),
        make_colour!(0x94, 0x3A, 0x00),
        make_colour!(0x4A, 0x00, 0x00)
    ),
    obj1: Matrix4::from_cols(
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0x5A, 0xBD, 0xFF),
        make_colour!(0xFF, 0x00, 0x00),
        make_colour!(0x00, 0x00, 0xFF)
    )
};

const MARIO_LAND: SGBPalette = SGBPalette {
    bg: Matrix4::from_cols(
        make_colour!(0xB5, 0xB5, 0xFF),
        make_colour!(0xFF, 0xFF, 0x94),
        make_colour!(0xAD, 0x5A, 0x42),
        make_colour!(0x00, 0x00, 0x00)
    ),
    obj0: Matrix4::from_cols(
        make_colour!(0x00, 0x00, 0x00),
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0xFF, 0x84, 0x84),
        make_colour!(0x94, 0x3A, 0x3A)
    ),
    obj1: Matrix4::from_cols(
        make_colour!(0x00, 0x00, 0x00),
        make_colour!(0xFF, 0xFF, 0xFF),
        make_colour!(0xFF, 0x84, 0x84),
        make_colour!(0x94, 0x3A, 0x3A)
    )
};