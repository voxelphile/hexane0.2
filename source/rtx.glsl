#version 450

#include "hexane.glsl"
#include "world.glsl"
#include "vertex.glsl"
#include "transform.glsl"
#include "voxel.glsl"
#include "aabb.glsl"
#include "raycast.glsl"

#define VERTICES_PER_CUBE 6

struct RtxPush {
	BufferId info_id;
	BufferId camera_id;
	BufferId sort_id;
	BufferId transform_id;
	BufferId world_id;
	ImageId perlin_id;
	ImageId prepass_id;
};

decl_push_constant(RtxPush)

decl_buffer(
	Camera,
	{
		mat4 projection;
		mat4 inv_projection;
		f32 far;
		vec2 resolution;
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

layout(location = 0) out vec4 internal_position;
layout(location = 1) out vec4 world_position;
layout(location = 2) out flat u32 chunk;

void main() {
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	Buffer(World) world = get_buffer(World, push_constant.world_id);
	
	u32 indices[36] = u32[](1, 0, 3, 1, 3, 2, 4, 5, 6, 4, 6, 7, 2, 3, 7, 2, 7, 6, 5, 4, 0, 5, 0, 1, 6, 5, 1, 6, 1, 2, 3, 0, 4, 3, 4, 7);

	u32 i = gl_VertexIndex / 36;
	u32 j = gl_VertexIndex % 36;

	chunk = i;

	//magical plus one because player is 0
	Transform ctransform = transforms.data[chunk + 1];
	Transform transform = transforms.data[0];
	transform.position.xyz += vec3(0.4, 1.8, 0.4);

	vec3 positional_offset = clamp(offsets[indices[j]], EPSILON * EPSILON, 1 - EPSILON * EPSILON) * CHUNK_SIZE;

	positional_offset = clamp(positional_offset, world.chunks[chunk].minimum, world.chunks[chunk].maximum); 

	internal_position = vec4(positional_offset, 1.0);
	world_position = vec4(internal_position.xyz + ctransform.position.xyz, 1.0);

	gl_Position = camera.projection * inverse(compute_transform_matrix(transform)) * world_position;

}

#elif defined fragment

layout (depth_greater) out float gl_FragDepth;

layout(location = 0) in vec4 internal_position;
layout(location = 1) in vec4 world_position;
layout(location = 2) in flat u32 chunk;

layout(location = 0) out vec4 result;

void main() {
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(World) world = get_buffer(World, push_constant.world_id);
	Image(3D, u32) perlin_img = get_image(3D, u32, push_constant.perlin_id);
	Image(2D, u32) prepass_img = get_image(2D, u32, push_constant.prepass_id);

	Transform transform = transforms.data[0];
	transform.position.xyz += vec3(0.4, 1.8, 0.4);

	vec2 screenPos = (gl_FragCoord.xy / camera.resolution.xy) * 2.0 - 1.0;
	vec4 target = camera.inv_projection * vec4(screenPos, 1, 1);
	vec3 dir = (compute_transform_matrix(transform) * vec4(normalize(vec3(target.xyz) / target.w), 0)).xyz;

	vec4 color = vec4(0, 0, 0, 1);

	vec3 origin = internal_position.xyz;

	Box chunk_box;
	chunk_box.position = transforms.data[chunk + 1].position.xyz + world.chunks[chunk].minimum;
	chunk_box.dimensions = world.chunks[chunk].maximum;

	Box player_box;
	player_box.dimensions = vec3(0.8, 2, 0.8);
	player_box.position = transform.position.xyz;

	if(aabb_check(player_box, chunk_box)) {
		origin = mod(transform.position.xyz, CHUNK_SIZE);
	}

	Ray ray;
	ray.chunk_id = world.chunks[chunk].data;
	ray.origin = origin;
	ray.direction = dir;

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


		vec4 v_clip_coord = camera.projection * inverse(compute_transform_matrix(transform)) * vec4(chunk_box.position + hit.back_step, 1.0);
		float f_ndc_depth = v_clip_coord.z / v_clip_coord.w;
		gl_FragDepth = (f_ndc_depth + 1.0) * 0.5;

	} else {
		discard;
	}	

	result = color;
}

#endif
