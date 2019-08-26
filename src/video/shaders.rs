pub mod vs {
    vulkano_shaders::shader!{
        ty: "vertex",
        src: r#"
#version 450
// View size constants
const float VIEW_WIDTH      = 20.0 / 10.0;
const float MAP_WIDTH       = 32.0 / 10.0;
const float SCROLL_X_OFFSET = VIEW_WIDTH - MAP_WIDTH;
const float TILE_WIDTH      = 1.0 / 10.0;
const float VIEW_HEIGHT     = 18.0 / 9.0;
const float MAP_HEIGHT      = 32.0 / 9.0;
const float SCROLL_Y_OFFSET = VIEW_HEIGHT - MAP_HEIGHT;
const float TILE_HEIGHT     = 1.0 / 9.0;

// Texture size constants
const uint MAX_TEX_NUM      = 384;
const uint SIGNED_OFFSET    = 256;
const uint TEX_ROW_SIZE     = 16;

// Corner enum
const uint LEFT     = 0 << 8;
const uint RIGHT    = 1 << 8;

// Flags
const uint WRAPAROUND = 1;

// Functions
vec2 calc_vertex_wraparound(vec2, uint, uint);
vec2 calc_vertex_compare(vec2, uint, uint);
vec2 calc_tex_coords(uint, uint);
vec2 get_tex_offset(uint, uint);

// Input
layout(location = 0) in vec2 position;
layout(location = 1) in uint data;

layout(push_constant) uniform PushConstants {
    vec2 tex_size;
    vec2 atlas_size;
    vec2 vertex_offset;
    uint tex_offset;
    uint palette_offset;    // 1 for window in GB mode, 8 for sprites in CGB mode
    uint flags;
} push_constants;

// Output
layout(location = 0) out vec2 texCoordOut;
layout(location = 1) out uint paletteNumOut;

void main() {
    // Vertex position offset with scroll / position
    vec2 vertex_position = position + push_constants.vertex_offset;

    if ((push_constants.flags & WRAPAROUND) != 0) {
        uint side = data & 0x100;
        uint tex_y = (data >> 9) & 7;
        vertex_position = calc_vertex_wraparound(vertex_position, side, tex_y);
    }

    gl_Position = vec4(vertex_position, 0.0, 1.0);

    texCoordOut = calc_tex_coords(data, push_constants.tex_offset);

    paletteNumOut = ((data >> 12) & 7) + push_constants.palette_offset;
}

vec2 calc_vertex_wraparound(vec2 vertex_coords, uint side, uint y) {
    vec2 compare = calc_vertex_compare(vertex_coords, side, y);
    vec2 result = vertex_coords;

    if (compare.x < SCROLL_X_OFFSET) {
        result.x += MAP_WIDTH;
    }
    if (compare.y < SCROLL_Y_OFFSET) {
        result.y += MAP_HEIGHT;
    }

    return result;
}

vec2 calc_vertex_compare(vec2 vertex_coords, uint side, uint y) {
    float y_offset = (float(y) * TILE_HEIGHT) / 8.0;
    switch(side) {
        case LEFT:  return vertex_coords - vec2(0.0, y_offset);
        default:    return vertex_coords - vec2(TILE_WIDTH, y_offset);
    }
}

vec2 calc_tex_coords(uint tex_data, uint tex_offset) {
// Unpack texture information
    uint tex_num = tex_data & 0xFF;
    uint side = tex_data & 0x100;
    uint bank_num = tex_data & 0x8000;
    uint tex_y = (tex_data >> 9) & 7;
// Get tex number in entire tile atlas
    tex_num += tex_offset;
    tex_num = tex_num >= MAX_TEX_NUM ? tex_num - SIGNED_OFFSET : tex_num;
    tex_num += bank_num == 0 ? 0 : MAX_TEX_NUM;
// Convert to 2D coords
    float x = float(tex_num % TEX_ROW_SIZE) / push_constants.atlas_size.x;
    float y = float(tex_num / TEX_ROW_SIZE) / push_constants.atlas_size.y;
    
    return vec2(x, y) + get_tex_offset(side, tex_y);
}

vec2 get_tex_offset(uint side, uint y) {
    float y_offset = (float(y) * push_constants.tex_size.y) / 8.0;
    switch (side) {
        case LEFT:  return vec2(0.0, y_offset);
        default:    return vec2(push_constants.tex_size.x, y_offset);
    }
}
"#
    }
}

pub mod fs {
    vulkano_shaders::shader!{
        ty: "fragment",
        src: r#"
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
"#
    }
}