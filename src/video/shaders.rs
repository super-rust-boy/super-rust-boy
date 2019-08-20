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
const uint TOP_LEFT     = 0 << 8;
const uint BOTTOM_LEFT  = 1 << 8;
const uint TOP_RIGHT    = 2 << 8;
const uint BOTTOM_RIGHT = 3 << 8;

// Functions
vec2 calc_vertex_wraparound(vec2, uint);
vec2 calc_vertex_compare(vec2, uint);
vec2 calc_tex_coords(uint, uint);
vec2 get_tex_corner_offset(uint);

// Input
layout(location = 0) in vec2 position;
layout(location = 1) in uint data;

layout(push_constant) uniform PushConstants {
    vec2 vertex_offset;
    vec2 tex_size;
    vec2 atlas_size;
    uint tex_offset;
    uint palette_offset;    // 1 for window in GB mode, 8 for sprites in CGB mode
    uint wraparound;
} push_constants;

// Output
layout(location = 0) out vec2 texCoordOut;
layout(location = 1) out uint paletteNumOut;

void main() {
    // Vertex position offset with scroll / position
    vec2 vertex_position = position + push_constants.vertex_offset;

    if (push_constants.wraparound == 1) {
        uint corner = data & 0x300;
        vertex_position = calc_vertex_wraparound(vertex_position, corner);
    }

    gl_Position = vec4(vertex_position, 0.0, 1.0);

    texCoordOut = calc_tex_coords(data, push_constants.tex_offset);

    paletteNumOut = ((data & 0x1C00) >> 10) + push_constants.palette_offset;
}

vec2 calc_vertex_wraparound(vec2 vertex_coords, uint corner) {
    vec2 compare = calc_vertex_compare(vertex_coords, corner);
    vec2 result = vertex_coords;

    if (compare.x < SCROLL_X_OFFSET) {
        result.x += MAP_WIDTH;
    }
    if (compare.y < SCROLL_Y_OFFSET) {
        result.y += MAP_HEIGHT;
    }

    return result;
}

vec2 calc_vertex_compare(vec2 vertex_coords, uint corner) {
    switch(corner) {
        case TOP_LEFT:      return vertex_coords;
        case BOTTOM_LEFT:   return vertex_coords - vec2(0.0, TILE_HEIGHT);
        case TOP_RIGHT:     return vertex_coords - vec2(TILE_WIDTH, 0.0);
        default:            return vertex_coords - vec2(TILE_WIDTH, TILE_HEIGHT);
    }
}

vec2 calc_tex_coords(uint tex_data, uint tex_offset) {
// Unpack texture information
    uint tex_num = tex_data & 0xFF;
    uint corner = tex_data & 0x300;
    uint bank_num = tex_data & 0x2000;
// Get tex number in entire tile atlas
    tex_num += tex_offset;
    tex_num = tex_num >= MAX_TEX_NUM ? tex_num - SIGNED_OFFSET : tex_num;
    tex_num += bank_num == 0 ? 0 : MAX_TEX_NUM;
// Convert to 2D coords
    float x = float(tex_num % TEX_ROW_SIZE) / push_constants.atlas_size.x;
    float y = float(tex_num / TEX_ROW_SIZE) / push_constants.atlas_size.y;
    
    return vec2(x, y) + get_tex_corner_offset(corner);
}

vec2 get_tex_corner_offset(uint corner) {
    switch (corner) {
        case TOP_LEFT:      return vec2(0.0, 0.0);
        case BOTTOM_LEFT:   return vec2(0.0, push_constants.tex_size.y);
        case TOP_RIGHT:     return vec2(push_constants.tex_size.x, 0.0);
        default:            return push_constants.tex_size;
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

layout(location = 0) in vec2 texCoord;
layout(location = 1) in flat uint paletteNum;

layout(set = 0, binding = 0) uniform usampler2D atlas;
layout(set = 1, binding = 0) uniform Palette {
    mat4 colours[16];
} PaletteTable;

layout(location = 0) out vec4 outColor;

void main() {
    uint texel = texture(atlas, texCoord).x;
    outColor = PaletteTable.colours[paletteNum][texel];
}
"#
    }
}