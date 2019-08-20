pub mod sprite;
pub mod tilemap;

pub use crate::video::renderer::Vertex;

use vulkano::{
    buffer::cpu_pool::CpuBufferPoolChunk,
    memory::pool::StdMemoryPool
};

use std::sync::Arc;

// Vertex data:
// 0-7: Tile number
// 8-9: Corner
// 10-12: Palette
// 13: VRAM bank
// 15-17: Ignore
// 18: priority

pub enum Corner {
    TopLeft     = 0 << 8,
    BottomLeft  = 1 << 8,
    TopRight    = 2 << 8,
    BottomRight = 3 << 8
}

pub type VertexBuffer = CpuBufferPoolChunk<Vertex, Arc<StdMemoryPool>>;