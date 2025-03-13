#version 140    
out vec4 color;

#pragma vscode_glsllint_stage : frag

uniform vec4 color_obj;

void main() {
    color = vec4(color_obj);
}