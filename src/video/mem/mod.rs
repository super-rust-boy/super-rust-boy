mod regs;

use crate::mem::MemDevice;

use super::vram::VRAM;

use std::sync::{
    Arc, Mutex
};

pub use regs::VideoRegs;

// Video memory layer
pub struct VideoMem {
    vram:               Arc<Mutex<VRAM>>,
    regs:               VideoRegs,

    // CGB things
    cgb_mode:           bool,
    vram_bank:          u8,

    // Misc
    cycle_count:        u32,
}

impl VideoMem {
    pub fn new(vram: Arc<Mutex<VRAM>>, cgb_mode: bool) -> Self {
        VideoMem {
            vram:               vram,
            regs:               VideoRegs::new(),
            
            cgb_mode:           cgb_mode,
            //colour_palettes:    DynamicPaletteMem::new(),
            vram_bank:          0,

            //clear_colour:       palette.get_colour_0(),
            cycle_count:        0
        }
    }

}

/*impl VideoMem {
    pub fn read_flags(&self) -> LCDStatusFlags {
        self.regs.lcd_status.read_flags()
    }

    pub fn read_mode(&self) -> super::Mode {
        self.regs.lcd_status.read_mode()
    }

    pub fn write_mode(&mut self, mode: super::Mode) {
        self.regs.lcd_status.write_mode(mode);
    }
}*/

// Renderer access functions.
impl VideoMem {

}

// Accessed from Adapters
impl VideoMem {

}


#[inline]
fn get_base_pixel(base: usize) -> usize {
    const PIX_SHIFT: usize = 1;
    const X_SHIFT: usize = 4;
    const Y_SHIFT: usize = 8;

    const PIX_MASK: usize = 0x7;
    const X_MASK: usize = 0xF;

    let pixel_row_num = (base >> PIX_SHIFT) & PIX_MASK;
    let tile_x = (base >> X_SHIFT) & X_MASK;
    let tile_y = base >> Y_SHIFT;

    // tile_x * 8 pixels across per tile
    // pixel_row_num * 8 pixels across per tile * 16 tiles per row
    // tile_y * 8x8 pixels per tile * 16 tiles per row
    let base_pixel = tile_x + (pixel_row_num * 16) + (tile_y * 8 * 16);

    base_pixel * 8
}

// Writing raw tile data explained:
// We have to convert a number in the range (0x8000, 0x9800) to 8 adjacent pixels.
// We then convert that 1D array of pixels to a 2D image (the texture atlas).
// The image is 16x24 tiles, and each tile is 8x8 pixels. So the total size is 128x192 pixels.
// As explained in patternmem.rs (in hex):
    // the tile x coord is nibble xxXx
    // the tile y coord is nibble xXxx
    // the row is 2 bytes of nibble xxxX
// The first 128 bytes (pixels) of the atlas is the first row of the first 16 tiles.
// So to get the exact pixel:
    // Subtract 0x8000 (ignore the top nibble)
    // Get the tile x coord, multiply by 8 to get the x offset
    // Get the row of the tile, multiply by (8x16) to get the inner tile y offset
    // Get the tile y coord, multiply by (8x16x8) to get the tile y offset
    // And then just add these offsets together as we are working with a 1D array.