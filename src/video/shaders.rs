pub const VERTEX_SRC: &'static str = r#"
    #version 140

    in vec2 position;
    in vec2 texcoord;

    out vec2 frag_tex;

    void main() {
        frag_tex = texcoord;
        gl_Position = vec4(position, 0.0, 1.0);
    }
"#;

pub const FRAGMENT_SRC: &'static str = r#"
    #version 140

    in vec2 frag_tex;

    out vec4 outColor;

    uniform sampler2D tex;

    void main() {
        outColor = texture(tex, frag_tex);
    }
"#;

