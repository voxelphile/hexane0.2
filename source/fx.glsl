#version 450

#include "hexane.glsl"

#ifdef vertex

vec2 positions[3] = vec2[](
	vec2(-1.0, -1.0),
	vec2(-1.0,  4.0),
	vec2( 4.0, -1.0)
);

void main() {
	gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
}

#endif
