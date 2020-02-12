mod patternmem;
//mod vertex;
mod palette;
mod drawing;
pub mod consts;
mod sprite;

use cgmath::Vector4;

use bitflags::bitflags;

use crate::mem::MemDevice;

use super::sgbpalettes::SGBPalette;
use super::types::{
    PaletteColours,
    Colour
};

use patternmem::*;
use palette::{
    r#static::StaticPaletteMem,
    //dynamic::DynamicPaletteMem
};
use sprite::{
    ObjectMem,
    Sprite
};

use consts::*;

bitflags! {
    #[derive(Default)]
    struct LCDControl: u8 {
        const ENABLE                    = bit!(7);
        const WINDOW_TILE_MAP_SELECT    = bit!(6);
        const WINDOW_DISPLAY_ENABLE     = bit!(5);
        const TILE_DATA_SELECT          = bit!(4);
        const BG_TILE_MAP_SELECT        = bit!(3);
        const OBJ_SIZE                  = bit!(2);
        const OBJ_DISPLAY_ENABLE        = bit!(1);
        const DISPLAY_PRIORITY          = bit!(0);
    }
}

bitflags! {
    #[derive(Default)]
    pub struct LCDStatusFlags: u8 {
        const COINCEDENCE_INT   = bit!(6);
        const OAM_INT           = bit!(5);
        const V_BLANK_INT       = bit!(4);
        const H_BLANK_INT       = bit!(3);
        const COINCEDENCE_FLAG  = bit!(2);
    }
}

pub struct LCDStatus {
    flags: LCDStatusFlags,
    video_mode: super::Mode,
}

impl LCDStatus {
    fn new() -> Self {
        LCDStatus {
            flags: LCDStatusFlags::default(),
            video_mode: super::Mode::_2
        }
    }

    fn read(&self) -> u8 {
        self.flags.bits() | self.video_mode as u8
    }

    fn write(&mut self, val: u8) {
        self.flags = LCDStatusFlags::from_bits_truncate(val);
        self.video_mode = super::Mode::from(val);
    }

    pub fn read_flags(&self) -> LCDStatusFlags {
        self.flags
    }

    pub fn read_mode(&self) -> super::Mode {
        self.video_mode
    }

    pub fn write_mode(&mut self, mode: super::Mode) {
        self.video_mode = mode;
    }
}

// Video memory layer
pub struct VideoMem {
    // Raw tile mem and tile maps
    tile_mem:           TileMem,
    tile_map_0:         Vec<u8>,
    tile_map_1:         Vec<u8>,
    tile_attrs_0:       Vec<u8>,
    tile_attrs_1:       Vec<u8>,
    object_mem:         ObjectMem,

    // Tile map caches
    map_cache_0:        Vec<Vec<u8>>,   // TODO: make this a type
    map_cache_0_dirty:  bool,
    map_cache_1:        Vec<Vec<u8>>,
    map_cache_1_dirty:  bool,

    // Flags / registers
    lcd_control:        LCDControl,
    pub lcd_status:     LCDStatus,
    scroll_y:           u8,
    scroll_x:           u8,
    lcdc_y:             u8,
    ly_compare:         u8,

    window_y:           u8,
    window_x:           u8,

    palettes:           StaticPaletteMem,

    // CGB things
    cgb_mode:           bool,
    //colour_palettes:    DynamicPaletteMem,
    vram_bank:          u8,

    // Misc
    //clear_colour:       Vector4<f32>,
    cycle_count:        u32,
}

impl VideoMem {
    pub fn new(palette: SGBPalette, cgb_mode: bool) -> Self {
        VideoMem {
            // TODO: move the raw mem elsewhere?
            tile_mem:           TileMem::new(if cgb_mode {TILE_DATA_HEIGHT_CGB} else {TILE_DATA_HEIGHT_GB} * TILE_DATA_WIDTH),
            tile_map_0:         vec![0; 32 * 32],
            tile_map_1:         vec![0; 32 * 32],
            tile_attrs_0:       if cgb_mode {vec![0; 32 * 32]} else {Vec::new()},
            tile_attrs_1:       if cgb_mode {vec![0; 32 * 32]} else {Vec::new()},
            object_mem:         ObjectMem::new(),

            map_cache_0:        vec![vec![0; 256]; 256],
            map_cache_0_dirty:  true,
            map_cache_1:        vec![vec![0; 256]; 256],
            map_cache_1_dirty:  true,

            lcd_control:        LCDControl::ENABLE,
            lcd_status:         LCDStatus::new(),
            scroll_y:           0,
            scroll_x:           0,
            lcdc_y:             0,
            ly_compare:         0,

            window_y:           0,
            window_x:           0,

            palettes:           StaticPaletteMem::new(palette),
            cgb_mode:           cgb_mode,
            //colour_palettes:    DynamicPaletteMem::new(),
            vram_bank:          0,

            //clear_colour:       palette.get_colour_0(),
            cycle_count:        0
        }
    }
    
