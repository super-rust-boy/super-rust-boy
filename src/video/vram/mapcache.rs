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
                let base_y = (i / 32) << 3;
                let base_x = (i % 32) << 3;
                for (y, texel_row) in self.texels.iter_mut().skip(base_y).take(8).enumerate() {
                    for (x, texel) in texel_row.iter_mut().skip(base_x).take(8).enumerate() {
                        let tile_index = if regs.lo_tile_data_select() {
                            *tile_num as usize
                        } else {
                            let signed = *tile_num as i8;
                            (256 + (signed as isize)) as usize
                        };
                        let tex = tile_mem.ref_tile(tile_index).get_texel(x, y);
                        *texel = tex;
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
                let base_y = (i / 32) << 3;
                let base_x = (i % 32) << 3;
                for (y, (texel_row, attrs_row)) in self.texels.iter_mut().zip(self.attrs.iter_mut()).skip(base_y).take(8).enumerate() {
                    for (x, (texel, attrs)) in texel_row.iter_mut().zip(attrs_row.iter_mut()).skip(base_x).take(8).enumerate() {
                        let bank_offset = if attr_flags.contains(TileAttributes::VRAM_BANK) {384} else {0};
                        let tile_index = if regs.lo_tile_data_select() {
                            *tile_num as usize
                        } else {
                            let signed = *tile_num as i8;
                            (256 + (signed as isize)) as usize
                        } + bank_offset;
                        
                        let tex_x = if attr_flags.contains(TileAttributes::X_FLIP) {7-x} else {x};
                        let tex_y = if attr_flags.contains(TileAttributes::Y_FLIP) {7-y} else {y};
                        
                        *texel = tile_mem.ref_tile(tile_index).get_texel(tex_x, tex_y);
                        *attrs = attr_flags;
                    }
                }
            }
    
            self.dirty = false;
        }
    }
}