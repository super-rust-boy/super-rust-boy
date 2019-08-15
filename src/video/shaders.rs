pub mod vs {
    vulkano_shaders::shader!{
        ty: "vertex",
        src: r#"
#version 450

const uint MAX_TEX_NUM = 384;
const uint SIGNED_OFFSET = 256;
const uint TEX_ROW_SIZE = 16;
const float TEX_WIDTH = 16.0;
const float TEX_HEIGHT = 24.0;

const uint TOP_LEFT = 0 << 8;
const uint BOTTOM_LEFT = 1 << 8;
const uint TOP_RIGHT = 2 << 8;
const uint BOTTOM_RIGHT = 3 << 8;

layout(location = 0) in vec2 position;
layout(location = 1) in uint data;

layout(push_constant) uniform PushConstants {
    vec2 vertex_offset;
    vec2 tex_size;
    uint tex_offset;
} push_constants;

layout(location = 0) out vec2 texCoordOut;
layout(location = 1) out uint paletteNumOut;

vec2 calc_tex_coords(uint tex_num, uint tex_offset, uint corner) {
    tex_num += tex_offset;
    tex_num = tex_num >= MAX_TEX_NUM ? tex_num - SIGNED_OFFSET : tex_num;
    float x = float(tex_num % TEX_ROW_SIZE) / TEX_WIDTH;
    float y = float(tex_num / TEX_ROW_SIZE) / TEX_HEIGHT;
    vec2 tex_corner_offset;
    switch (corner) {
        case TOP_LEFT: tex_corner_offset = vec2(0.0, 0.0); break;
        case BOTTOM_LEFT: tex_corner_offset = vec2(0.0, push_constants.tex_size.y); break;
        case TOP_RIGHT: tex_corner_offset = vec2(push_constants.tex_size.x, 0.0); break;
        default: tex_corner_offset = push_constants.tex_size; break;
    }
    return vec2(x, y) + tex_corner_offset;
}

void main() {
    // Unpack texture information
    uint tex_num = data & 0xFF;
    uint corner = data & 0x300;

    gl_Position = vec4(position + push_constants.vertex_offset, 0.0, 1.0);
    texCoordOut = calc_tex_coords(tex_num, push_constants.tex_offset, corner);

    paletteNumOut = (data & 0xC00) >> 10;
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
layout(location = 1) in flat uint paletteNum;

layout(set = 0, binding = 0) uniform usampler2D atlas;
layout(set = 1, binding = 0) uniform Palette {
    mat4 colours[3];
} PaletteTable;

layout(location = 0) out vec4 outColor;

void main() {
    uint texel = texture(atlas, texCoord).x;
    outColor = PaletteTable.colours[paletteNum][texel];
}
"#
    }
}