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

	if(!region.dirty) {
		return;
	}
	region.dirty = false;

	u32 chunk = gl_GlobalInvocationID.x  + gl_GlobalInvocationID.y * AXIS_MAX_CHUNKS + gl_GlobalInvocationID.z * AXIS_MAX_CHUNKS * AXIS_MAX_CHUNKS;
	ImageId temp = region.reserve[chunk].data;
	region.reserve[chunk].data = region.chunks[chunk].data;
	region.chunks[chunk].data = temp;
	
	region.chunks[chunk].minimum = region.reserve[chunk].minimum;
	region.chunks[chunk].maximum = region.reserve[chunk].maximum;
}

#endif

