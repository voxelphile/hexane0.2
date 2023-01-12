#version 450

#include "hexane.glsl"
#include "world.glsl"
#include "vertex.glsl"
#include "transform.glsl"
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

vec3 offsets[8] = vec3[](
        vec3(0, 0, 1),
        vec3(0, 1, 1),
        vec3(1, 1, 1),
        vec3(1, 0, 1),
        vec3(0, 0, 0),
        vec3(0, 1, 0),
        vec3(1, 1, 0),
	vec3(1, 0, 0)
);

layout(location = 0) out vec4 position;
layout(location = 1) out flat u32 chunk;

void main() {
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	
	u32 indices[36] = u32[](1, 0, 3, 1, 3, 2, 4, 5, 6, 4, 6, 7, 2, 3, 7, 2, 7, 6, 5, 4, 0, 5, 0, 1, 6, 5, 1, 6, 1, 2, 3, 0, 4, 3, 4, 7);

	u32 i = gl_VertexIndex / 36;
	u32 j = gl_VertexIndex % 36;

	chunk = i;

	//magical plus one because player is 0
	Transform ctransform = transforms.data[chunk + 1];
	Transform transform = transforms.data[0];

	vec3 positional_offset = offsets[indices[j]] * CHUNK_SIZE;

	position = vec4(positional_offset * ctransform.position.xyz, 1.0);

	gl_Position = camera.projection * inverse(compute_transform_matrix(transform)) * position;
}

#elif defined fragment

layout(location = 0) in vec4 position;
layout(location = 1) in flat u32 chunk;

layout(location = 0) out vec4 result;

void main() {
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(World) world = get_buffer(World, push_constant.world_id);
	Image(3D, u32) perlin_img = get_image(3D, u32, push_constant.perlin_id);

	Transform transform = transforms.data[0];

	transform.position.xyz += vec3(0.4, 1.8, 0.4);

	vec2 screenPos = (gl_FragCoord.xy / camera.resolution.xy) * 2.0 - 1.0;
	vec4 target = inverse(camera.projection) * vec4(screenPos, 1, 1);
	vec3 dir = (compute_transform_matrix(transform) * vec4(normalize(vec3(target.xyz) / target.w), 0)).xyz;

	vec4 color = vec4(0, 0, 0, 1);

	Ray ray;
	ray.chunk_id = world.chunks[chunk];
	ray.origin = position.xyz;
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
	} else {
		color.xyz = vec3(0.2, 0.4, 0.8);
	}	

	result = color;
}

#endif
