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
	vec2 resolution;
	vec2 tiles;
	BufferId info_id;
	BufferId camera_id;
	BufferId vertex_id;
	BufferId transform_id;
	BufferId world_id;
	BufferId cache_id;
	ImageId cache_pos_image;
	ImageId cache_color_image;
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

layout(location = 0) out vec4 position;
layout(location = 1) out flat vec4 tile;

vec2 positions[4] = vec2[](
	vec2(-1.0, -1.0),
	vec2(1.0, -1.0),
	vec2(1.0, 1.0),
	vec2(-1.0, 1.0)
);

void main() {
	i32 i = gl_VertexIndex / 6;
	i32 j = gl_VertexIndex % 6;

	i32 indices[6] = i32[](1, 0, 3, 1, 3, 2);

	vec2 size = 1 / push_constant.tiles;
	vec2 p = vec2(i % i32(push_constant.tiles.x), i / i32(push_constant.tiles.x));

	vec2 tp[4] = vec2[](
		p * size,
		vec2((1 + p.x) * size.x, p.y * size.y),
		vec2((1 + p.x) * size.x, (1 + p.y) * size.y),
		vec2(p.x * size.x, (1 + p.y) * size.y)
	);

	vec2 p2 = tp[indices[j]];

	if(p == vec2(1)) {
		p2 = vec2(0);
	}

	p2 *= 2;
	p2 -= 1.0;
	
	position = vec4(p2, 0, 1);
	tile = vec4(p, 0, 1);

	gl_Position = position;
}

#elif defined fragment

layout(location = 0) in vec4 position;

layout(location = 0) out vec4 result;

void main() {
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Cache) cache = get_buffer(Cache, push_constant.cache_id);
	Image(2D, f32) cache_pos_image = get_image(
		2D, 
		f32,
		push_constant.cache_pos_image
	);
	Image(2D, f32) cache_color_image = get_image(
		2D, 
		f32,
		push_constant.cache_color_image
	);

	Transform transform = transforms.data[0];

	transform.position.xyz += vec3(0.4, 1.8, 0.4);

	vec2 screenPos = (gl_FragCoord.xy / camera.resolution.xy) * 2.0 - 1.0;
	vec4 target = inverse(camera.projection) * vec4(screenPos, 1, 1);
	vec3 ray_dir = (compute_transform_matrix(transform) * vec4(normalize(vec3(target.xyz) / target.w), 0)).xyz;

	vec4 prev_pos = imageLoad(cache_pos_image, i32vec2(gl_FragCoord.xy)).rgba;
	vec4 cache_color = imageLoad(cache_color_image, i32vec2(gl_FragCoord.xy)).rgba;
	
	vec4 color = vec4(0, 0, 0, 1);

	if (cache_color.a == 0.0) {
	Ray ray;
	ray.world_id = push_constant.world_id;
	ray.origin = transform.position.xyz;
	ray.direction = ray_dir;
	ray.max_distance = 500;

	RayHit ray_hit;

	bool success = ray_cast(ray, ray_hit);


	if (success) {
		imageStore(
			cache_pos_image, 
			i32vec2(gl_FragCoord.xy),
			vec4(ray_hit.destination.xyz, 1.0)
		);

		if(ray_hit.mask.x) {
			color.xyz = vec3(0.5, 0, 0);
		}
		if(ray_hit.mask.y) {
			color.xyz = vec3(1.0, 0, 0);
		}
		if(ray_hit.mask.z) {
			color.xyz = vec3(0.75, 0, 0);
		}
	
		imageStore(
			cache_color_image, 
			i32vec2(gl_FragCoord.xy),
			vec4(color.xyz, 1.0)
		);
	} else {
		color.xyz = vec3(0, 1, 0);
	}	

	
	} else {
		color.xyz = 1 - cache_color.xyz;
	}

	result = color;
}

#endif
