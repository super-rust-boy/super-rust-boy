pub const TILE_DATA_WIDTH: usize = 16;      // Width of the tile data in tiles.
pub const TILE_DATA_HEIGHT_GB: usize = 24;  // Height of the tile data in tiles for GB.
pub const TILE_DATA_HEIGHT_CGB: usize = 48; // Height of the tile data in tiles for GB Color.
pub const MAP_SIZE: usize = 32;             // Width / Height of bg/window tile maps.
pub const VIEW_WIDTH: usize = 20;           // Width of visible area.
pub const VIEW_HEIGHT: usize = 18;          // Height of visible area.

pub const OFFSET_FRAC_X: f32 = (MAP_SIZE as f32 / VIEW_WIDTH as f32) / 128.0;   // Mult with an offset to get the amount to offset by
pub const OFFSET_FRAC_Y: f32 = (MAP_SIZE as f32 / VIEW_HEIGHT as f32) / 128.0;  // Mult with an offset to get the amount to offset by

pub const TEX_WIDTH: usize = 8;                     // Width of a tile in pixels.
pub const TEX_HEIGHT: usize = 8;                    // Height of a tile in pixels.
pub const TEX_AREA: usize = TEX_WIDTH * TEX_HEIGHT; // Area of a tile in total number of pixels.
