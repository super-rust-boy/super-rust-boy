mod drawing;
mod consts;
mod patternmem;
mod sprite;
mod palette;
mod mapcache;

use super::types::{
    Colour
};
use consts::*;
use patternmem::*;
use palette::{
    r#static::StaticPaletteMem,
    dynamic::DynamicPaletteMem
};
use sprite::{
    ObjectMem,
    Sprite
};
use mapcache::*;
use super::sgbpalettes::SGBPalette;

use super::regs::VideoRegs;

// VRAM is shared between threads and contains some cached data
pub struct VRAM {
    // Raw tile mem and tile maps
    pub tile_mem:           TileMem,
    pub tile_map_0:         Vec<u8>,
    pub tile_map_1:         Vec<u8>,
    pub tile_attrs_0:       Vec<u8>,
    pub tile_attrs_1:       Vec<u8>,
    pub object_mem:         ObjectMem,

    // Tile map caches // TODO move these
    map_cache_0:            MapCache,
    map_cache_1:            MapCache,

    // Palettes
    pub palettes:           StaticPaletteMem,
    pub colour_palettes:    DynamicPaletteMem,
}

impl VRAM {
    pub fn new(palette: SGBPalette, cgb_mode: bool) -> Self {
        VRAM {
            tile_mem:           TileMem::new(if cgb_mode {TILE_DATA_HEIGHT_CGB} else {TILE_DATA_HEIGHT_GB} * TILE_DATA_WIDTH),
            tile_map_0:         vec![0; 32 * 32],
            tile_map_1:         vec![0; 32 * 32],
            tile_attrs_0:       if cgb_mode {vec![0; 32 * 32]} else {Vec::new()},
            tile_attrs_1:       if cgb_mode {vec![0; 32 * 32]} else {Vec::new()},
            object_mem:         ObjectMem::new(),

            map_cache_0:        MapCache::new(cgb_mode),
            map_cache_1:        MapCache::new(cgb_mode),

            palettes:           StaticPaletteMem::new(palette),
            colour_palettes:    DynamicPaletteMem::new(),
        }
    }
}

impl VRAM {

    pub fn ref_tile<'a>(&'a self, tile_num: usize) -> &'a Tile {
        self.tile_mem.ref_tile(tile_num)
    }

    // Get background tilemap data.
    pub fn ref_background<'a>(&'a self, regs: &VideoRegs) -> &'a Vec<Vec<u8>> {
        if !regs.bg_tile_map_select() {
            self.map_cache_0.ref_texels()
        } else {
            self.map_cache_1.ref_texels()
        }
    }

    // Get window tilemap data.
    pub fn ref_window<'a>(&'a self, regs: &VideoRegs) -> &'a Vec<Vec<u8>> {
        if !regs.window_tile_map_select() {
            self.map_cache_0.ref_texels()
        } else {
            self.map_cache_1.ref_texels()
        }
    }

    // Get background tilemap attribute for pixel pos.
    pub fn get_background_attr<'a>(&'a self, regs: &VideoRegs) -> &'a Vec<Vec<TileAttributes>> {
        if !regs.bg_tile_map_select() {
            self.map_cache_0.ref_attrs()
        } else {
            self.map_cache_1.ref_attrs()
        }
    }

    // Get window tilemap attributes.
    pub fn ref_window_attrs<'a>(&'a self, regs: &VideoRegs) -> &'a Vec<Vec<TileAttributes>> {
        if !regs.window_tile_map_select() {
            self.map_cache_0.ref_attrs()
        } else {
            self.map_cache_1.ref_attrs()
        }
    }

    pub fn get_objects_for_line(&self, y: u8, regs: &VideoRegs) -> Vec<Sprite> {
        if regs.display_sprites() {
            let large_sprites = regs.is_large_sprites();
            self.object_mem.get_objects_for_line(y, large_sprites)
        } else {
            Vec::new()
        }
    }

    #[inline]
    pub fn get_bg_colour(&self, texel: u8) -> Colour {
        self.palettes.get_colour(0, texel)
    }

    #[inline]
    pub fn get_obj_0_colour(&self, texel: u8) -> Colour {
        self.palettes.get_colour(1, texel)
    }

    #[inline]
    pub fn get_obj_1_colour(&self, texel: u8) -> Colour {
        self.palettes.get_colour(2, texel)
    }

    #[inline]
    pub fn get_gbc_bg_colour(&self, which: u8, texel: u8) -> Colour {
        self.colour_palettes.get_bg_colour(which as usize, texel)
    }

    #[inline]
    pub fn get_gbc_obj_colour(&self, which: u8, texel: u8) -> Colour {
        self.colour_palettes.get_obj_colour(which as usize, texel)
    }

    pub fn set_cache_0_dirty(&mut self) {
        self.map_cache_0.set_dirty();
    }

    pub fn set_cache_1_dirty(&mut self) {
        self.map_cache_1.set_dirty();
    }
}