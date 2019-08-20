pub mod dynamic;
pub mod r#static;

pub use crate::video::PaletteColours;

use vulkano::{
    buffer::cpu_pool::CpuBufferPoolChunk,
    memory::pool::StdMemoryPool
};

use std::sync::Arc;

pub type PaletteBuffer = CpuBufferPoolChunk<PaletteColours, Arc<StdMemoryPool>>;
