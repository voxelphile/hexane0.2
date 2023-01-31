#version 450

#include "hexane.glsl"
#include "luminosity.glsl"
#include "info.glsl"

struct LuminosityPush {
	BufferId luminosity_id;
	BufferId info_id;
	ImageId prepass_id;
};

decl_push_constant(LuminosityPush)

#ifdef compute

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

void main() {
	Buffer(Luminosity) luminosity = get_buffer(Luminosity, push_constant.luminosity_id);
	Buffer(Info) info = get_buffer(Info, push_constant.info_id);
	Image(2D, f32) prepass_image = get_image(2D, f32, push_constant.prepass_id);

	if(any(greaterThanEqual(uvec2(gl_GlobalInvocationID), uvec2(imageSize(prepass_image))))) {
		return;
	}
	
	vec3 rgb = imageLoad(prepass_image, i32vec2(gl_GlobalInvocationID)).rgb;
	vec3 lum = vec3(0.2126, 0.7152, 0.0722);
	vec3 rgb_lum = rgb * lum;

	f32 max_lum = max(rgb_lum.x, max(rgb_lum.y, rgb_lum.z));
	u32 lum_level = u32(MAX_LUMINOSITY_LEVELS * max_lum); 
	
	atomicAdd(luminosity.lum, lum_level);
}

#endif

