#version 450

#include "hexane.glsl"
#include "region.glsl"
#include "transform.glsl"
#include "voxel.glsl"

struct BuildRegionPush {
	BufferId region_id;
	BufferId transform_id;
	ImageId perlin_id;
};

decl_push_constant(BuildRegionPush)

#ifdef compute

layout (local_size_x = 8, local_size_y = 8, local_size_z = 8) in;

void main() {
	Image(3D, u32) perlin_image = get_image(3D, u32, push_constant.perlin_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);

	if(region.observer_position != ivec3(transforms.data[0].position.xyz)) {
		region.dirty = true;
	} else{
		return;
	}

	ivec3 diff = region.observer_position - ivec3(transforms.data[0].position.xyz);

	ivec3 from_position = ivec3(gl_GlobalInvocationID.xyz);
	ivec3 to_position = from_position + diff;

	if(any(lessThan(to_position, ivec3(0))) || any(greaterThan(to_position, ivec3(AXIS_MAX_CHUNKS * CHUNK_SIZE)))) {
		return;	
	}

	u32 from_chunk = from_position.x / CHUNK_SIZE + from_position.y / CHUNK_SIZE * AXIS_MAX_CHUNKS + from_position.z / CHUNK_SIZE * AXIS_MAX_CHUNKS * AXIS_MAX_CHUNKS;
	u32 to_chunk = to_position.x / CHUNK_SIZE + to_position.y / CHUNK_SIZE * AXIS_MAX_CHUNKS + to_position.z / CHUNK_SIZE * AXIS_MAX_CHUNKS * AXIS_MAX_CHUNKS;

	VoxelQuery query;
	query.chunk_id = region.chunks[from_chunk].data;
	query.position = mod(vec3(from_position), CHUNK_SIZE);
	
	if(!voxel_query(query)) {
		return;
	}

	VoxelChange change;
	change.chunk_id = region.reserve[to_chunk].data;
	change.position = mod(vec3(to_position), CHUNK_SIZE);
	change.id = query.id;

	voxel_change(change);
	
	VoxelChange change2;
	change2.chunk_id = region.chunks[from_chunk].data;
	change2.position = mod(vec3(from_position), CHUNK_SIZE);
	change2.id = u16(0);

	voxel_change(change2);
}

#endif

