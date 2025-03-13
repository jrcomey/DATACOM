#version 140
in vec2 v_uv;
out vec4 color;

#pragma vscode_glsllint_stage : frag

uniform sampler2D tex;  // Texture for font
uniform vec4 color_obj; // Font color

void main() {
    // Extract alpha from the texture's alpha channel
    float alpha = texture(tex, v_uv).a;
    
    // Apply the font color with the sampled alpha
    color = vec4(color_obj.rgb, alpha);
}