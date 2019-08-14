pub mod vs {
    vulkano_shaders::shader!{
        ty: "vertex",
        src: r#"
#version 450

const int MAX_TEX_NUM = 384;
const int SIGNED_OFFSET = 256;
const int TEX_ROW_SIZE = 16;
const float TEX_WIDTH = 16.0;
const float TEX_HEIGHT = 24.0;

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 tex_corner_offset;
layout(location = 2) in int tex_num;

layout(push_constant) uniform PushConstants {
    vec2 vertex_offset;
    int tex_offset;
} push_constants;

layout(location = 0) out vec2 texCoordOut;

vec2 calc_tex_coords(int tex_num, int offset) {
    tex_num += offset;
    tex_num = tex_num >= MAX_TEX_NUM ? tex_num - SIGNED_OFFSET : tex_num;
    float x = float(tex_num % TEX_ROW_SIZE) / TEX_WIDTH;
    float y = float(tex_num / TEX_ROW_SIZE) / TEX_HEIGHT;
    return vec2(x, y) + tex_corner_offset;
}

void main() {
    gl_Position = vec4(position + push_constants.vertex_offset, 0.0, 1.0);
    texCoordOut = calc_tex_coords(tex_num, push_constants.tex_offset);
}
"#
    }
}

pub mod fs {
    vulkano_shaders::shader!{
        ty: "fragment",
        src: r#"
#version 450

layout(location = 0) in vec2 texCoord;

layout(set = 0, binding = 0) uniform usampler2D atlas;
layout(set = 1, binding = 0) uniform Palette {
    mat4 colours;
} PaletteTable;

layout(location = 0) out vec4 outColor;

void main() {
    uint texel = texture(atlas, texCoord).x;
    outColor = PaletteTable.colours[texel];
}
"#
    }
}