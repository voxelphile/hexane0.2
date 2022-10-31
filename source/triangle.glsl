#version 450

#ifdef vertex

vec2 positions[3] = vec2[](
    vec2(0.0, -0.5),
    vec2(0.5, 0.5),
    vec2(-0.5, 0.5)
);

void main() {
    gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
}

#elif defined fragment

layout(location = 0) out vec4 result;

void main() {
    result = vec4(1.0, 0.0, 0.0, 1.0);
}

#endif
