#version 450

#include "hexane.glsl"
#include "region.glsl"
#include "transform.glsl"
#include "voxel.glsl"

struct BuildRegionPush {
	BufferId region_id;
	BufferId transform_id;
	ImageId perlin_id;
	ImageId worley_id;
};

decl_push_constant(BuildRegionPush)

#ifdef compute

layout (local_size_x = 8, local_size_y = 8, local_size_z = 8) in;

void main() {
	Image(3D, u32) perlin_image = get_image(3D, u32, push_constant.perlin_id);
	Image(3D, u32) worley_image = get_image(3D, u32, push_constant.perlin_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);

	if(!region.dirty) 
	{
		return;
	}
	
	region.floating_origin = region.observer_position;
	
	ivec3 local_position = ivec3(gl_GlobalInvocationID);
	ivec3 world_position = region.floating_origin - ivec3(vec3(REGION_SIZE / 2)) + local_position;

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
	
	f32 perlin_noise_factor = f32(imageLoad(perlin_image, abs(i32vec3(world_position.x, 32, world_position.z)) % i32vec3(imageSize(perlin_image))).r) / f32(~0u);

	f32 height = perlin_noise_factor * 20 + 128;

	f32 vertical_compression = 4;

	f32 worley_noise_factor = f32(imageLoad(worley_image, abs(i32vec3(world_position.x, world_position.y * vertical_compression, world_position.z)) % i32vec3(imageSize(worley_image))).r) / f32(~0u);

	f32 cave_frequency = 1e-5;
	vec3 cave_offset = vec3(100, 200, 300);
	f32 cave_smudge = 1e-7;
	f32 cave_noise_factor = f32(imageLoad(perlin_image, abs(i32vec3(vec3(world_position.x * cave_frequency, 32, world_position.z * cave_frequency) + cave_offset)) % i32vec3(imageSize(perlin_image))).r) / f32(~0u);

	//dunno why this is bugged.. if this statement isnt made like this
	//then grass spawns on chunk corners
	if(worley_noise_factor > 0.5 && cave_noise_factor > 0.5 - cave_smudge) {
		change.id = u16(1);
	} else if(world_position.y > height - 1 && world_position.y < height + 1) {
		change.id = u16(2);
	} else if(world_position.y > height - 10 && world_position.y < height) {
		change.id = u16(4);
	} else if(world_position.y < height) {
		change.id = u16(3);
	} else {
		change.id = u16(1);
	}

	voxel_change(change);
}

#endif

