#version 450

#include "hexane.glsl"
#include "region.glsl"
#include "vertex.glsl"
#include "transform.glsl"
#include "voxel.glsl"
#include "aabb.glsl"
#include "raycast.glsl"
#include "ao.glsl"

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
layout(location = 1) out vec4 chunk_position;
layout(location = 4) out vec4 region_position;
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

	vec3 positional_offset = clamp(offsets[indices[j]], pow(EPSILON, 3), 1 - pow(EPSILON, 3)) * CHUNK_SIZE;
	
	positional_offset = clamp(positional_offset, region.chunks[chunk].minimum, region.chunks[chunk].maximum); 

	internal_position = vec4(positional_offset, 1.0);
	chunk_position = vec4(positional_offset + ctransform.position.xyz, 1.0);
	ivec3 diff = region.floating_origin - region.observer_position;
	region_position = vec4(chunk_position.xyz - vec3(AXIS_MAX_CHUNKS * CHUNK_SIZE / 2) + vec3(REGION_SIZE / 2) - vec3(diff) , 1);
	region_position.xyz += transforms.data[0].position.xyz - region.observer_position;

	clip_position = camera.projection * inverse(compute_transform_matrix(transform)) * chunk_position;

	gl_Position = clip_position;

}

#elif defined fragment

#ifdef volume

layout (depth_greater) out float gl_FragDepth;

layout(location = 0) in vec4 internal_position;
layout(location = 1) in vec4 chunk_position;
layout(location = 4) in vec4 region_position;
layout(location = 3) in vec4 clip_position;
layout(location = 2) in flat u32 chunk;

#endif

#define ENABLE_AO true

layout(location = 0) out vec4 result;

void main() {
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Image(3D, u32) perlin_img = get_image(3D, u32, push_constant.perlin_id);

	Transform transform = transforms.data[0];
	Transform chunk_transform = transform;
	chunk_transform.position.xyz = vec3(AXIS_MAX_CHUNKS * CHUNK_SIZE / 2);
	
	Transform region_transform = transform;
	ivec3 diff = region.floating_origin - region.observer_position;
	region_transform.position.xyz = vec3(REGION_SIZE / 2) - vec3(diff);
	region_transform.position.xyz += transforms.data[0].position.xyz - region.observer_position;


	
	vec2 screenPos = (gl_FragCoord.xy / camera.resolution.xy) * 2.0 - 1.0;
	vec4 far = camera.inv_projection * vec4(screenPos, 1, 1);
	far /= far.w;
#ifdef fx
	vec4 near = camera.inv_projection * vec4(screenPos, 0, 1);
	near /= near.w;
	vec3 chunk_origin = (compute_transform_matrix(chunk_transform) * near).xyz;
	vec3 origin = (compute_transform_matrix(region_transform) * near).xyz;
	
	u32 chunk = u32(chunk_origin.x) / CHUNK_SIZE + u32(chunk_origin.y) / CHUNK_SIZE * AXIS_MAX_CHUNKS + u32(chunk_origin.z) / CHUNK_SIZE * AXIS_MAX_CHUNKS * AXIS_MAX_CHUNKS;
#elif defined(volume)
	vec3 origin = region_position.xyz;
	vec3 chunk_origin = chunk_position.xyz;
#endif
	vec3 dir = (compute_transform_matrix(region_transform) * vec4(normalize(far.xyz), 0)).xyz;
	
	vec4 color = vec4(0, 0, 0, 1);

	vec3 chunk_pos = transforms.data[1 + chunk].position.xyz - vec3(uvec3(vec3(AXIS_MAX_CHUNKS * CHUNK_SIZE / 2))) + vec3(uvec3(vec3(REGION_SIZE / 2))) - vec3(diff);

	Ray ray;
	ray.region = region;
	ray.origin = origin;
	ray.direction = dir;
	ray.max_distance = sqrt(f32(3)) * CHUNK_SIZE + 1; 
	ray.minimum = chunk_pos + region.chunks[chunk].minimum - 1;
	ray.maximum = chunk_pos + region.chunks[chunk].maximum + 1;

	RayHit hit;

	bool success = ray_cast(ray, hit);
		
	if (success) {
		if(distance(hit.destination, vec3(region_transform.position.xyz)) > VIEW_DISTANCE) {
			discard;
		}

		f32 noise_factor = f32(imageLoad(perlin_img, i32vec3(abs(round(vec3(region.floating_origin) - vec3(REGION_SIZE / 2) + hit.destination + vec3(0.5)))) % i32vec3(imageSize(perlin_img))).r) / f32(~0u);
		if(hit.id == 0) {
			color.xyz = vec3(1, 0, 1);
		}
		if(hit.id == 2) {
			color.xyz = mix(vec3(170, 255, 21) / 256, vec3(34, 139, 34) / 256, noise_factor);
		}
		if(hit.id == 4) {
			color.xyz = mix(vec3(107, 84, 40) / 256, vec3(64, 41, 5) / 256, noise_factor);
		}

		vec4 ambient = voxel_ao(
			region.data,
			hit.back_step, 
			abs(hit.normal.zxy), 
			abs(hit.normal.yzx)
			);

		float ao = 0;

		if (ENABLE_AO) {
			ao = mix(mix(ambient.z, ambient.w, hit.uv.x), mix(ambient.y, ambient.x, hit.uv.x), hit.uv.y);
		}

		color.xyz = color.xyz - vec3(1 - ao) * 0.25;

		vec3 sun_pos = vec3(10000);
		Ray shadow_ray;
		shadow_ray.region = region;
		shadow_ray.origin = hit.destination + hit.normal * EPSILON;
		shadow_ray.direction = normalize(sun_pos - hit.destination);
		shadow_ray.max_distance = 4; 
		shadow_ray.minimum = vec3(0);
		shadow_ray.maximum = vec3(REGION_SIZE);


		RayHit shadow_hit;

		bool shadow_success = ray_cast(shadow_ray, shadow_hit);

		if(shadow_success) {
			color *= 0.5;
		}	

		vec4 v_clip_coord = camera.projection * inverse(compute_transform_matrix(region_transform)) * vec4(hit.destination, 1.0);
		float f_ndc_depth = v_clip_coord.z / v_clip_coord.w;
		gl_FragDepth = (f_ndc_depth + 1.0) * 0.5;
	} else {
		discard;
	}	

	result = color;
}

#endif
