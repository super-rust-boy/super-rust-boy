mod patternmem;
mod vertex;
mod palette;

use vulkano::device::{
    Device,
    Queue
};

use cgmath::Vector4;

use bitflags::bitflags;

use std::sync::Arc;

use crate::mem::MemDevice;

use super::sgbpalettes::SGBPalette;

use patternmem::*;
use vertex::{
    VertexBuffer,
    tilemap::VertexGrid,
    sprite::ObjectMem
};
use palette::{
    r#static::StaticPaletteMem,
    dynamic::DynamicPaletteMem
};

pub use patternmem::TileImage;
pub use palette::PaletteBuffer;

const TILE_DATA_WIDTH: usize = 16;      // Width of the tile data in tiles.
const TILE_DATA_HEIGHT_GB: usize = 24;  // Height of the tile data in tiles for GB.
const TILE_DATA_HEIGHT_CGB: usize = 48; // Height of the tile data in tiles for GB Color.
const MAP_SIZE: usize = 32;             // Width / Height of bg/window tile maps.
const VIEW_WIDTH: usize = 20;           // Width of visible area.
const VIEW_HEIGHT: usize = 18;          // Height of visible area.

const OFFSET_FRAC_X: f32 = (MAP_SIZE as f32 / VIEW_WIDTH as f32) / 128.0;   // Mult with an offset to get the amount to offset by
const OFFSET_FRAC_Y: f32 = (MAP_SIZE as f32 / VIEW_HEIGHT as f32) / 128.0;  // Mult with an offset to get the amount to offset by

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
    tile_mem:   TileAtlas,
    tile_map_0: VertexGrid,
    tile_map_1: VertexGrid,
    object_mem: ObjectMem,

    // Flags / registers
    lcd_control:    LCDControl,
    pub lcd_status: LCDStatus,
    scroll_y:       u8,
    scroll_x:       u8,
    lcdc_y:         u8,
    ly_compare:     u8,

    window_y:       u8,
    window_x:       u8,

    palettes:           StaticPaletteMem,

    // CGB things
    cgb_mode:           bool,
    colour_palettes:    DynamicPaletteMem,
    vram_bank:          u8,

    // Misc
    clear_colour:   Vector4<f32>,
    cycle_count:    u32,
}

