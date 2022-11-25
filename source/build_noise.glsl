#version 450

#include "hexane.glsl"
#include "noise.glsl"

struct BuildNoisePush {
	BufferId mersenne_id;
	ImageId noise_id;
};

decl_push_constant(BuildNoisePush)

#ifdef compute

layout (local_size_x = 8, local_size_y = 8, local_size_z = 8) in;

void main() {
	Image3Du32 noise_img = get_image(3D, u32, push_constant.noise_id);

	u32 value = random(push_constant.mersenne_id);

	imageStore(noise_img, i32vec3(gl_GlobalInvocationID), u32vec4(value, 0, 0, 0));
}

#endif
