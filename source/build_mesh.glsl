#version 450

#include "hexane.glsl"
#include "octree.glsl"
#include "vertex.glsl"
#include "noise.glsl"
#include "rigidbody.glsl"
#include "transform.glsl"

struct BuildMeshPush {
	BufferId octree_id;
	BufferId vertex_id;
	ImageId perlin_id;
};

decl_push_constant(BuildMeshPush)

#ifdef compute

layout (local_size_x = 8, local_size_y = 8, local_size_z = 8) in;



float vertex_ao(vec2 side, float corner) {
	return (side.x + side.y + max(corner, side.x * side.y)) / 3.0;
}

vec4 voxel_ao(vec3 pos, vec3 d1, vec3 d2) {
	OctreeQuery query;
	query.octree_id = push_constant.octree_id;


	vec4 side = vec4(0);

	query.position = pos + d1;
	side.x = float(octree_query(query)); 
	query.position = pos + d2;
	side.y = float(octree_query(query)); 
	query.position = pos - d1;
	side.z = float(octree_query(query)); 
	query.position = pos - d2;
	side.w = float(octree_query(query));

	vec4 corner = vec4(0);

	query.position = pos + d1 + d2;
	corner.x = float(octree_query(query)); 
	query.position = pos - d1 + d2;
	corner.y = float(octree_query(query)); 
	query.position = pos - d1 - d2;
	corner.z = float(octree_query(query)); 
	query.position = pos + d1 - d2;
	corner.w = float(octree_query(query));

	vec4 ao;
	ao.x = vertex_ao(side.xy, corner.x);
	ao.y = vertex_ao(side.yz, corner.y);
	ao.z = vertex_ao(side.zw, corner.z);
	ao.w = vertex_ao(side.wx, corner.w);

	return 1.0 - ao;
}

void main() {
	Buffer(Octree) octree = get_buffer(Octree, push_constant.octree_id);
	Buffer(Vertices) verts = get_buffer(Vertices, push_constant.vertex_id);
	Image(3D, u32) perlin_img = get_image(3D, u32, push_constant.perlin_id);
	
	if(any(greaterThanEqual(gl_GlobalInvocationID, u32vec3(pow(2, octree.size))))) {
		return;
	}
	
	OctreeQuery query;
	query.octree_id = push_constant.octree_id;
	query.position = vec3(gl_GlobalInvocationID);

	bool exists = octree_query(query);

	if(!exists) {
		return;
	}

	Node node = octree.nodes[query.node_index];

	uint normal_count = 0;
	i32vec3 normals[12];

	{
		i32vec3 normal = i32vec3(0, 0, 1);
		OctreeQuery query;
		query.octree_id = push_constant.octree_id;
		query.position = vec3(gl_GlobalInvocationID + normal);

		bool exists = octree_query(query);

		if(!exists) {
			normals[normal_count] = normal;
			normal_count++;
		}
	}
	{
		i32vec3 normal = i32vec3(0, 0, -1);
		OctreeQuery query;
		query.octree_id = push_constant.octree_id;
		query.position = vec3(gl_GlobalInvocationID + normal);

		bool exists = octree_query(query);

		if(!exists) {
			normals[normal_count] = normal;
			normal_count++;
		}
	}
	{
		i32vec3 normal = i32vec3(0, 1, 0);
		OctreeQuery query;
		query.octree_id = push_constant.octree_id;
		query.position = vec3(gl_GlobalInvocationID + normal);

		bool exists = octree_query(query);

		if(!exists) {
			normals[normal_count] = normal;
			normal_count++;
		}
	}
	{
		i32vec3 normal = i32vec3(0, -1, 0);
		OctreeQuery query;
		query.octree_id = push_constant.octree_id;
		query.position = vec3(gl_GlobalInvocationID + normal);

		bool exists = octree_query(query);

		if(!exists) {
			normals[normal_count] = normal;
			normal_count++;
		}
	}
	{
		i32vec3 normal = i32vec3(1, 0, 0);
		OctreeQuery query;
		query.octree_id = push_constant.octree_id;
		query.position = vec3(gl_GlobalInvocationID + normal);

		bool exists = octree_query(query);

		if(!exists) {
			normals[normal_count] = normal;
			normal_count++;
		}
	}
	{
		i32vec3 normal = i32vec3(-1, 0, 0);
		OctreeQuery query;
		query.octree_id = push_constant.octree_id;
		query.position = vec3(gl_GlobalInvocationID + normal);

		bool exists = octree_query(query);

		if(!exists) {
			normals[normal_count] = normal;
			normal_count++;
		}
	}

	vec3 color = vec3(1);

	f32 noise_factor = f32(imageLoad(perlin_img, i32vec3(gl_GlobalInvocationID.xyz)).r) / f32(~0u);

	if(node.id == 0) {
		color = vec3(1, 0, 1);
	}
	if(node.id == 2) {
		color = mix(vec3(170, 255, 21) / 256, vec3(34, 139, 34) / 256, noise_factor);
	}
	if(node.id == 4) {
		color = mix(vec3(107, 84, 40) / 256, vec3(64, 41, 5) / 256, noise_factor);
	}

	uint i = atomicAdd(verts.count, normal_count);

	for(uint j = 0; j < normal_count; j++) {
		Vertex v;
		v.position = vec4(gl_GlobalInvocationID, 1);
		v.color = vec4(color, 1);
		v.normal = i32vec4(normals[j], 0);
		v.ambient = voxel_ao(v.position.xyz + vec3(v.normal.xyz), abs(vec3(v.normal.zxy)), abs(vec3(v.normal.yzx)));

		verts.data[i + j] = v;
	}
}

#endif
