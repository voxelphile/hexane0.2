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

layout (local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

void main() {
	Image(3D, u32) perlin_image = get_image(3D, u32, push_constant.perlin_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);

	u32 chunk = gl_GlobalInvocationID.x + gl_GlobalInvocationID.y * AXIS_MAX_CHUNKS + gl_GlobalInvocationID.z * AXIS_MAX_CHUNKS * AXIS_MAX_CHUNKS;
	/*
	uvec3 minimum = uvec3(CHUNK_SIZE);
	uvec3 maximum = uvec3(0);
	
	Transform region_transform;
	ivec3 diff = region.floating_origin - region.observer_position;
	region_transform.position.xyz = vec3(REGION_SIZE / 2) - vec3(diff);
	region_transform.position.xyz += transforms.data[0].position.xyz - region.observer_position;

	vec3 chunk_pos = transforms.data[1 + chunk].position.xyz - vec3(AXIS_MAX_CHUNKS * CHUNK_SIZE / 2) + region_transform.position.xyz;

	for(u32 x = 0; x < CHUNK_SIZE; x++) {
		for(u32 y = 0; y < CHUNK_SIZE; y++) {
			for(u32 z = 0; z < CHUNK_SIZE; z++) {
				uvec3 internal_position = uvec3(x, y, z);

				VoxelQuery query;
				query.region_data = region.data;
				query.position = uvec3(chunk_pos) + internal_position;

				if(voxel_query(query)) {
					minimum = min(minimum, internal_position);
					maximum = max(maximum, internal_position + 1);
				}
			}
		}
	}

	*/

	region.chunks[chunk].minimum = uvec3(0);
	region.chunks[chunk].maximum = uvec3(CHUNK_SIZE);
		
	transforms.data[1 + chunk].position.xyz = vec3(gl_GlobalInvocationID) * CHUNK_SIZE;
}

#endif