    pub fn inc_lcdc_y(&mut self) {
        self.lcdc_y += 1;
        self.lcd_status.flags.set(LCDStatusFlags::COINCEDENCE_FLAG, self.lcdc_y == self.ly_compare);
    }

    pub fn set_lcdc_y(&mut self, val: u8) {
        self.lcdc_y = val;
        self.lcd_status.flags.set(LCDStatusFlags::COINCEDENCE_FLAG, self.lcdc_y == self.ly_compare);
    }

    pub fn compare_ly_equal(&self) -> bool {
        self.lcdc_y == self.ly_compare
    }

    pub fn inc_cycle_count(&mut self, cycles: u32) {
        self.cycle_count += cycles;
    }

    pub fn frame_cycle_reset(&mut self) {
        self.cycle_count -= 154 * 456;
    }

    pub fn get_cycle_count(&self) -> u32 {
        self.cycle_count
    }
}

// Renderer access functions.
impl VideoMem {
    // Check if display is enabled.
    pub fn display_enabled(&self) -> bool {
        self.lcd_control.contains(LCDControl::ENABLE)
    }

    // Get clear colour.
    /*pub fn get_clear_colour(&self) -> [f32; 4] {
        if self.display_enabled() && !self.cgb_mode {
            self.palettes.get_colour_0()
        } else {
            self.clear_colour
        }.into()
    }*/

    // Get push constants
    pub fn get_bg_scroll(&self) -> [f32; 2] {
        [self.scroll_x as f32 * -OFFSET_FRAC_X, self.scroll_y as f32 * -OFFSET_FRAC_Y]
    }

    pub fn get_window_position(&self) -> [f32; 2] {
        [(self.window_x as f32 - 7.0) * OFFSET_FRAC_X, self.window_y as f32 * OFFSET_FRAC_Y]
    }

    pub fn get_tile_data_offset(&self) -> u32 {
        if self.lcd_control.contains(LCDControl::TILE_DATA_SELECT) {
            0
        } else {
            256
        }
    }

    // Get the size of a single tile in the atlas.
    /*pub fn get_tile_size(&self) -> [f32; 2] {
        self.tile_mem.get_tile_size()
    }

    // Get the size of the atlas (in tiles).
    pub fn get_atlas_size(&self) -> [f32; 2] {
        self.tile_mem.get_atlas_size()
    }*/

    // Y lines
    pub fn get_lcd_y(&self) -> u8 {
        self.lcdc_y
    }

    pub fn get_scroll_y(&self) -> u8 {
        self.scroll_y
    }

    pub fn get_window_y(&self) -> u8 {
        self.window_y
    }
}

// Accessed from Adapters
impl VideoMem {
    // For rendering background.
    pub fn get_background_priority(&self) -> bool {
        self.lcd_control.contains(LCDControl::DISPLAY_PRIORITY)
    }

    // For rendering window.
    pub fn get_window_enable(&self) -> bool {
        self.lcd_control.contains(LCDControl::DISPLAY_PRIORITY | LCDControl::WINDOW_DISPLAY_ENABLE)
    }

    // For sprites.
    pub fn is_large_sprites(&self) -> bool {
        self.lcd_control.contains(LCDControl::OBJ_SIZE)
    }

    pub fn is_cgb_mode(&self) -> bool {
        self.cgb_mode
    }

    // Returns the raw tile atlas data (pattern memory). If None is returned, the data has not changed since last time.
    /*pub fn ref_tile_atlas<'a>(&'a mut self) -> Option<&'a [u8]> {
        if self.tile_mem.is_dirty() {
            Some(self.tile_mem.ref_data())
        } else {
            None
        }
    }*/

