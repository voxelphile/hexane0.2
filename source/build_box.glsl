#version 450

#include "hexane.glsl"
#include "world.glsl"
#include "transform.glsl"
#include "voxel.glsl"

struct BuildWorldPush {
	BufferId world_id;
	BufferId transform_id;
	ImageId perlin_id;
};

decl_push_constant(BuildWorldPush)

#ifdef compute

layout (local_size_x = 8, local_size_y = 8, local_size_z = 8) in;

void main() {
	Image(3D, u32) perlin_image = get_image(3D, u32, push_constant.perlin_id);
	Buffer(World) world = get_buffer(World, push_constant.world_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);

	u32 chunk = gl_GlobalInvocationID.x + gl_GlobalInvocationID.y * AXIS_MAX_CHUNKS + gl_GlobalInvocationID.z * AXIS_MAX_CHUNKS * AXIS_MAX_CHUNKS;
	
	vec3 minimum = vec3(CHUNK_SIZE);
	vec3 maximum = vec3(0);

	for(u32 x = 0; x < CHUNK_SIZE; x++) {
		for(u32 y = 0; y < CHUNK_SIZE; y++) {
			for(u32 z = 0; z < CHUNK_SIZE; z++) {
				VoxelQuery query;
				query.chunk_id = world.chunks[chunk].data;
				query.position = vec3(x, y, z);

				if(voxel_query(query)) {
					minimum = min(minimum, query.position);
					maximum = max(maximum, query.position + 1);
				}
			}
		}
	}

	world.chunks[chunk].minimum = minimum;
	world.chunks[chunk].maximum = maximum;
}

#endif

