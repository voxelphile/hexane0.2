#version 450

#include "hexane.glsl"
#include "world.glsl"
#include "vertex.glsl"
#include "transform.glsl"
#include "bits.glsl"
#include "voxel.glsl"
#include "raycast.glsl"

#define VERTICES_PER_CUBE 6

struct RtxPush {
	BufferId info_id;
	BufferId camera_id;
	BufferId vertex_id;
	BufferId transform_id;
	BufferId world_id;
	ImageId perlin_id;
};

decl_push_constant(RtxPush)

decl_buffer(
	Camera,
	{
		mat4 projection;
		vec2 resolution;
	}
)

decl_buffer(
	Cache,
	{
		Transform last;
	}
)
	
#ifdef vertex

vec2 positions[3] = vec2[](
	vec2(-1.0, -1.0),
	vec2(-1.0,  4.0),
	vec2( 4.0, -1.0)
);

void main() {
	gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
}

#elif defined fragment

layout(location = 0) out vec4 result;

void main() {
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Image(3D, u32) perlin_img = get_image(3D, u32, push_constant.perlin_id);

	Transform transform = transforms.data[0];

	transform.position.xyz += vec3(0.4, 1.8, 0.4);

	vec2 screenPos = (gl_FragCoord.xy / camera.resolution.xy) * 2.0 - 1.0;
	vec4 target = inverse(camera.projection) * vec4(screenPos, 1, 1);
	vec3 dir = (compute_transform_matrix(transform) * vec4(normalize(vec3(target.xyz) / target.w), 0)).xyz;

	vec4 color = vec4(0, 0, 0, 1);

	Ray ray;
	ray.world_id = push_constant.world_id;
	ray.origin = transform.position.xyz;
	ray.direction = dir;
	ray.max_distance = 100;

	RayHit hit;

	bool success = ray_cast(ray, hit);


	if (success) {
		f32 noise_factor = f32(imageLoad(perlin_img, i32vec3(hit.back_step) % i32vec3(imageSize(perlin_img))).r) / f32(~0u);
		if(hit.id == 0) {
			color.xyz = vec3(1, 0, 1);
		}
		if(hit.id == 2) {
			color.xyz = mix(vec3(170, 255, 21) / 256, vec3(34, 139, 34) / 256, noise_factor);
		}
		if(hit.id == 4) {
			color.xyz = mix(vec3(107, 84, 40) / 256, vec3(64, 41, 5) / 256, noise_factor);
		}
		if(hit.mask.x) {
			color.xyz *= 0.5;
		}
		if(hit.mask.z) {
			color.xyz *= 0.75;
		}
	} else {
		color.xyz = vec3(0.2, 0.4, 0.8);
	}	

	result = color;
}

#endif
