use crate::video::renderer::Vertex;

use vulkano::{
    buffer::CpuBufferPool,
    buffer::cpu_pool::CpuBufferPoolChunk,
    device::Device,
    memory::pool::StdMemoryPool
};

use std::sync::Arc;

pub type VertexBuffer = CpuBufferPoolChunk<Vertex, Arc<StdMemoryPool>>;

// Struct that contains the vertices to be used for rendering, in addition to the buffer pool and cached buffer chunk for rendering.
pub struct VertexGrid {
    vertices: Vec<Vertex>,
    row_len: usize,
    buffer_pool: CpuBufferPool<Vertex>,
    current_buffer: Option<VertexBuffer>
}

impl VertexGrid {
    // Make a new, 2D vertex grid of size grid_size, scaled to fit in view_size.
        // Note that this creates a background of total size grid_size scaled to fit in the view_size.
        // All parameters should be given in number of tiles.
        // The vertex position values can be offset in the vertex shader to shift this visible area around.
    pub fn new(device: &Arc<Device>, grid_size: (usize, usize), view_size: (usize, usize), tex_atlas_size: (usize, usize)) -> Self {
        let mut vertices = Vec::new();

        let tex_corner_offset = (1.0 / tex_atlas_size.0 as f32, 1.0 / tex_atlas_size.1 as f32);

        let x_frac = 2.0 / view_size.0 as f32;
        let y_frac = 2.0 / view_size.1 as f32;
        let mut lo_y = -1.0;
        let mut hi_y = lo_y + y_frac;

        for _ in 0..grid_size.1 {
            let mut lo_x = -1.0;
            let mut hi_x = lo_x + x_frac;
            for _ in 0..grid_size.0 {
                vertices.push(Vertex{ position: [lo_x, lo_y], tex_corner_offset: [0.0, 0.0], tex_num: 0 });
                vertices.push(Vertex{ position: [lo_x, hi_y], tex_corner_offset: [0.0, tex_corner_offset.1], tex_num: 0 });
                vertices.push(Vertex{ position: [hi_x, lo_y], tex_corner_offset: [tex_corner_offset.0, 0.0], tex_num: 0 });
                vertices.push(Vertex{ position: [lo_x, hi_y], tex_corner_offset: [0.0, tex_corner_offset.1], tex_num: 0 });
                vertices.push(Vertex{ position: [hi_x, lo_y], tex_corner_offset: [tex_corner_offset.0, 0.0], tex_num: 0 });
                vertices.push(Vertex{ position: [hi_x, hi_y], tex_corner_offset: [tex_corner_offset.0, tex_corner_offset.1], tex_num: 0 });

                lo_x = hi_x;
                hi_x += x_frac;
            }
            lo_y = hi_y;
            hi_y += y_frac;
        }

        VertexGrid {
            vertices: vertices,
            row_len: grid_size.0,
            buffer_pool: CpuBufferPool::vertex_buffer(device.clone()),
            current_buffer: None
        }
    }

    // Sets the tex coords for a tile.
    pub fn set_tile_texture(&mut self, tile_x: usize, tile_y: usize, tex_num: i32) {
        let y_offset = tile_y * self.row_len * 6;
        let index = y_offset + (tile_x * 6);

        for i in index..(index + 6) {
            self.vertices[i].tex_num = tex_num;
        }

        // Invalidate buffer chunk.
        self.current_buffer = None;
    }

    // Makes a new vertex buffer if the data has changed. Else, retrieves the current one.
    pub fn get_vertex_buffer(&mut self) -> VertexBuffer {
        if let Some(buf) = &self.current_buffer {
            buf.clone()
        } else {
            let buf = self.buffer_pool.chunk(
                self.vertices.iter().cloned()
            ).unwrap();
            self.current_buffer = Some(buf.clone());
            buf
        }
    }
}