// This module deals with raw tile memory, and its mapping to Vulkan.
// Tiles are stored from 0x8000 - 0x97FF. (3 2kB blocks)
    // Each tile is 8x8 pixels (2 bits per pixel). Each pixel row takes 2 bytes.
    // Bit 7 is the leftmost pixel, Bit 0 is the rightmost.
    // Byte 0 is the LSB of the pixel, Byte 1 is the MSB of the pixel.
// These are mapped onto a 2D texture for rendering purposes.
    // The texture is 16 tiles wide, 24 high. 8 tile rows per block.
    // The tile at (0,0) is the tile from 0x8000 - 0x800F. (1,0) corresponds to 0x8010 - 0x801F.
    // The tile at (0,1) corresponds to 0x8100 - 0x810F.
// The texture uses one byte per pixel.
    // Each pixel is in reality, a 2-bit value mapped appropriately.
    // The fragment shader assigns the colour based on the value for the pixel.


use vulkano::{
    command_buffer::{
        AutoCommandBuffer,
        CommandBufferExecFuture,
    },
    device::Queue,
    image::{
        Dimensions,
        immutable::ImmutableImage
    },
    format::{
        R8Uint
    },
    sync::NowFuture
};

use std::sync::Arc;

pub type TileImage = Arc<ImmutableImage<R8Uint>>;
pub type TileFuture = CommandBufferExecFuture<NowFuture, AutoCommandBuffer>;

pub struct TileAtlas {
    pub atlas: Vec<u8>,         // formatted atlas of tiles
    atlas_size: (usize, usize), // width/height of texture in tiles
    tex_size: usize             // width/height of tile in texels
}

impl TileAtlas {
    pub fn new(atlas_size: (usize, usize), tex_size: usize) -> Self {
        let tex_area = tex_size * tex_size;
        let atlas_area = (atlas_size.0 * atlas_size.1) * tex_area;
        TileAtlas {
            atlas:      vec![0; atlas_area],
            atlas_size: atlas_size,
            tex_size:   tex_size
        }
    }

    // Make an image from the atlas.
    pub fn make_image(&self, queue: Arc<Queue>) -> (TileImage, TileFuture) {
        let width = (self.atlas_size.0 * self.tex_size) as u32;
        let height = (self.atlas_size.1 * self.tex_size) as u32;
        ImmutableImage::from_iter(
            self.atlas.clone().into_iter(),
            Dimensions::Dim2d { width: width, height: height },
            R8Uint,
            queue
        ).expect("Couldn't create image.")
    }

    // Get the size of a tile in the atlas.
    pub fn get_tile_size(&self) -> [f32; 2] {
        [1.0 / self.atlas_size.0 as f32, 1.0 / self.atlas_size.1 as f32]
    }

    // Get the size of the atlas (in tiles).
    pub fn get_atlas_size(&self) -> [f32; 2] {
        [self.atlas_size.0 as f32, self.atlas_size.1 as f32]
    }
}