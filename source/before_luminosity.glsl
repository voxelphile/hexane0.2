#version 450

#include "hexane.glsl"
#include "luminosity.glsl"

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
	luminosity.lum = 0; 
}

#endif

