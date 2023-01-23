#version 450

#include "hexane.glsl"

#define U32_MAX (~0u)

struct PerlinPush {
	ImageId noise_id;
	ImageId perlin_id;
};

decl_push_constant(PerlinPush)

#ifdef compute

layout (local_size_x = 8, local_size_y = 8, local_size_z = 8) in;

vec3 random_gradient(u32vec3 position) {
	Image3Du32 noise_img = get_image(3D, u32, push_constant.noise_id);

	u32vec2 random_numbers = u32vec2(imageLoad(noise_img, i32vec3(position)).rg);
	
	f32 alpha = f32(random_numbers.x) / f32(U32_MAX) * 3.14159265;
	f32 beta = f32(random_numbers.y) / f32(U32_MAX) * 3.14159265;

	return vec3(
		cos(alpha) * cos(beta),
		sin(beta),
		sin(alpha) * cos(beta)
	);
}

float dot_grid_gradient(u32vec3 i, vec3 p) {
	vec3 a = vec3(i);
	vec3 b = vec3(p);

	return dot(random_gradient(i), b - a);
}

void main() {
	Image3Du32 perlin_img = get_image(3D, u32, push_constant.perlin_id);
	Image3Du32 noise_img = get_image(3D, u32, push_constant.noise_id);

	if(any(greaterThanEqual(gl_GlobalInvocationID, imageSize(perlin_img)))) {
		return;	
	}

	f32vec3 sample_basis = f32vec3(imageSize(perlin_img) / imageSize(noise_img));

	f32vec3 p = f32vec3(gl_GlobalInvocationID) / sample_basis;

	u32vec3 m0 = u32vec3(floor(p));

	u32vec3 m1 = m0 + 1;

	f32vec3 s = p - f32vec3(m0);

	float n0, n1, ix0, ix1, jx0, jx1, k;
	u32 value;

	n0 = dot_grid_gradient(u32vec3(m0.x, m0.y, m0.z), p);
	n1 = dot_grid_gradient(u32vec3(m1.x, m0.y, m0.z), p);
	ix0 = mix(n0, n1, s.x);

	n0 = dot_grid_gradient(u32vec3(m0.x, m1.y, m0.z), p);
	n1 = dot_grid_gradient(u32vec3(m1.x, m1.y, m0.z), p);
	ix1 = mix(n0, n1, s.x);

	jx0 = mix(ix0, ix1, s.y); 
	
	n0 = dot_grid_gradient(u32vec3(m0.x, m0.y, m1.z), p);
	n1 = dot_grid_gradient(u32vec3(m1.x, m0.y, m1.z), p);
	ix0 = mix(n0, n1, s.x);

	n0 = dot_grid_gradient(u32vec3(m0.x, m1.y, m1.z), p);
	n1 = dot_grid_gradient(u32vec3(m1.x, m1.y, m1.z), p);
	ix1 = mix(n0, n1, s.x);

	jx1 = mix(ix0, ix1, s.y); 

	k = mix(jx0, jx1, s.z);

	value = u32(((k + 1) / 2) * U32_MAX);

	imageStore(perlin_img, i32vec3(gl_GlobalInvocationID), u32vec4(value, 0, 0, 0));
}

#endif
