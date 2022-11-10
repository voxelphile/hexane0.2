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

DECL_BUFFER_STRUCT(
	CameraBuffer,
	{
		mat4 projection;
		mat4 transform;
		mat4 view;
	}
)

struct Push {
	BufferId color_buffer_id;
	BufferId camera_buffer_id;
};

USE_PUSH_CONSTANT(Push)

layout(location = 0) out vec4 color;

void main() {
	BufferRef(CameraBuffer) camera_buffer = buffer_id_to_ref(CameraBuffer, BufferRef, push_constant.camera_buffer_id);
	BufferRef(ColorBuffer) color_buffer = buffer_id_to_ref(ColorBuffer, BufferRef, push_constant.color_buffer_id);
	
	color = color_buffer.colors[gl_VertexIndex];

	gl_Position = camera_buffer.projection * camera_buffer.transform * vec4(positions[gl_VertexIndex], 0.0, 1.0);
}

#elif defined fragment

layout(location = 0) in vec4 color;

layout(location = 0) out vec4 result;

void main() {
    	result = color;
}

#endif
