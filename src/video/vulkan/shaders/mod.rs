pub mod vs {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/video/vulkan/shaders/vertex.glsl"
    }
}

pub mod fs {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/video/vulkan/shaders/fragment.glsl"
    }
}

mod _refresh_files {
    #[allow(dead_code)]
    const VS: &str = include_str!("../shaders/vertex.glsl");
    #[allow(dead_code)]
    const FS: &str = include_str!("../shaders/fragment.glsl");
}