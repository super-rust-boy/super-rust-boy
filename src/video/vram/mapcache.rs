// Cache for storing tile maps.
use bitflags::bitflags;

use super::patternmem::TileMem;
use super::super::regs::VideoRegs;

bitflags!{
    #[derive(Default)]
    pub struct TileAttributes: u8 {
        const PRIORITY  = bit!(7);
        const Y_FLIP    = bit!(6);
        const X_FLIP    = bit!(5);
        const VRAM_BANK = bit!(3);
        const CGB_PAL   = bit!(2) | bit!(1) | bit!(0);
    }
}

pub struct MapCache {
    texels: Vec<Vec<u8>>,
    attrs:  Vec<Vec<TileAttributes>>,   // TODO: is this the best way of doing this?

    dirty:  bool,
}

impl MapCache {
    pub fn new(cgb_mode: bool) -> Self {
        MapCache {
            texels: vec![vec![0; 256]; 256],
            attrs:  if cgb_mode {vec![vec![TileAttributes::default(); 256]; 256]} else {Vec::new()},

            dirty:  true,
        }
    }

    #[inline]
    pub fn get_texel(&self, x: usize, y: usize) -> u8 {
        self.texels[y][x]
    }

    #[inline]
    pub fn get_attrs(&self, x: usize, y: usize) -> TileAttributes {
        self.attrs[y][x]
    }

    pub fn set_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn construct_gb(&mut self, tile_map: &[u8], tile_mem: &TileMem, regs: &VideoRegs) {
        if self.dirty {
            for (i, tile_num) in tile_map.iter().enumerate() {
                // TODO: iterate over tile
                let base_y = (i / 32) << 3;
                let base_x = (i % 32) << 3;
                for y in 0..8 {
                    for x in 0..8 {
                        // TODO: attrs
                        let tile_index = if regs.lo_tile_data_select() {
                            *tile_num as usize
                        } else {
                            let signed = *tile_num as i8;
                            (256 + (signed as isize)) as usize
                        };
                        let tex = tile_mem.ref_tile(tile_index).get_texel(x, y);
                        self.texels[base_y + y][base_x + x] = tex;
                    }
                }
            }
    
            self.dirty = false;
        }
    }

    pub fn construct_cgb(&mut self, tile_map: &[u8], tile_attrs: &[u8], tile_mem: &TileMem, regs: &VideoRegs) {
        if self.dirty {
            for (i, (tile_num, attrs)) in tile_map.iter().zip(tile_attrs.iter()).enumerate() {
                let attr_flags = TileAttributes::from_bits_truncate(*attrs);
                // TODO: iterate over tile
                let base_y = (i / 32) << 3;
                let base_x = (i % 32) << 3;
                for y in 0..8 {
                    for x in 0..8 {
                        let bank_offset = if attr_flags.contains(TileAttributes::VRAM_BANK) {384} else {0};
                        let tile_index = if regs.lo_tile_data_select() {
                            *tile_num as usize
                        } else {
                            let signed = *tile_num as i8;
                            (256 + (signed as isize)) as usize
                        } + bank_offset;
                        
                        let tex_x = if attr_flags.contains(TileAttributes::X_FLIP) {7-x} else {x};
                        let tex_y = if attr_flags.contains(TileAttributes::Y_FLIP) {7-y} else {y};
                        
                        let tex = tile_mem.ref_tile(tile_index).get_texel(tex_x, tex_y);
                        self.texels[base_y + y][base_x + x] = tex;
                        
                        self.attrs[base_y + y][base_x + x] = attr_flags;
                    }
                }
            }
    
            self.dirty = false;
        }
    }
}