    pub fn ref_tile<'a>(&'a self, tile_num: usize) -> &'a Tile {
        self.tile_mem.ref_tile(tile_num)
    }

    // Get background tilemap data.
    pub fn ref_background<'a>(&'a self) -> &'a Vec<Vec<u8>> {
        if !self.lcd_control.contains(LCDControl::BG_TILE_MAP_SELECT) {
            &self.map_cache_0
        } else {
            &self.map_cache_1
        }
    }

    // Get window tilemap data.
    pub fn ref_window<'a>(&'a self) -> &'a Vec<Vec<u8>> {
        if !self.lcd_control.contains(LCDControl::WINDOW_TILE_MAP_SELECT) {
            &self.map_cache_0
        } else {
            &self.map_cache_1
        }
    }

    // Get background tilemap attributes.
    pub fn ref_background_attrs<'a>(&'a self) -> &'a [u8] {
        if !self.lcd_control.contains(LCDControl::BG_TILE_MAP_SELECT) {
            &self.tile_attrs_0
        } else {
            &self.tile_attrs_1
        }
    }

    // Get window tilemap attributes.
    pub fn ref_window_attrs<'a>(&'a self) -> &'a [u8] {
        if !self.lcd_control.contains(LCDControl::WINDOW_TILE_MAP_SELECT) {
            &self.tile_attrs_0
        } else {
            &self.tile_attrs_1
        }
    }

    // Get low-priority sprites (below the background) for given y line.
    /*pub fn ref_sprites_lo<'a>(&'a mut self, y: u8) -> Option<&'a mut Vec<Vertex>> {
        if self.lcd_control.contains(LCDControl::OBJ_DISPLAY_ENABLE) {
            let large_sprites = self.lcd_control.contains(LCDControl::OBJ_SIZE);
            let vertices = self.object_mem.ref_lo_vertices(y, large_sprites, self.cgb_mode);
            if !vertices.is_empty() {
                Some(vertices)
            } else {
                None
            }
        } else {
            None
        }
    }

    // Get high-priority sprites (above the background) for given y line.
    pub fn ref_sprites_hi<'a>(&'a mut self, y: u8) -> Option<&'a mut Vec<Vertex>> {
        if self.lcd_control.contains(LCDControl::OBJ_DISPLAY_ENABLE) {
            let large_sprites = self.lcd_control.contains(LCDControl::OBJ_SIZE);
            let vertices = self.object_mem.ref_hi_vertices(y, large_sprites, self.cgb_mode);
            if !vertices.is_empty() {
                Some(vertices)
            } else {
                None
            }
        } else {
            None
        }
    }*/
    pub fn ref_objects_for_line<'a>(&'a self, y: u8) -> Option<Vec<&'a Sprite>> {
        if self.lcd_control.contains(LCDControl::OBJ_DISPLAY_ENABLE) {
            let large_sprites = self.is_large_sprites();
            let vertices = self.object_mem.ref_objects_for_line(y, large_sprites);
            if !vertices.is_empty() {
                Some(vertices)
            } else {
                None
            }
        } else {
            None
        }
    }

    // Get palettes
    /*pub fn make_palettes(&mut self) -> Option<Vec<PaletteColours>> {
        if self.cgb_mode {
            if self.colour_palettes.is_dirty() {
                Some(self.colour_palettes.make_data())
            } else {
                None
            }
        } else {
            if self.palettes.is_dirty() {
                Some(self.palettes.make_data())
            } else {
                None
            }
        }
    }*/

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
}

// Internal methods.
impl VideoMem {
    #[inline]
    fn can_access_vram(&self) -> bool {
        self.lcd_status.read_mode() != super::Mode::_3
    }

    #[inline]
    fn can_access_oam(&self) -> bool {
        !self.lcd_control.contains(LCDControl::OBJ_DISPLAY_ENABLE) ||
        (self.lcd_status.read_mode() == super::Mode::_0) ||
        (self.lcd_status.read_mode() == super::Mode::_1)
    }

    fn set_lcd_control(&mut self, val: u8) {
        let was_display_enabled = self.display_enabled();
        self.lcd_control = LCDControl::from_bits_truncate(val);
        let is_display_enabled = self.display_enabled();

        // Has display been toggled on/off?
        if is_display_enabled != was_display_enabled {
            if is_display_enabled { // ON
                self.lcd_status.write_mode(super::Mode::_2);
                self.cycle_count = 0;
            } else {                // OFF
                self.lcd_status.write_mode(super::Mode::_0);
                self.lcdc_y = 0;
            }
        }
    }
}

