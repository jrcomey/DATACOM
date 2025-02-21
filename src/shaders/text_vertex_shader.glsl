#version 140
in vec3 position;
in vec2 tex_coords;
out vec2 v_uv;

uniform vec2 screen_size; 
uniform mat4 model;

void main() {
    vec2 ndc_pos = (position.xy / screen_size) * 2.0;
    // ndc_pos.x *= screen_size.x / screen_size.y;
    ndc_pos *=  screen_size.y / screen_size.x;
    // ndc_pos.x *= screen_size.y/screen_size.x;
    // ndc_pos.x *= 3.0 / 3.141592 * screen_size.x / screen_size.y;
    vec4 transform = model * (vec4(ndc_pos, 0.0, 1.0));
    // transform.y *= screen_size.y / screen_size.x;
    gl_Position = transform;
    v_uv = tex_coords;
}