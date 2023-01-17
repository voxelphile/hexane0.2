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

	if(!region.dirty) 
	{
		return;
	}
	
	region.observer_position = ivec3(transforms.data[0].position.xyz);
	
	uvec3 local_position = gl_GlobalInvocationID;
	ivec3 world_position = region.observer_position + ivec3(local_position) - ivec3(vec3(CHUNK_SIZE * AXIS_MAX_CHUNKS / 2));

	u32 chunk = local_position.x / CHUNK_SIZE + local_position.y / CHUNK_SIZE * AXIS_MAX_CHUNKS + local_position.z / CHUNK_SIZE * AXIS_MAX_CHUNKS * AXIS_MAX_CHUNKS;

	VoxelQuery query;
	query.region_data = region.reserve;
	query.position = local_position;

	if(voxel_query(query)) {
		return;
	}

	VoxelChange change;
	change.region_data = region.reserve;
	change.id = u16(0);
	change.position = local_position;
	
	f32 noise_factor = f32(imageLoad(perlin_image, abs(i32vec3(world_position.x, 32, world_position.z)) % i32vec3(imageSize(perlin_image))).r) / f32(~0u);

	f32 height = noise_factor * 20 + 128;

	//dunno why this is bugged.. if this statement isnt made like this
	//then grass spawns on chunk corners
	if(world_position.y > height - 1 && world_position.y < height + 1) {
		change.id = u16(2);
	} else if(world_position.y < height) {
		change.id = u16(4);
	} else {
		change.id = u16(1);
	}

	voxel_change(change);
}

#endif

