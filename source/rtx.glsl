#version 450

#include "hexane.glsl"
#include "region.glsl"
#include "vertex.glsl"
#include "noise.glsl"
#include "transform.glsl"
#include "voxel.glsl"
#include "aabb.glsl"
#include "raycast.glsl"
#include "camera.glsl"
#include "ao.glsl"

#define PI 3.14159265

struct RtxPush {
	BufferId info_id;
	BufferId camera_id;
	BufferId transform_id;
	BufferId region_id;
	BufferId mersenne_id;
	ImageId perlin_id;
};

decl_push_constant(RtxPush)


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
	
	positional_offset = clamp(positional_offset, 0, CHUNK_SIZE); 

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

struct Light {
	vec3 position;
	vec4 color;
};

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
	vec3 light_energy = vec3(0);
	float transmittance = 1;

	vec3 chunk_pos = transforms.data[1 + chunk].position.xyz - vec3(uvec3(vec3(AXIS_MAX_CHUNKS * CHUNK_SIZE / 2))) + vec3(uvec3(vec3(REGION_SIZE / 2))) - vec3(diff);

	vec3 sun_pos = vec3(10000);
	
	u32 fluff = 5;

	Ray ray;
	ray.region = region;
	ray.origin = origin;
	ray.direction = dir;
	ray.max_distance = sqrt(f32(3)) * CHUNK_SIZE + 1; 
	ray.minimum = chunk_pos + 0 - fluff;
	ray.maximum = chunk_pos + CHUNK_SIZE + fluff;

	RayState ray_state = ray_cast_start(ray);

	while(ray_cast_drive(ray, ray_state)) {}
	
	RayHit ray_hit;

	bool success = ray_cast_complete(ray, ray_state, ray_hit);
		
	if (success) {
		if(distance(ray_hit.destination, vec3(region_transform.position.xyz)) > VIEW_DISTANCE) {
			discard;
		}

		f32 noise_factor = f32(imageLoad(perlin_img, i32vec3(abs(round(vec3(region.floating_origin) - vec3(REGION_SIZE / 2) + ray_hit.destination + vec3(0.5)))) % i32vec3(imageSize(perlin_img))).r) / f32(~0u);
		if(ray_hit.id == 0) {
			color.xyz = vec3(1, 0, 1);
		}
		if(ray_hit.id == 2) {
			color.xyz = mix(vec3(170, 255, 21) / 256, vec3(34, 139, 34) / 256, noise_factor);
		}
		if(ray_hit.id == 3) {
			color.xyz = mix(vec3(135) / 256, vec3(80) / 256, noise_factor);
		}

		if(ray_hit.id == 4) {
			color.xyz = mix(vec3(107, 84, 40) / 256, vec3(64, 41, 5) / 256, noise_factor);
		}

		vec4 ambient = voxel_ao(
			region.data,
			ray_hit.back_step, 
			abs(ray_hit.normal.zxy), 
			abs(ray_hit.normal.yzx)
			);

		float ao = 0;

		if (ENABLE_AO) {
			ao = mix(mix(ambient.z, ambient.w, ray_hit.uv.x), mix(ambient.y, ambient.x, ray_hit.uv.x), ray_hit.uv.y);
		}

		color.xyz = color.xyz - vec3(1 - ao) * 0.25;

		float intensity = 0.3;

		u32 light_count = 1;
		Light lights[1];
		lights[0].position = sun_pos;
		lights[0].color = vec4(0.95, 0.99, 1.0, 1.0);

		for(i32 i = 0; i < light_count; i++) {
		if(i != 0) {
			u32 x = random(push_constant.mersenne_id) - (~0u) / 2;
			u32 y = random(push_constant.mersenne_id) - (~0u) / 2;
			u32 z = random(push_constant.mersenne_id) - (~0u) / 2;
		
					
			lights[i].position = normalize(vec3(x, y, z)) * 1000000; 
			lights[i].color = vec4(lights[0].color.xyz, (1 - 0.3) / light_count);
		}

		Ray shadow_ray;
		shadow_ray.region = region;
		shadow_ray.origin = ray_hit.destination + ray_hit.normal * EPSILON * EPSILON;
		shadow_ray.direction = normalize(lights[i].position - ray_hit.destination);
		shadow_ray.max_distance = 100; 
		shadow_ray.minimum = vec3(0);
		shadow_ray.maximum = vec3(REGION_SIZE);

		RayState shadow_ray_state = ray_cast_start(shadow_ray);

		while(ray_cast_drive(shadow_ray, shadow_ray_state)) {}
	
		RayHit shadow_ray_hit;

		bool shadow_success = ray_cast_complete(shadow_ray, shadow_ray_state, shadow_ray_hit);
			
		if(!shadow_success) {
			intensity += lights[i].color.a;

		}	

		if(intensity >= 1) {
			break;
		}

		color.xyz *= lights[i].color.xyz;
		}

		color *= min(intensity, 1);

		vec4 v_clip_coord = camera.projection * inverse(compute_transform_matrix(region_transform)) * vec4(ray_hit.destination, 1.0);
		float f_ndc_depth = v_clip_coord.z / v_clip_coord.w;
		gl_FragDepth = (f_ndc_depth + 1.0) * 0.5;
	} else {
		discard;
	}	

	result = color;
}

#endif