impl VideoMem {
    pub fn new(device: &Arc<Device>, palette: SGBPalette, cgb_mode: bool) -> Self {
        VideoMem {
            tile_mem:   TileAtlas::new(
                (TILE_DATA_WIDTH, if cgb_mode {TILE_DATA_HEIGHT_CGB} else {TILE_DATA_HEIGHT_GB}),
            ),
            tile_map_0: VertexGrid::new(device, (MAP_SIZE, MAP_SIZE), (VIEW_WIDTH, VIEW_HEIGHT)),
            tile_map_1: VertexGrid::new(device, (MAP_SIZE, MAP_SIZE), (VIEW_WIDTH, VIEW_HEIGHT)),
            object_mem: ObjectMem::new(device),

            lcd_control:    LCDControl::ENABLE,
            lcd_status:     LCDStatus::new(),
            scroll_y:       0,
            scroll_x:       0,
            lcdc_y:         0,
            ly_compare:     0,

            window_y:       0,
            window_x:       0,

            palettes:           StaticPaletteMem::new(device, palette),
            cgb_mode:           cgb_mode,
            colour_palettes:    DynamicPaletteMem::new(device),
            vram_bank:          0,

            clear_colour:   palette.get_colour_0(),
            cycle_count:    0
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
    pub fn get_clear_colour(&self) -> [f32; 4] {
        if self.display_enabled() && !self.cgb_mode {
            self.palettes.get_colour_0()
        } else {
            self.clear_colour
        }.into()
    }

    // For rendering background.
    pub fn get_background_priority(&self) -> bool {
        self.lcd_control.contains(LCDControl::DISPLAY_PRIORITY)
    }

    // Get background vertices for given y line.
    pub fn get_background(&mut self, y: u8) -> Option<VertexBuffer> {
        if self.cgb_mode || self.lcd_control.contains(LCDControl::DISPLAY_PRIORITY) {
            if !self.lcd_control.contains(LCDControl::BG_TILE_MAP_SELECT) {
                self.tile_map_0.get_lo_vertex_buffer(y)
            } else {
                self.tile_map_1.get_lo_vertex_buffer(y)
            }
        } else {
            None
        }
    }

    // Get background vertices with priority bit set for given y line.
    pub fn get_background_hi(&mut self, y: u8) -> Option<VertexBuffer> {
        if self.cgb_mode {
            if !self.lcd_control.contains(LCDControl::BG_TILE_MAP_SELECT) {
                self.tile_map_0.get_hi_vertex_buffer(y)
            } else {
                self.tile_map_1.get_hi_vertex_buffer(y)
            }
        } else {
            None
        }
    }

    // Get window for given y line.
    pub fn get_window(&mut self, y: u8) -> Option<VertexBuffer> {
        if self.lcd_control.contains(LCDControl::DISPLAY_PRIORITY | LCDControl::WINDOW_DISPLAY_ENABLE) {
            if !self.lcd_control.contains(LCDControl::WINDOW_TILE_MAP_SELECT) {
                self.tile_map_0.get_lo_vertex_buffer(y)
            } else {
                self.tile_map_1.get_lo_vertex_buffer(y)
            }
        } else {
            None
        }
    }

    // Get window vertices with priority bit set for given y line.
    pub fn get_window_hi(&mut self, y: u8) -> Option<VertexBuffer> {
        if self.cgb_mode {
            if !self.lcd_control.contains(LCDControl::WINDOW_TILE_MAP_SELECT) {
                self.tile_map_0.get_hi_vertex_buffer(y)
            } else {
                self.tile_map_1.get_hi_vertex_buffer(y)
            }
        } else {
            None
        }
    }

    // Get low-priority sprites (below the background) for given y line.
    pub fn get_sprites_lo(&mut self, y: u8) -> Option<VertexBuffer> {
        if self.lcd_control.contains(LCDControl::OBJ_DISPLAY_ENABLE) {
            let large_sprites = self.lcd_control.contains(LCDControl::OBJ_SIZE);
            self.object_mem.get_lo_vertex_buffer(y, large_sprites, self.cgb_mode)
        } else {
            None
        }
    }

    // Get high-priority sprites (above the background) for given y line.
    pub fn get_sprites_hi(&mut self, y: u8) -> Option<VertexBuffer> {
        if self.lcd_control.contains(LCDControl::OBJ_DISPLAY_ENABLE) {
            let large_sprites = self.lcd_control.contains(LCDControl::OBJ_SIZE);
            self.object_mem.get_hi_vertex_buffer(y, large_sprites, self.cgb_mode)
        } else {
            None
        }
    }

    // Get tile atlas
    pub fn get_tile_atlas(&mut self, device: &Arc<Device>, queue: &Arc<Queue>) -> (TileImage, TileFuture) {
        self.tile_mem.get_image(device, queue)
    }

    // Get palettes
    pub fn get_palette_buffer(&mut self) -> PaletteBuffer {
        if self.cgb_mode {
            self.colour_palettes.get_buffer()
        } else {
            self.palettes.get_buffer()
        }
    }

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
    pub fn get_tile_size(&self) -> [f32; 2] {
        self.tile_mem.get_tile_size()
    }

    // Get the size of the atlas (in tiles).
    pub fn get_atlas_size(&self) -> [f32; 2] {
        self.tile_mem.get_atlas_size()
    }

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
                    self.tile_mem.get_pixel_lower_row(get_base_pixel(base))
                } else {            // Upper bit
                    self.tile_mem.get_pixel_upper_row(get_base_pixel(base - 1))
                }
            },
            // Background Map A
            0x9800..=0x9BFF if self.can_access_vram() => {
                let base = (loc - 0x9800) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                if self.vram_bank == 0 {
                    self.tile_map_0.get_tile_texture(x, y)
                } else {
                    self.tile_map_0.get_tile_attribute(x, y)
                }
            },
            // Background Map B
            0x9C00..=0x9FFF if self.can_access_vram() => {
                let base = (loc - 0x9C00) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                if self.vram_bank == 0 {
                    self.tile_map_1.get_tile_texture(x, y)
                } else {
                    self.tile_map_1.get_tile_attribute(x, y)
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
            0xFF68 => self.colour_palettes.read_bg_index(),
            0xFF69 => self.colour_palettes.read_bg(),
            0xFF6A => self.colour_palettes.read_obj_index(),
            0xFF6B => self.colour_palettes.read_obj(),
            _ => 0xFF
        }
    }

    fn write(&mut self, loc: u16, val: u8) {
        match loc {
            // Raw tile data
            0x8000..=0x97FF if self.can_access_vram() => {
                let base = (loc - 0x8000) as usize + (self.vram_bank as usize * 0x1800);

                if base % 2 == 0 {  // Lower bit
                    self.tile_mem.set_pixel_lower_row(get_base_pixel(base), val);
                } else {            // Upper bit
                    self.tile_mem.set_pixel_upper_row(get_base_pixel(base - 1), val);
                }
            },
            // Background Map A
            0x9800..=0x9BFF if self.can_access_vram() => {
                let base = (loc - 0x9800) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                if self.vram_bank == 0 {
                    self.tile_map_0.set_tile_texture(x, y, val);
                } else {
                    self.tile_map_0.set_tile_attribute(x, y, val);
                }
            },
            // Background Map B
            0x9C00..=0x9FFF if self.can_access_vram() => {
                let base = (loc - 0x9C00) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                if self.vram_bank == 0 {
                    self.tile_map_1.set_tile_texture(x, y, val);
                } else {
                    self.tile_map_1.set_tile_attribute(x, y, val);
                }
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
            0xFF68 => self.colour_palettes.write_bg_index(val),
            0xFF69 => self.colour_palettes.write_bg(val),
            0xFF6A => self.colour_palettes.write_obj_index(val),
            0xFF6B => self.colour_palettes.write_obj(val),
            _ => {}//unreachable!()
        }
    }
}

#[inline]
fn get_base_pixel(base: usize) -> usize {
    // TODO: shift and mask these for a more efficient operation...
    let tile_x = (base % 0x100) / 0x10;
    let tile_y = base / 0x100;

    let pixel_row_num = (base / 2) % 8;

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