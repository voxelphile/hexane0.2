#version 450

#include "hexane.glsl"

#define VERTICES_PER_CUBE 6

struct UpscalePush {
	ImageId from_id;
	u32 scale;
};

decl_push_constant(UpscalePush)

#ifdef fragment

layout(location = 0) out vec4 result;

void main() {
	Image(2D, f32) perlin_img = get_image(2D, f32, push_constant.from_id);
	
	result = imageLoad(perlin_img, i32vec2(gl_FragCoord.xy / push_constant.scale));
}

#endif
