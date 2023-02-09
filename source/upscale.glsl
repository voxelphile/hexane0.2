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

#define DOF_SAMPLES 32
float GOLDEN_RATIO = 3.141592 * (3.0 - sqrt(5.0));

f32 dof_circle_of_confusion(f32 pixel_depth, f32 focus_depth) {
    // https://developer.nvidia.com/gpugems/gpugems/part-iv-image-processing/chapter-23-depth-field-survey-techniques
    f32 aperture = min(1.0, focus_depth * focus_depth * 0.5);
    f32 focal_length = 0.01;
    f32 depth_ratio = (pixel_depth - focus_depth) / (focus_depth - focal_length);
    return abs(aperture * focal_length * depth_ratio / pixel_depth);
}

float dof_rand(float p) {
    // https://www.shadertoy.com/view/4djSRW - Dave Hoskins
    vec2 p2 = fract(vec2(p) * vec2(4.438975, 3.972973));
    p2 += dot(p2.yx, p2.xy + 19.19);
    return fract(p2.x * p2.y);
}

void main() {
	Image(2D, f32) prepass_img = get_image(2D, f32, push_constant.from_id);
	Buffer(Luminosity) luminosity = get_buffer(Luminosity, push_constant.luminosity_id);

    	vec4 hdrColor = imageLoad(prepass_img, i32vec2(gl_FragCoord.xy / push_constant.scale)).rgba;
	
    	// exposure tone mapping
	vec3 mapped = mix(
			vec3(1.0) - exp(-hdrColor.rgb * luminosity.exposure),
			ACESFilm(hdrColor.rgb),
			0.4
		);
	
	const float gamma = 2.2;
    	mapped.rgb = pow(mapped.rgb, vec3(1.0/gamma));


	result = vec4(mapped.rgb, 1);
}

#endif