impl MemDevice for VideoMem {
    fn read(&self, loc: u16) -> u8 {
        match loc {
            // Raw tile data
            0x8000..=0x97FF if self.can_access_vram() => {
                let base = (loc - 0x8000) as usize + (self.vram_bank as usize * 0x1800);

                if base % 2 == 0 {  // Lower bit
                    self.tile_mem.get_pixel_lower_row(base)
                } else {            // Upper bit
                    self.tile_mem.get_pixel_upper_row(base)
                }
            },
            // Background Map A
            0x9800..=0x9BFF if self.can_access_vram() => {
                /*let base = (loc - 0x9800) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                if self.vram_bank == 0 {
                    self.tile_map_0.get_tile_texture(x, y)
                } else {
                    self.tile_map_0.get_tile_attribute(x, y)
                }*/
                let index = (loc - 0x9800) as usize;
                if self.vram_bank == 0 {
                    self.tile_map_0[index]
                } else {
                    self.tile_attrs_0[index]
                }
            },
            // Background Map B
            0x9C00..=0x9FFF if self.can_access_vram() => {
                /*let base = (loc - 0x9C00) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                if self.vram_bank == 0 {
                    self.tile_map_1.get_tile_texture(x, y)
                } else {
                    self.tile_map_1.get_tile_attribute(x, y)
                }*/
                let index = (loc - 0x9C00) as usize;
                if self.vram_bank == 0 {
                    self.tile_map_1[index]
                } else {
                    self.tile_attrs_1[index]
                }
            },
            // Sprite data
            0xFE00..=0xFE9F if self.can_access_oam() => self.object_mem.read(loc - 0xFE00),
            // Registers
            0xFF40 => self.lcd_control.bits(),
            0xFF41 => self.lcd_status.read(),
            0xFF42 => self.scroll_y,
            0xFF43 => self.scroll_x,
            0xFF44 => self.lcdc_y,
            0xFF45 => self.ly_compare,
            0xFF47 => self.palettes.read(0),
            0xFF48 => self.palettes.read(1),
            0xFF49 => self.palettes.read(2),
            0xFF4A => self.window_y,
            0xFF4B => self.window_x,
            0xFF4F => self.vram_bank | 0xFE,
            // Colour palettes
            //0xFF68 => self.colour_palettes.read_bg_index(),
            //0xFF69 => self.colour_palettes.read_bg(),
            //0xFF6A => self.colour_palettes.read_obj_index(),
            //0xFF6B => self.colour_palettes.read_obj(),
            _ => 0xFF
        }
    }

    fn write(&mut self, loc: u16, val: u8) {
        match loc {
            // Raw tile data
            0x8000..=0x97FF if self.can_access_vram() => {
                let base = (loc - 0x8000) as usize + (self.vram_bank as usize * 0x1800);

                if base % 2 == 0 {  // Lower bit
                    self.tile_mem.set_pixel_lower_row(base, val);
                } else {            // Upper bit
                    self.tile_mem.set_pixel_upper_row(base, val);
                }

                self.map_cache_0_dirty = true;
                self.map_cache_1_dirty = true;
            },
            // Background Map A
            0x9800..=0x9BFF if self.can_access_vram() => {
                /*let base = (loc - 0x9800) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                if self.vram_bank == 0 {
                    self.tile_map_0.set_tile_texture(x, y, val);
                } else {
                    self.tile_map_0.set_tile_attribute(x, y, val);
                }*/
                let index = (loc - 0x9800) as usize;
                if self.vram_bank == 0 {
                    self.tile_map_0[index] = val;
                } else {
                    self.tile_attrs_0[index] = val;
                }

                self.map_cache_0_dirty = true;
            },
            // Background Map B
            0x9C00..=0x9FFF if self.can_access_vram() => {
                /*let base = (loc - 0x9C00) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                if self.vram_bank == 0 {
                    self.tile_map_1.set_tile_texture(x, y, val);
                } else {
                    self.tile_map_1.set_tile_attribute(x, y, val);
                }*/
                let index = (loc - 0x9C00) as usize;
                if self.vram_bank == 0 {
                    self.tile_map_1[index] = val;
                } else {
                    self.tile_attrs_1[index] = val;
                }
                
                self.map_cache_1_dirty = true;
            },
            // Sprite data
            0xFE00..=0xFE9F if self.can_access_oam() => self.object_mem.write(loc - 0xFE00, val),
            0xFF40 => self.set_lcd_control(val),
            0xFF41 => self.lcd_status.write(val),
            0xFF42 => self.scroll_y = val,
            0xFF43 => self.scroll_x = val,
            0xFF44 => self.set_lcdc_y(0),
            0xFF45 => self.ly_compare = val,
            0xFF47 => self.palettes.write(0, val),
            0xFF48 => self.palettes.write(1, val),
            0xFF49 => self.palettes.write(2, val),
            0xFF4A => self.window_y = val,
            0xFF4B => self.window_x = val,
            0xFF4F => self.vram_bank = val & 1,
            // Colour palettes
            //0xFF68 => self.colour_palettes.write_bg_index(val),
            //0xFF69 => self.colour_palettes.write_bg(val),
            //0xFF6A => self.colour_palettes.write_obj_index(val),
            //0xFF6B => self.colour_palettes.write_obj(val),
            _ => {}//unreachable!()
        }
    }
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