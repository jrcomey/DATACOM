#version 140    
out vec4 color;

uniform vec4 color_obj;

void main() {
    color = vec4(color_obj);
}