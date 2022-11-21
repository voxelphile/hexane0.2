#version 450

#include "hexane.glsl"
#include "octree.glsl"
#include "rigidbody.glsl"
#include "transform.glsl"

struct Vertex {
	vec4 position;
	u32vec4 normal;
	vec4 color;
	vec4 ambient;
};

DECL_BUFFER_STRUCT(
	VertexBuffer,
	{
		u32vec4 vertex_count;
		Vertex verts[255680];
	}
)

struct BuildMeshPush {
	BufferId octree_buffer_id;
	BufferId vertex_buffer_id;
};

USE_PUSH_CONSTANT(BuildMeshPush)

#ifdef compute

layout (local_size_x = 8, local_size_y = 8, local_size_z = 8) in;

void main() {
	BufferRef(OctreeBuffer) octree_buffer = buffer_id_to_ref(OctreeBuffer, BufferRef, push_constant.octree_buffer_id);
	BufferRef(VertexBuffer) vertex_buffer = buffer_id_to_ref(VertexBuffer, BufferRef, push_constant.vertex_buffer_id);
	
	if(gl_GlobalInvocationID.x > pow(2, octree_buffer.octree.size)) {
		return;
	}
	if(gl_GlobalInvocationID.y > pow(2, octree_buffer.octree.size)) {
		return;
	}
	if(gl_GlobalInvocationID.z > pow(2, octree_buffer.octree.size)) {
		return;
	}
	
	OctreeQuery query;
	query.octree_buffer_id = push_constant.octree_buffer_id;
	query.position = vec3(gl_GlobalInvocationID);

	bool exists = octree_query(query);

	if(!exists) {
		return;
	}

	uint normal_count = 0;
	u32vec3 normals[12];

	{
		u32vec3 normal = u32vec3(0, 0, 1);
		OctreeQuery query;
		query.octree_buffer_id = push_constant.octree_buffer_id;
		query.position = vec3(gl_GlobalInvocationID + normal);

		bool exists = octree_query(query);

		if(!exists && !any(lessThan(query.position, vec3(0))) && !any(greaterThan(query.position, vec3(511)))) {
			normals[normal_count] = normal;
			normal_count++;
		}
	}
	{
		u32vec3 normal = u32vec3(0, 0, -1);
		OctreeQuery query;
		query.octree_buffer_id = push_constant.octree_buffer_id;
		query.position = vec3(gl_GlobalInvocationID + normal);

		bool exists = octree_query(query);

		if(!exists && !any(lessThan(query.position, vec3(0))) && !any(greaterThan(query.position, vec3(511)))) {
			normals[normal_count] = normal;
			normal_count++;
		}
	}
	{
		u32vec3 normal = u32vec3(0, 1, 0);
		OctreeQuery query;
		query.octree_buffer_id = push_constant.octree_buffer_id;
		query.position = vec3(gl_GlobalInvocationID + normal);

		bool exists = octree_query(query);

		if(!exists && !any(lessThan(query.position, vec3(0))) && !any(greaterThan(query.position, vec3(511)))) {
			normals[normal_count] = normal;
			normal_count++;
		}
	}
	{
		u32vec3 normal = u32vec3(0, -1, 0);
		OctreeQuery query;
		query.octree_buffer_id = push_constant.octree_buffer_id;
		query.position = vec3(gl_GlobalInvocationID + normal);

		bool exists = octree_query(query);

		if(!exists && !any(lessThan(query.position, vec3(0))) && !any(greaterThan(query.position, vec3(511)))) {
			normals[normal_count] = normal;
			normal_count++;
		}
	}
	{
		u32vec3 normal = u32vec3(1, 0, 0);
		OctreeQuery query;
		query.octree_buffer_id = push_constant.octree_buffer_id;
		query.position = vec3(gl_GlobalInvocationID + normal);

		bool exists = octree_query(query);

		if(!exists && !any(lessThan(query.position, vec3(0))) && !any(greaterThan(query.position, vec3(511)))) {
			normals[normal_count] = normal;
			normal_count++;
		}
	}
	{
		u32vec3 normal = u32vec3(-1, 0, 0);
		OctreeQuery query;
		query.octree_buffer_id = push_constant.octree_buffer_id;
		query.position = vec3(gl_GlobalInvocationID + normal);

		bool exists = octree_query(query);

		if(!exists && !any(lessThan(query.position, vec3(0))) && !any(greaterThan(query.position, vec3(511)))) {
			normals[normal_count] = normal;
			normal_count++;
		}
	}
	
	uint i = atomicAdd(vertex_buffer.vertex_count.x, normal_count);

	for(uint j = 0; j < normal_count; j++) {
		Vertex vert;
		vert.position = vec4(gl_GlobalInvocationID, 1);
		vert.color = vec4(1, 0, 0, 1);
		vert.normal = u32vec4(normals[j], 0);
		vert.ambient = vec4(1, 1, 1, 1);
		vertex_buffer.verts[i + j] = vert;
	}
}

#endif
