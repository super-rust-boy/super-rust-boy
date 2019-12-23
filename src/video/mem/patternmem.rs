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
    device::{
        Device,
        Queue
    },
    image::{
        Dimensions,
        immutable::ImmutableImage
    },
    format::{
        R8Uint
    },
    sync::{
        now, GpuFuture
    }
};

use std::sync::Arc;

const TEX_WIDTH: usize = 8;                     // Width of a tile in pixels.
const TEX_HEIGHT: usize = 8;                    // Height of a tile in pixels.
const TEX_AREA: usize = TEX_WIDTH * TEX_HEIGHT; // Area of a tile in total number of pixels.

pub type TileImage = Arc<ImmutableImage<R8Uint>>;
pub type TileFuture = Box<dyn GpuFuture>;

pub struct TileAtlas {
    atlas:      Vec<u8>,            // formatted atlas of tiles
    atlas_size: (usize, usize),     // width/height of texture in tiles
    
    image:      Option<TileImage>   // cached images
}

impl TileAtlas {
    pub fn new(atlas_size: (usize, usize)) -> Self {
        let atlas_area = (atlas_size.0 * atlas_size.1) * TEX_AREA;
        TileAtlas {
            atlas:      vec![0; atlas_area],
            atlas_size: atlas_size,

            image:      None,
        }
    }

    // Write a pixel to the atlas.
    // The least significant bit.
    #[inline]
    pub fn set_pixel_lower_row(&mut self, loc: usize, row: u8) {
        for i in 0..8 {
            let bit = (row >> (7 - i)) & 1;
            self.atlas[loc + i] = (self.atlas[loc + i] & 0b10) | bit;
        }

        self.image = None;
    }

    // The most significant bit.
    #[inline]
    pub fn set_pixel_upper_row(&mut self, loc: usize, row: u8) {
        for i in 0..8 {
            let bit = (row >> (7 - i)) & 1;
            self.atlas[loc + i] = (self.atlas[loc + i] & 0b01) | (bit << 1);
        }

        self.image = None;
    }

    // Read a pixel row from the atlas.
    pub fn get_pixel_lower_row(&self, loc: usize) -> u8 {
        (0..8).fold(0, |acc, i| {
            let bit = self.atlas[loc + i] & 0b01;
            let shift = 7 - i;
            acc | (bit << shift)
        })
    }

    pub fn get_pixel_upper_row(&self, loc: usize) -> u8 {
        (0..8).fold(0, |acc, i| {
            let bit = (self.atlas[loc + i] & 0b10) >> 1;
            let shift = 7 - i;
            acc | (bit << shift)
        })
    }

    // Make an image from the atlas.
    pub fn get_image(&mut self, device: &Arc<Device>, queue: &Arc<Queue>) -> (TileImage, TileFuture) {
        if let Some(image) = &self.image {
            (image.clone(), Box::new(now(device.clone())))
        } else {
            let width = (self.atlas_size.0 * TEX_WIDTH) as u32;
            let height = (self.atlas_size.1 * TEX_HEIGHT) as u32;

            let (image, future) = ImmutableImage::from_iter(
                self.atlas.clone().into_iter(),
                Dimensions::Dim2d { width: width, height: height },
                R8Uint,
                queue.clone()
            ).expect("Couldn't create image.");

            self.image = Some(image.clone());

            (image, Box::new(future))
        }
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