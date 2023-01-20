#version 450

#include "hexane.glsl"

#define U32_MAX (~0u)
#define U16_MAX (~0u)

struct PerlinPush {
	ImageId noise_id;
	ImageId worley_id;
};

decl_push_constant(PerlinPush)

#ifdef compute

layout (local_size_x = 8, local_size_y = 8, local_size_z = 8) in;

vec3 calculate_position(i32vec3 noise_pos) {
	Image3Du32 worley_img = get_image(3D, u32, push_constant.worley_id);
	Image3Du32 noise_img = get_image(3D, u32, push_constant.noise_id);
	
	vec3 cell_size = imageSize(worley_img) / imageSize(noise_img);

	u32vec3 random_numbers = u32vec3(imageLoad(noise_img, i32vec3(noise_pos)).rgb);

	vec3 cell_pos = noise_pos + vec3(random_numbers) / U32_MAX; 
	cell_pos *= cell_size;

	return cell_pos;
}

void main() {
	Image3Du32 worley_img = get_image(3D, u32, push_constant.worley_id);
	Image3Du32 noise_img = get_image(3D, u32, push_constant.noise_id);
	
	vec3 cell_size = imageSize(worley_img) / imageSize(noise_img);

	if(any(greaterThanEqual(gl_GlobalInvocationID, imageSize(worley_img)))) {
		return;	
	}

	f32 dist = U32_MAX;

	i32vec3 voxel_pos = i32vec3(gl_GlobalInvocationID);

	for(int x = -1; x <= 1; x++) {
	for(int y = -1; y <= 1; y++) {
	for(int z = -1; z <= 1; z++) {
		i32vec3 noise_pos = voxel_pos / i32vec3(cell_size);
		vec3 cell_pos = calculate_position(noise_pos + i32vec3(x, y, z));
		dist = min(dist, distance(cell_pos, voxel_pos));
	}
	}
	}

	dist /= imageSize(worley_img).x;

	imageStore(worley_img, i32vec3(gl_GlobalInvocationID), u32vec4(dist * U32_MAX));

}

#endif
