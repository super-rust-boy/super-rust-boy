// Background and Window tile maps.
use vulkano::{
    buffer::CpuBufferPool,
    device::Device
};

use bitflags::bitflags;

use std::sync::Arc;

use super::{
    Side, Vertex, VertexBuffer
};

bitflags! {
    #[derive(Default)]
    struct Attributes: u8 {
        //const BG_OAM_PRIORITY    = bit!(7);
        const Y_FLIP             = bit!(6);
        const X_FLIP             = bit!(5);
        //const TILE_VRAM_BANK_NUM = bit!(3);
    }
}

const BG_OAM_PRIORITY: u32 = 1 << 19;

#[derive(Clone)]
enum BufferCache {
    Buffer(VertexBuffer),   // There exists a buffer that is not dirty.
    Empty,                  // This data is intentionally empty.
    Dirty                   // This data is dirty and must be recreated.
}

// Struct that contains the vertices to be used for rendering, in addition to the buffer pool.
pub struct VertexGrid {
    vertices:           Vec<Vec<Vertex>>,

    buffer_pool:        CpuBufferPool<Vertex>,
    lo_vertex_buffers:  Vec<BufferCache>,
    hi_vertex_buffers:  Vec<BufferCache>
}

impl VertexGrid {
    // Make a new, 2D vertex grid of size grid_size, scaled to fit in view_size.
        // Note that this creates a background of total size grid_size scaled to fit in the view_size.
        // All parameters should be given in number of tiles.
        // The vertex position values can be offset in the vertex shader to shift this visible area around.
    pub fn new(device: &Arc<Device>, grid_size: (usize, usize), view_size: (usize, usize)) -> Self {
        let mut vertices = Vec::new();
        let mut vertex_buffers = Vec::new();

        let x_frac = 2.0 / view_size.0 as f32;
        let y_frac = (2.0 / view_size.1 as f32) / 8.0;  // Each y tile is 8 lines high.
        let mut lo_y = -1.0;
        let mut hi_y = lo_y + y_frac;

        for y in 0..(grid_size.1 * 8) {
            let mut row_vertices = Vec::new();

            let y_coord = ((y % 8) << 9) as u32;
            let mut left_x = -1.0;
            let mut right_x = left_x + x_frac;

            for _ in 0..grid_size.0 {
                row_vertices.push(Vertex{ position: [left_x, lo_y], data: y_coord | Side::Left as u32 });
                row_vertices.push(Vertex{ position: [left_x, hi_y], data: y_coord | Side::Left as u32 });
                row_vertices.push(Vertex{ position: [right_x, lo_y], data: y_coord | Side::Right as u32 });
                row_vertices.push(Vertex{ position: [left_x, hi_y], data: y_coord | Side::Left as u32 });
                row_vertices.push(Vertex{ position: [right_x, lo_y], data: y_coord | Side::Right as u32 });
                row_vertices.push(Vertex{ position: [right_x, hi_y], data: y_coord | Side::Right as u32 });

                left_x = right_x;
                right_x += x_frac;
            }
            lo_y = hi_y;
            hi_y += y_frac;

            vertices.push(row_vertices);
            vertex_buffers.push(BufferCache::Dirty);
        }

        VertexGrid {
            vertices:           vertices,

            buffer_pool:        CpuBufferPool::vertex_buffer(device.clone()),
            lo_vertex_buffers:  vertex_buffers.clone(),
            hi_vertex_buffers:  vertex_buffers
        }
    }

    // Sets the tex number for a tile.
    pub fn set_tile_texture(&mut self, tile_x: usize, tile_y: usize, tex_num: u8) {
        let start_row = tile_y * 8;
        let end_row = start_row + 8;
        let row_index = tile_x * 6;

        for row in start_row..end_row {
            self.vertices[row][row_index].data = (self.vertices[row][row_index].data & 0xFFFFFF00) | tex_num as u32;
            self.vertices[row][row_index + 1].data = (self.vertices[row][row_index + 1].data & 0xFFFFFF00) | tex_num as u32;
            self.vertices[row][row_index + 2].data = (self.vertices[row][row_index + 2].data & 0xFFFFFF00) | tex_num as u32;
            self.vertices[row][row_index + 3].data = (self.vertices[row][row_index + 3].data & 0xFFFFFF00) | tex_num as u32;
            self.vertices[row][row_index + 4].data = (self.vertices[row][row_index + 4].data & 0xFFFFFF00) | tex_num as u32;
            self.vertices[row][row_index + 5].data = (self.vertices[row][row_index + 5].data & 0xFFFFFF00) | tex_num as u32;

            self.lo_vertex_buffers[row] = BufferCache::Dirty;
            self.hi_vertex_buffers[row] = BufferCache::Dirty;
        }
    }

