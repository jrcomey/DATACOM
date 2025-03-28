#version 140
in vec3 position;
uniform mat4 model;
uniform mat4 view;
uniform mat4 perspective;
uniform mat4 vp;

void main() {
    gl_Position = vp * perspective * view * model * vec4(position, 1.0);
}