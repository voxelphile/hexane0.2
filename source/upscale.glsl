#version 450

#include "hexane.glsl"
#include "luminosity.glsl"

#define VERTICES_PER_CUBE 6

struct UpscalePush {
	ImageId from_id;
	u32 scale;
	BufferId luminosity_id;
};

decl_push_constant(UpscalePush)

#ifdef fragment

vec3 ACESFilm(vec3 x)
{
float a = 2.51f;
float b = 0.03f;
float c = 2.43f;
float d = 0.59f;
float e = 0.14f;
return clamp((x*(a*x+b))/(x*(c*x+d)+e), 0, 1);
}  

layout(location = 0) out vec4 result;

void main() {
	Image(2D, f32) prepass_img = get_image(2D, f32, push_constant.from_id);
	Buffer(Luminosity) luminosity = get_buffer(Luminosity, push_constant.luminosity_id);

    	vec4 hdrColor = imageLoad(prepass_img, i32vec2(gl_FragCoord.xy / push_constant.scale)).rgba;
    	// exposure tone mapping
    	vec3 mapped = mix(
			vec3(1.0) - exp(-hdrColor.rgb * luminosity.exposure),
			ACESFilm(hdrColor.rgb),
			0.5
		);

	const float gamma = 2.2;
    	mapped.rgb = pow(mapped.rgb, vec3(1.0/gamma));

	mapped.rgb *= hdrColor.a;

	result = vec4(mapped, 1);
}

#endif