    // Gets the tex number for a tile.
    pub fn get_tile_texture(&self, tile_x: usize, tile_y: usize) -> u8 {
        let row = tile_y * 8;
        let row_index = tile_x * 6;

        self.vertices[row][row_index].data as u8
    }

    // Writing and reading attributes (CGB mode).
    pub fn set_tile_attribute(&mut self, tile_x: usize, tile_y: usize, attributes: u8) {
        let start_row = tile_y * 8;
        let end_row = start_row + 8;
        let row_index = tile_x * 6;

        let flags = Attributes::from_bits_truncate(attributes);
        let data = (attributes as u32) << 12;

        let (left, right) = if flags.contains(Attributes::X_FLIP) {
            (Side::Right, Side::Left)
        } else {
            (Side::Left, Side::Right)
        };

        let y_coords = if flags.contains(Attributes::Y_FLIP) {
            (0..8).rev().collect::<Vec<u32>>()
        } else {
            (0..8).collect::<Vec<u32>>()
        };

        for (row, y) in (start_row..end_row).zip(&y_coords) {
            let y = y << 9;
            self.vertices[row][row_index].data =     (self.vertices[row][row_index].data & 0x000000FF) | data | y | left as u32;
            self.vertices[row][row_index + 1].data = (self.vertices[row][row_index + 1].data & 0x000000FF) | data | y | left as u32;
            self.vertices[row][row_index + 2].data = (self.vertices[row][row_index + 2].data & 0x000000FF) | data | y | right as u32;
            self.vertices[row][row_index + 3].data = (self.vertices[row][row_index + 3].data & 0x000000FF) | data | y | left as u32;
            self.vertices[row][row_index + 4].data = (self.vertices[row][row_index + 4].data & 0x000000FF) | data | y | right as u32;
            self.vertices[row][row_index + 5].data = (self.vertices[row][row_index + 5].data & 0x000000FF) | data | y | right as u32;

            self.lo_vertex_buffers[row] = BufferCache::Dirty;
            self.hi_vertex_buffers[row] = BufferCache::Dirty;
        }
    }

    pub fn get_tile_attribute(&self, tile_x: usize, tile_y: usize) -> u8 {
        let row = tile_y * 8;
        let row_index = tile_x * 6;

        (self.vertices[row][row_index].data >> 12) as u8
    }

    // Get a line of vertices.
    // Only retrieves the vertices that appear below the objects.
    // (This will get the whole background in GB mode).
    pub fn get_lo_vertex_buffer(&mut self, y: u8) -> Option<VertexBuffer> {
        let row = y as usize;
        let cached_buffer = &mut self.lo_vertex_buffers[row];

        match cached_buffer {
            BufferCache::Buffer(buffer) =>  Some(buffer.clone()),
            BufferCache::Empty =>           None,
            BufferCache::Dirty => {
                let tile_map = self.vertices[row].iter()
                    .cloned()
                    .filter(|v| (v.data & BG_OAM_PRIORITY) == 0)
                    .collect::<Vec<_>>();

                if tile_map.is_empty() {
                    *cached_buffer = BufferCache::Empty;
                    None
                } else {
                    let buffer = self.buffer_pool.chunk(tile_map).unwrap();
                    *cached_buffer = BufferCache::Buffer(buffer.clone());
                    Some(buffer)
                }
            }
        }
    }

    // Only retrieves the vertices that appear above the objects.
    pub fn get_hi_vertex_buffer(&mut self, y: u8) -> Option<VertexBuffer> {
        let row = y as usize;
        let cached_buffer = &mut self.hi_vertex_buffers[row];

        match cached_buffer {
            BufferCache::Buffer(buffer) =>  Some(buffer.clone()),
            BufferCache::Empty =>           None,
            BufferCache::Dirty => {
                let tile_map = self.vertices[row].iter()
                    .cloned()
                    .filter(|v| (v.data & BG_OAM_PRIORITY) != 0)
                    .collect::<Vec<_>>();

                if tile_map.is_empty() {
                    *cached_buffer = BufferCache::Empty;
                    None
                } else {
                    let buffer = self.buffer_pool.chunk(tile_map).unwrap();
                    *cached_buffer = BufferCache::Buffer(buffer.clone());
                    Some(buffer)
                }
            }
        }
    }
}