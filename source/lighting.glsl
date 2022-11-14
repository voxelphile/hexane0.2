#version 450

#include "hexane.glsl"

#define VERTICES_PER_CUBE 6

struct Push {
	BufferId color_buffer_id;
	BufferId camera_buffer_id;
	BufferId vertex_buffer_id;
	BufferId octree_buffer_id;
};

USE_PUSH_CONSTANT(Push)
	
#ifdef vertex

vec3 offsets[8] = vec3[](
        vec3(0, 0, 1),
        vec3(0, 1, 1),
        vec3(1, 1, 1),
        vec3(1, 0, 1),
        vec3(0, 0, 0),
        vec3(0, 1, 0),
        vec3(1, 1, 0),
	vec3(1, 0, 0)
);

struct Vertex {
	vec4 position;
	vec4 normal;
	vec4 color;
	vec4 ambient;
};

DECL_BUFFER_STRUCT(
	CameraBuffer,
	{
		mat4 projection;
		mat4 transform;
		mat4 viev;
	}
)

DECL_BUFFER_STRUCT(
	VertexBuffer,
	{
		u32 vertex_count;
		Vertex verts[1048576];
	}
)

void main() {
	BufferRef(CameraBuffer) camera_buffer = buffer_id_to_ref(CameraBuffer, BufferRef, push_constant.camera_buffer_id);
	BufferRef(VertexBuffer) vertex_buffer = buffer_id_to_ref(VertexBuffer, BufferRef, push_constant.vertex_buffer_id);

	u32 i = gl_VertexIndex / VERTICES_PER_CUBE;
	u32 j = gl_VertexIndex % VERTICES_PER_CUBE;
	
	vec4 position = vertex_buffer.verts[i].position;
	vec4 normal = vertex_buffer.verts[i].normal;
	
	vec2 uvs[6] = vec2[](
		vec2(0, 0),
		vec2(0, 1),
		vec2(1, 1),
		vec2(0, 0),
		vec2(1, 1),
		vec2(1, 0)
	);

	if(normal.xyz == vec3(0, 0, 1)) {
		u32 i[6] = u32[](1, 0, 3, 1, 3, 2);
	
		position.xyz += offsets[i[j]];	
	}
	
	if(normal.xyz == vec3(0, 0, -1)) {
		u32 i[6] = u32[](4, 5, 6, 4, 6, 7);
		
		position.xyz += offsets[i[j]];	
	}
	
	if(normal.xyz == vec3(1, 0, 0)) {
		u32 i[6] = u32[](2, 3, 7, 2, 7, 6);
		
		position.xyz += offsets[i[j]];	
	}
	
	if(normal.xyz == vec3(-1, 0, 0)) {
		u32 i[6] = u32[](5, 4, 0, 5, 0, 1);
		
		position.xyz += offsets[i[j]];	
	}
	
	if(normal.xyz == vec3(0, 1, 0)) {
		u32 i[6] = u32[](6, 5, 1, 6, 1, 2);
		
		position.xyz += offsets[i[j]];	
	}
	
	if(normal.xyz == vec3(0, -1, 0)) {
		u32 i[6] = u32[](3, 0, 4, 3, 4, 7);
		
		position.xyz += offsets[i[j]];	
	}
			
	gl_Position = camera_buffer.projection * inverse(camera_buffer.transform) * position;
}

#elif defined fragment

void main() {
	gl_FragDepth = gl_FragCoord.z;
}

#endif
