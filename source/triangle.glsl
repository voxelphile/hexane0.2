#version 450

#include "hexane.glsl"

#ifdef vertex

vec2 positions[3] = vec2[](
    	vec2(0.0, -0.5),
    	vec2(0.5, 0.5),
    	vec2(-0.5, 0.5)
);

DECL_BUFFER_STRUCT(
	ColorBuffer,
	{
		vec4 colors[3];
	}
)

struct Push {
	BufferId color_buffer_id;
};

USE_PUSH_CONSTANT(Push)

layout(location = 0) out vec4 color;

void main() {
	color = buffer_id_to_ref(ColorBuffer, BufferRef, push_constant.color_buffer_id).colors[gl_VertexIndex];
	gl_Position = vec4(positions[gl_VertexIndex], 0.5, 1.0);
}

#elif defined fragment

layout(location = 0) in vec4 color;

layout(location = 0) out vec4 result;

void main() {
    	result = color;
}

#endif
