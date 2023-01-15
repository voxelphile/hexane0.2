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

	if(!region.dirty) {
		return;
	}
	region.dirty = false;

	u32 chunk = gl_GlobalInvocationID.x / CHUNK_SIZE + gl_GlobalInvocationID.y / CHUNK_SIZE * AXIS_MAX_CHUNKS + gl_GlobalInvocationID.z / CHUNK_SIZE * AXIS_MAX_CHUNKS * AXIS_MAX_CHUNKS;

	VoxelQuery query;
	query.chunk_id = region.reserve[chunk].data;
	query.position = mod(vec3(gl_GlobalInvocationID), CHUNK_SIZE);
	
	voxel_query(query);

	VoxelChange change;
	change.chunk_id = region.chunks[chunk].data;
	change.position = mod(vec3(gl_GlobalInvocationID), CHUNK_SIZE);
	change.id = query.id;

	voxel_change(change);
	
	VoxelChange change2;
	change2.chunk_id = region.reserve[chunk].data;
	change2.position = mod(vec3(gl_GlobalInvocationID), CHUNK_SIZE);
	change2.id = u16(0);

	voxel_change(change2);
	
	region.chunks[chunk].minimum = region.reserve[chunk].minimum;
	region.chunks[chunk].maximum = region.reserve[chunk].maximum;
}

#endif

