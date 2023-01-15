#version 450

#include "hexane.glsl"
#include "region.glsl"
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
	BufferId region_id;
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

#if defined(volume) && defined(vertex)

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
layout(location = 1) out vec4 region_position;
layout(location = 3) out vec4 clip_position;
layout(location = 2) out flat u32 chunk;

void main() {
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	
	u32 indices[36] = u32[](1, 0, 3, 1, 3, 2, 4, 5, 6, 4, 6, 7, 2, 3, 7, 2, 7, 6, 5, 4, 0, 5, 0, 1, 6, 5, 1, 6, 1, 2, 3, 0, 4, 3, 4, 7);

	u32 i = gl_VertexIndex / 36;
	u32 j = gl_VertexIndex % 36;

	chunk = i;

	//magical plus one because player is 0
	Transform ctransform = transforms.data[chunk + 1];
	Transform transform = transforms.data[0];
	transform.position.xyz = vec3(AXIS_MAX_CHUNKS * CHUNK_SIZE / 2);
	transform.position.xyz += fract(transforms.data[0].position.xyz - 0.5);

	vec3 positional_offset = clamp(offsets[indices[j]], pow(EPSILON, 3), 1 - pow(EPSILON, 3)) * CHUNK_SIZE;
	

	positional_offset = clamp(positional_offset, region.chunks[chunk].minimum, region.chunks[chunk].maximum); 

	internal_position = vec4(positional_offset, 1.0);
	region_position = vec4(positional_offset + ctransform.position.xyz, 1.0);

	clip_position = camera.projection * inverse(compute_transform_matrix(transform)) * region_position;

	gl_Position = clip_position;

}

#elif defined fragment

#ifdef volume

layout (depth_greater) out float gl_FragDepth;

layout(location = 0) in vec4 internal_position;
layout(location = 1) in vec4 region_position;
layout(location = 3) in vec4 clip_position;
layout(location = 2) in flat u32 chunk;

#endif

layout(location = 0) out vec4 result;

void main() {
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Image(3D, u32) perlin_img = get_image(3D, u32, push_constant.perlin_id);

	Transform transform = transforms.data[0];
	Transform eye_transform = transform;
	eye_transform.position.xyz = vec3(AXIS_MAX_CHUNKS * CHUNK_SIZE / 2);
	eye_transform.position.xyz += fract(transforms.data[0].position.xyz - 0.5);
	
	vec2 screenPos = (gl_FragCoord.xy / camera.resolution.xy) * 2.0 - 1.0;
	vec4 far = camera.inv_projection * vec4(screenPos, 1, 1);
	far.xyz /= far.w;
#ifdef fx
	vec4 near = camera.inv_projection * vec4(screenPos, 0.0, 1);
	near /= near.w;
	vec3 origin = (compute_transform_matrix(eye_transform) * near).xyz;
#elif defined(volume)
	vec3 origin = internal_position.xyz;
#endif
	vec3 dir = (compute_transform_matrix(eye_transform) * vec4(normalize(far.xyz), 0)).xyz;
	
	vec4 color = vec4(0, 0, 0, 1);
#ifdef fx
	u32 chunk = u32(origin.x) / CHUNK_SIZE + u32(origin.y) / CHUNK_SIZE * AXIS_MAX_CHUNKS + u32(origin.z) / CHUNK_SIZE * AXIS_MAX_CHUNKS * AXIS_MAX_CHUNKS;
	
	origin = mod(origin, CHUNK_SIZE);
#endif

	Ray ray;
	ray.chunk_id = region.chunks[chunk].data;
	ray.origin = origin;
	ray.direction = dir;

	RayHit hit;

	bool success = ray_cast(ray, hit);
		
	if (success) {
		f32 noise_factor = f32(imageLoad(perlin_img, (region.observer_position + ivec3(transforms.data[chunk + 1].position.xyz) + i32vec3(hit.back_step)) % i32vec3(imageSize(perlin_img))).r) / f32(~0u);
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


		vec4 v_clip_coord = camera.projection * inverse(compute_transform_matrix(eye_transform)) * vec4(transforms.data[chunk + 1].position.xyz + hit.destination, 1.0);
		float f_ndc_depth = v_clip_coord.z / v_clip_coord.w;
		gl_FragDepth = (f_ndc_depth + 1.0) * 0.5;

	} else {
		discard;
	}	

	result = color;
}

#endif
