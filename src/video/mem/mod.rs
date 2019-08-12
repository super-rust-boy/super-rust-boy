mod tilemem;
mod vertexgrid;
mod palette;

use vulkano::device::{
    Device,
    Queue
};

use bitflags::bitflags;

use std::sync::Arc;

use crate::mem::MemDevice;

use tilemem::*;
use vertexgrid::*;
use palette::*;

const TILE_SIZE: usize = 8;         // Width / Height of a tile in pixels.
const TILE_DATA_WIDTH: usize = 16;  // Width of the tile data in tiles.
const TILE_DATA_HEIGHT: usize = 24; // Height of the tile data in tiles.
const MAP_SIZE: usize = 32;         // Width / Height of bg/window tile maps.
const VIEW_WIDTH: usize = 20;       // Width of visible area.
const VIEW_HEIGHT: usize = 18;      // Height of visible area.

const OFFSET_FRAC_X: f32 = (MAP_SIZE as f32 / VIEW_WIDTH as f32) / 256.0;   // Mult with an offset to get the amount to offset by
const OFFSET_FRAC_Y: f32 = (MAP_SIZE as f32 / VIEW_HEIGHT as f32) / 256.0;  // Mult with an offset to get the amount to offset by

bitflags! {
    #[derive(Default)]
    struct LCDControl: u8 {
        const Enable                = 0b10000000;
        const WindowTileMapSelect   = 0b01000000;
        const WindowDisplayEnable   = 0b00100000;
        const TileDataSelect        = 0b00010000;
        const BGTileMapSelect       = 0b00001000;
        const ObjSize               = 0b00000100;
        const ObjDisplayEnable      = 0b00000010;
        const DisplayPriority       = 0b00000001;
    }
}

bitflags! {
    #[derive(Default)]
    struct LCDStatus: u8 {
        const CoincedenceInt        = 0b01000000;
        const OAMInt                = 0b00100000;
        const VBlankInt             = 0b00010000;
        const HBlankInt             = 0b00001000;
        const CoincedenceFlag       = 0b00000100;
    }
}

// Video memory layer
pub struct VideoMem {
    // Raw tile mem and tile maps
    tile_mem: TileAtlas,
    tile_map_0: VertexGrid,
    tile_map_1: VertexGrid,
    // sprites?

    // Flags / registers
    lcd_control: LCDControl,
    lcd_status: LCDStatus,
    scroll_y: u8,
    scroll_x: u8,
    lcdc_y: u8,
    ly_compare: u8,

    bg_palette: Palette,
    obj_0_palette: Palette,
    obj_1_palette: Palette,

    window_y: u8,
    window_x: u8
}

impl VideoMem {
    pub fn new(device: &Arc<Device>) -> Self {
        VideoMem {
            tile_mem: TileAtlas::new((TILE_DATA_WIDTH, TILE_DATA_HEIGHT), TILE_SIZE),
            tile_map_0: VertexGrid::new(device, (VIEW_WIDTH, VIEW_HEIGHT), (TILE_DATA_WIDTH, TILE_DATA_HEIGHT), (MAP_SIZE, MAP_SIZE)),
            tile_map_1: VertexGrid::new(device, (VIEW_WIDTH, VIEW_HEIGHT), (TILE_DATA_WIDTH, TILE_DATA_HEIGHT), (MAP_SIZE, MAP_SIZE)),

            lcd_control: LCDControl::default(),
            lcd_status: LCDStatus::default(),
            scroll_y: 0,
            scroll_x: 0,
            lcdc_y: 0,
            ly_compare: 0,

            bg_palette: Palette::new_monochrome(device),
            obj_0_palette: Palette::new_monochrome(device),
            obj_1_palette: Palette::new_monochrome(device),

            window_y: 0,
            window_x: 0
        }
    }

    // Get background vertices.
    pub fn get_background(&mut self) -> VertexBuffer {
        if !self.lcd_control.contains(LCDControl::BGTileMapSelect) {
            self.tile_map_0.get_vertex_buffer()
        } else {
            self.tile_map_1.get_vertex_buffer()
        }
    }

