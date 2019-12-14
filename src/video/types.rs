
#[derive(Default, Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub data: u32
}

#[derive(Clone, Debug)]
pub struct PushConstants {
    pub tex_size:       [f32; 2],
    pub atlas_size:     [f32; 2],
    pub vertex_offset:  [f32; 2],
    pub tex_offset:     u32,
    pub palette_offset: u32,
    pub flags:          u32
}