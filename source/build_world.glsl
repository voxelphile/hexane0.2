#version 450

#include "hexane.glsl"
#include "world.glsl"
#include "voxel.glsl"

struct BuildWorldPush {
	BufferId world_id;
	ImageId perlin_id;
};

decl_push_constant(BuildWorldPush)

#ifdef compute

layout (local_size_x = 8, local_size_y = 8, local_size_z = 8) in;

void main() {
	Image(3D, u32) perlin_image = get_image(3D, u32, push_constant.perlin_id);

	VoxelChange change;
	change.world_id = push_constant.world_id;
	change.id = u16(0);
	change.position = f32vec3(gl_GlobalInvocationID);
	
	f32 noise_factor = f32(imageLoad(perlin_image, i32vec3(gl_GlobalInvocationID.x, 32, gl_GlobalInvocationID.z)).r) / f32(~0u);

	f32 height = noise_factor * 32 + 64;

	//dunno why this is bugged.. if this statement isnt made like this
	//then grass spawns on chunk corners
	if(gl_GlobalInvocationID.y > height - 1 && gl_GlobalInvocationID.y < height + 1 ) {
		change.id = u16(2);
	} else if(gl_GlobalInvocationID.y < height) {
		change.id = u16(4);
	}

	voxel_change(change);
}

#endif