    // Get window
    pub fn get_window(&mut self) -> Option<VertexBuffer> {
        if self.lcd_control.contains(LCDControl::WindowDisplayEnable) {
            Some(if !self.lcd_control.contains(LCDControl::WindowTileMapSelect) {
                self.tile_map_0.get_vertex_buffer()
            } else {
                self.tile_map_1.get_vertex_buffer()
            })
        } else {
            None
        }
    }
    // Get sprites

    // Get tile atlas
    pub fn get_tile_atlas(&mut self, queue: Arc<Queue>) -> (TileImage, TileFuture) {
        self.tile_mem.make_image(queue)
    }

    // Get palette for bg and window
    pub fn get_palette_buffer(&mut self) -> PaletteBuffer {
        self.bg_palette.get_buffer()
    }

    // Get push constants
    pub fn get_bg_scroll(&self) -> [f32; 2] {
        [self.scroll_x as f32 * -OFFSET_FRAC_X, self.scroll_y as f32 * -OFFSET_FRAC_Y]
    }

    pub fn get_window_position(&self) -> [f32; 2] {
        [(self.window_x as f32 - 7.0) * OFFSET_FRAC_X, self.window_y as f32 * OFFSET_FRAC_Y]
    }

    pub fn get_tile_data_offset(&self) -> i32 {
        if self.lcd_control.contains(LCDControl::TileDataSelect) {
            0
        } else {
            256
        }
    }
}

impl MemDevice for VideoMem {
    fn read(&self, loc: u16) -> u8 {
        match loc {
            // Raw tile data
            0x8000...0x97FF => {
                let base = (loc - 0x8000) as usize;

                let mut ret = 0;

                if base % 2 == 0 {  // Lower bit
                    let base_pixel = base * 4;
                    for i in 0..8 {
                        ret |= (self.tile_mem.atlas[base_pixel + i] & 0b01) << (7 - i);
                    }
                } else {    // Upper bit
                    let base_pixel = (base - 1) * 4;
                    for i in 0..8 {
                        ret |= ((self.tile_mem.atlas[base_pixel + i] & 0b10) >> 1) << (7 - i);
                    }
                }

                ret
            },
            // Background Map A
            0x9800...0x9BFF => {
                0
            },
            // Background Map B
            0x9C00...0x9FFF => {
                0
            },
            // Sprite data
            0xFE00...0xFE9F => {
                0
            },
            _ => 0
        }
    }

    fn write(&mut self, loc: u16, val: u8) {
        match loc {
            // Raw tile data
            0x8000...0x97FF => {
                let base = (loc - 0x8000) as usize;

                if base % 2 == 0 {  // Lower bit
                    let base_pixel = base * 4;
                    for i in 0..8 {
                        let lower_bit = (val >> (7 - i)) & 1;
                        let upper_bit = self.tile_mem.atlas[base_pixel + i] & 0b10;
                        self.tile_mem.atlas[base_pixel + i] = upper_bit | lower_bit;
                    }
                } else {    // Upper bit
                    let base_pixel = (base - 1) * 4;
                    for i in 0..8 {
                        let upper_bit = (val >> (7 - i)) & 1;
                        let lower_bit = self.tile_mem.atlas[base_pixel + i] & 0b01;
                        self.tile_mem.atlas[base_pixel + i] = (upper_bit << 1) | lower_bit;
                    }
                }
            },
            // Background Map A
            0x9800...0x9BFF => {
                let base = (loc - 0x9800) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                self.tile_map_0.set_tile_texture(x, y, val as i32);
            },
            // Background Map B
            0x9C00...0x9FFF => {
                let base = (loc - 0x9C00) as usize;
                let x = base % 0x20;
                let y = base / 0x20;

                self.tile_map_1.set_tile_texture(x, y, val as i32);
            },
            // Sprite data
            0xFE00...0xFE9F => {

            },
            _ => {}
        }
    }
}