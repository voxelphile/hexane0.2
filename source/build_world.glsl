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
	
	region.observer_position = ivec3(transforms.data[0].position.xyz);
	
	ivec3 local_position = ivec3(gl_GlobalInvocationID);
	ivec3 world_position = region.observer_position + local_position;


	u32 chunk = local_position.x / CHUNK_SIZE + local_position.y / CHUNK_SIZE * AXIS_MAX_CHUNKS + local_position.z / CHUNK_SIZE * AXIS_MAX_CHUNKS * AXIS_MAX_CHUNKS;
	
	if (all(equal(mod(f32vec3(local_position) / f32(CHUNK_SIZE), 1), vec3(0)))) {
		transforms.data[1 + chunk].position.xyz = vec3(local_position);
	}

	VoxelQuery query;
	query.chunk_id = region.reserve[chunk].data;
	query.position = mod(f32vec3(local_position), CHUNK_SIZE);


	VoxelChange change;
	change.chunk_id = region.reserve[chunk].data;
	change.id = u16(0);
	change.position = mod(f32vec3(local_position), CHUNK_SIZE);
	
	f32 noise_factor = f32(imageLoad(perlin_image, i32vec3(world_position.x, 32, world_position.z) % i32vec3(imageSize(perlin_image))).r) / f32(~0u);

	f32 height = noise_factor * 20 + 64;

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

