#version 440 core

layout (location = 0) in vec3 vInColor;
layout (location = 1) in vec2 vInUv;

layout (location = 0) out vec4 vOutColor;

layout (binding = 10) uniform sampler2D pixels; 

void main() {
    vOutColor = texture(pixels, vInUv);
}