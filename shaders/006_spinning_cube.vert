#version 440 core

layout (location = 1) in vec3 vPosition;
layout (location = 2) in vec3 vInColor;

layout (location = 0) out vec3 vOutColor;

layout (binding = 5) uniform Matrices {
    mat4 MVP;
} matrices;

void main() {
    vOutColor = vInColor;
    gl_Position = matrices.MVP * vec4(vPosition, 1.0);
}