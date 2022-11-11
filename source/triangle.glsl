#version 450

#include "hexane.glsl"

#ifdef vertex

struct Vertex {
	vec4 position;
	vec4 normal;
	vec4 color;
};

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

DECL_BUFFER_STRUCT(
	VertexBuffer,
	{
		Vertex verts[1048576];
	}
)

DECL_BUFFER_STRUCT(
	IndexBuffer,
	{
		u32 indices[65536];
	}
)

struct Push {
	BufferId color_buffer_id;
	BufferId camera_buffer_id;
	BufferId vertex_buffer_id;
	BufferId index_buffer_id;
};

USE_PUSH_CONSTANT(Push)

layout(location = 0) out vec4 color;

void main() {
	BufferRef(CameraBuffer) camera_buffer = buffer_id_to_ref(CameraBuffer, BufferRef, push_constant.camera_buffer_id);
	BufferRef(ColorBuffer) color_buffer = buffer_id_to_ref(ColorBuffer, BufferRef, push_constant.color_buffer_id);
	BufferRef(VertexBuffer) vertex_buffer = buffer_id_to_ref(VertexBuffer, BufferRef, push_constant.vertex_buffer_id);
	BufferRef(IndexBuffer) index_buffer = buffer_id_to_ref(IndexBuffer, BufferRef, push_constant.index_buffer_id);

	vec4 position = vertex_buffer.verts[gl_VertexIndex].position;
	vec4 normal = vertex_buffer.verts[gl_VertexIndex].normal;

	color = vertex_buffer.verts[gl_VertexIndex].color;

	if (abs(normal.x) == 1) {
		color.xyz *= 0.8;
	}

	if (abs(normal.z) == 1) {
		color.xyz *= 0.6;
	}

	gl_Position = camera_buffer.projection * camera_buffer.transform * position;
}

#elif defined fragment

layout(location = 0) in vec4 color;

layout(location = 0) out vec4 result;

void main() {
    	result = color;
}

#endif
