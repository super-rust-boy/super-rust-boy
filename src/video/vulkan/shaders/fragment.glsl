#version 450

// Flags
const uint BLOCK_COLOUR = 2;

layout(location = 0) in vec2 texCoord;
layout(location = 1) in flat uint paletteNum;

layout(set = 0, binding = 0) uniform usampler2D atlas;
layout(set = 1, binding = 0) uniform Palette {
    mat4 colours[24];
} PaletteTable;

layout(push_constant) uniform PushConstants {
    vec2 tex_size;
    vec2 atlas_size;
    vec2 vertex_offset;
    uint tex_offset;
    uint palette_offset;
    uint flags;
} push_constants;

layout(location = 0) out vec4 outColor;

void main() {
    if ((push_constants.flags & BLOCK_COLOUR) != 0) {
        outColor = PaletteTable.colours[paletteNum][0];
    } else {
        uint texel = texture(atlas, texCoord).x;
        outColor = PaletteTable.colours[paletteNum][texel];
    }
}