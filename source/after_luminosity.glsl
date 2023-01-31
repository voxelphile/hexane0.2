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

layout (local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

void main() {
	Buffer(Luminosity) luminosity = get_buffer(Luminosity, push_constant.luminosity_id);
	Buffer(Info) info = get_buffer(Info, push_constant.info_id);
	Image(2D, f32) prepass_image = get_image(2D, f32, push_constant.prepass_id);

	u32 pixels = imageSize(prepass_image).x * imageSize(prepass_image).y;

	f32 avg_lum = f32(luminosity.lum) / f32(pixels);

	luminosity.target_exposure = 1 / max(avg_lum, 0.00001);

	f32 rate = exp2(1);
	
	luminosity.exposure = mix(luminosity.exposure, luminosity.target_exposure, exp2(-rate * info.delta_time));
}

#endif

