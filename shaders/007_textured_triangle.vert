#version 440 core

layout (location = 1) in vec3 vInPosition;
layout (location = 2) in vec3 vInColor;
layout (location = 3) in vec2 vInUv; 

layout (binding = 5) uniform Matrices {
    mat4 mModel;
    mat4 mView;
    mat4 mProjection;
} matrices;

layout (location = 0) out vec3 vOutColor;
layout (location = 1) out vec2 vOutUv; 

void main() {
    gl_Position = matrices.mProjection * matrices.mView * matrices.mModel * vec4(vInPosition, 1.0);
    vOutColor = vInColor;
    vOutUv = vInUv;
}