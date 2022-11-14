#version 450

#include "hexane.glsl"

#define CHUNK_SIZE 8
#define MAX_STEP_COUNT 4096

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
		mat4 view;
	}
)

DECL_BUFFER_STRUCT(
	VertexBuffer,
	{
		u32 vertex_count;
		Vertex verts[1048576];
	}
)

layout(location = 0) flat out vec4 position;
layout(location = 1) flat out vec4 normal;
layout(location = 2) flat out vec4 color;
layout(location = 3) flat out vec4 ambient;
layout(location = 4) out vec4 uv;

void main() {
	BufferRef(CameraBuffer) camera_buffer = buffer_id_to_ref(CameraBuffer, BufferRef, push_constant.camera_buffer_id);
	BufferRef(VertexBuffer) vertex_buffer = buffer_id_to_ref(VertexBuffer, BufferRef, push_constant.vertex_buffer_id);

	u32 i = gl_VertexIndex / VERTICES_PER_CUBE;
	u32 j = gl_VertexIndex % VERTICES_PER_CUBE;
	
	position = vertex_buffer.verts[i].position;
	normal = vertex_buffer.verts[i].normal;
	color = vertex_buffer.verts[i].color;
	ambient = vertex_buffer.verts[i].ambient;
	
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
		uv.xy = vec2(uvs[j].x, 1 - uvs[j].y);
	}
	
	if(normal.xyz == vec3(0, 0, -1)) {
		u32 i[6] = u32[](4, 5, 6, 4, 6, 7);
		
		position.xyz += offsets[i[j]];	
		uv.xy = uvs[j].xy;
	}
	
	if(normal.xyz == vec3(1, 0, 0)) {
		u32 i[6] = u32[](2, 3, 7, 2, 7, 6);
		
		position.xyz += offsets[i[j]];	
		uv.xy = 1 - uvs[j].yx;
	}
	
	if(normal.xyz == vec3(-1, 0, 0)) {
		u32 i[6] = u32[](5, 4, 0, 5, 0, 1);
		
		position.xyz += offsets[i[j]];
		uv.xy = vec2(1 - uvs[j].y, uvs[j].x);
	}
	
	if(normal.xyz == vec3(0, 1, 0)) {
		u32 i[6] = u32[](6, 5, 1, 6, 1, 2);
		
		position.xyz += offsets[i[j]];	
		uv.xy = vec2(uvs[j].x, 1 - uvs[j].y);
	}
	
	if(normal.xyz == vec3(0, -1, 0)) {
		u32 i[6] = u32[](3, 0, 4, 3, 4, 7);
		
		position.xyz += offsets[i[j]];	
		//TODO
		uv.xy = uvs[j].xy;
	}
			
	gl_Position = camera_buffer.projection * inverse(camera_buffer.transform) * position;
}

#elif defined fragment

#define SHOW_UV false
#define SHOW_RGB_STRIATION false
#define SHOW_NORMALS false
#define SHOW_AO true

layout(location = 0) flat in vec4 position;
layout(location = 1) flat in vec4 normal;
layout(location = 2) flat in vec4 color;
layout(location = 3) flat in vec4 ambient;
layout(location = 4) in vec4 uv;

layout(location = 0) out vec4 result;

void main() {
    	result = color;
	
	float ao = 0;

	ao = mix(mix(ambient.z, ambient.w, uv.y), mix(ambient.y, ambient.x, uv.y), uv.x);

	if(SHOW_UV) {
		result = vec4(0, 0, 0, 1);
		result.xy = uv.xy;
	}

	if(SHOW_RGB_STRIATION) {
#define STRIATE 8
		result.xyz = mod(position.xyz, STRIATE) / STRIATE;
	}
	
	if(SHOW_NORMALS) {
		result.xyz = normal.xyz;
		if(normal.x < -1 + EPSILON || normal.y < -1 + EPSILON || normal.z < -1 + EPSILON ){
			result.xyz = 1 - result.xyz;
		}
	}
	
	if(SHOW_AO) {
		result.xyz = result.xyz - vec3(1 - ao) * 0.25;
	}
}

#endif
