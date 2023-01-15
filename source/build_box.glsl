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
	
	vec3 minimum = vec3(CHUNK_SIZE);
	vec3 maximum = vec3(0);

	for(u32 x = 0; x < CHUNK_SIZE; x++) {
		for(u32 y = 0; y < CHUNK_SIZE; y++) {
			for(u32 z = 0; z < CHUNK_SIZE; z++) {
				VoxelQuery query;
				query.chunk_id = region.chunks[chunk].data;
				query.position = vec3(x, y, z);

				if(voxel_query(query)) {
					minimum = min(minimum, query.position);
					maximum = max(maximum, query.position + 1);
				}
			}
		}
	}

	region.chunks[chunk].minimum = minimum;
	region.chunks[chunk].maximum = maximum;
}

#endif

