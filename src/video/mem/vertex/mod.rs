pub mod sprite;
pub mod tilemap;

pub use super::super::types::Vertex;

use vulkano::{
    buffer::cpu_pool::CpuBufferPoolChunk,
    memory::pool::StdMemoryPool
};

use std::sync::Arc;

// Vertex data:
// 0-7: Tile number
// 8: Side
// 9-11: Tile Y Coord
// 12-14: Palette
// 15: VRAM bank
// 16-18: other attributes
// 19: priority

#[derive(Copy, Clone)]
pub enum Side {
    Left     = 0 << 8,
    Right    = 1 << 8,
}

pub type VertexBuffer = CpuBufferPoolChunk<Vertex, Arc<StdMemoryPool>>;