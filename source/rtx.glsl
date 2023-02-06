#version 450

#include "hexane.glsl"
#include "region.glsl"
#include "voxel.glsl"
#include "ao.glsl"
#include "camera.glsl"
#include "raycast.glsl"
#include "transform.glsl"
#include "noise.glsl"
#include "luminosity.glsl"

#define VERTICES_PER_CUBE 6

struct RtxPush {
	BufferId info_id;
	BufferId camera_id;
	BufferId transform_id;
	BufferId region_id;
	BufferId mersenne_id;
	ImageId perlin_id;
	ImageId prepass_id;
	ImageId dir_id;
	ImageId pos_id;
	BufferId luminosity_id;
	BufferId entity_id;
};

decl_push_constant(RtxPush)

#ifdef fragment

layout(location = 0) out vec4 result;

f32 wrap(f32 n) {
	const float m = BLOCK_DETAIL;
	return n >= 0 ? mod(n, m) : mod(mod(n, m + m), m);
}

vec3 wrap(vec3 n) {
	return vec3(wrap(n.x), wrap(n.y), wrap(n.z));
}
void main() {
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Image(3D, u32) perlin_img = get_image(3D, u32, push_constant.perlin_id);

	Transform region_transform = transforms.data[0];
	ivec3 diff = region.floating_origin - region.observer_position;
	region_transform.position.xyz = vec3(REGION_SIZE / 2) - vec3(diff);
	region_transform.position.xyz += transforms.data[0].position.xyz - region.observer_position;
	
	vec2 screenPos = (1 - (vec2(gl_FragCoord) / vec2(camera.resolution.xy))) * 2.0 - 1.0;
	vec4 far = camera.inv_projection * vec4(screenPos, 1, 1);
	far /= far.w;

	vec3 dir = (compute_transform_matrix(region_transform) * vec4(normalize(far.xyz), 0)).xyz;

	Buffer(Luminosity) luminosity = get_buffer(Luminosity, push_constant.luminosity_id);
	f32 c_pi = 3.1415;
	f32 DOFApertureRadius = 0.04;
	f32 DOFFocalLength = luminosity.focal_depth;

	vec3 fwdVector = dir;
        vec3 rightVector = (vec4(1, 0, 0, 0) * inverse(compute_transform_matrix(region_transform))).xyz;
        vec3 upVector = (vec4(0, 1, 0, 0) * inverse(compute_transform_matrix(region_transform))).xyz;
        vec3 cameraForward = (vec4(0, 0, 1, 0) * inverse(compute_transform_matrix(region_transform))).xyz;

	float f01 = f32(random(push_constant.mersenne_id)) / f32(~0u);
	float f02 = f32(random(push_constant.mersenne_id)) / f32(~0u);
	float angle = f01 * 2.0f * c_pi;
        float radius = sqrt(f02);
        vec2 off = vec2(cos(angle), sin(angle)) * radius * DOFApertureRadius;
        float shapeArea = c_pi * DOFApertureRadius * DOFApertureRadius;
	f32 lightMultiplier = shapeArea; 
	vec3 sensorPos;
	vec4 sensorPlane = vec4(cameraForward, 0);
	vec3 cameraSensorPlanePoint = region_transform.position.xyz - fwdVector;
	sensorPlane.w = -(sensorPlane.x * cameraSensorPlanePoint.x + sensorPlane.y * cameraSensorPlanePoint.y + sensorPlane.z * cameraSensorPlanePoint.z);
        {
        	float t = -(dot( region_transform.position.xyz, sensorPlane.xyz) + sensorPlane.w) / dot(dir, sensorPlane.xyz);
		sensorPos = region_transform.position.xyz + dir * t;

        	vec3 cameraSpacePos = (vec4(sensorPos, 1.0f) * inverse(compute_transform_matrix(region_transform))).xyz;

		cameraSpacePos.z *= DOFFocalLength;

		sensorPos = (vec4(cameraSpacePos, 1.0f) * compute_transform_matrix(region_transform)).xyz;
	}

	vec3 aperturePos = region_transform.position.xyz + rightVector * off.x + upVector.xyz * off.y;

	vec3 cameraPos = region_transform.position.xyz;

	vec3 cameraFocalPlanePoint = cameraPos + vec3(cameraForward) * DOFFocalLength;

        vec4 focalPlane = vec4(-cameraForward, 0);

        focalPlane.w = -(focalPlane.x * cameraFocalPlanePoint.x + focalPlane.y * cameraFocalPlanePoint.y + focalPlane.z * cameraFocalPlanePoint.z);


	vec3 rstart = cameraPos;
        vec3 rdir = -dir;
	float t = -(dot(rstart, focalPlane.xyz) + focalPlane.w) / dot(rdir, focalPlane.xyz);
        vec3 focusPos = rstart + rdir * t;

	RayHit initial_hit;
	
	RayState initial_state;
	
	Ray ray;
	ray.region_data = region.data;
	ray.max_distance = VIEW_DISTANCE; 
	ray.minimum = vec3(0);
	ray.maximum = vec3(REGION_SIZE);
	ray.direction = normalize(focusPos - aperturePos);

	initial_hit.id = u16(1);
	initial_hit.destination = aperturePos.xyz;

	bool tracing = true;
	bool hit = false;
	vec4 color = vec4(1);
	RayHit pen_hit;

	do {
	ray.origin = initial_hit.destination;
	ray.medium = u16(initial_hit.id);

	ray_cast_start(ray, initial_state);

	while(ray_cast_drive(initial_state)) {}

	bool success = ray_cast_complete(initial_state, initial_hit);
	
	f32 dist = VIEW_DISTANCE;
	if(success) {
		dist = initial_hit.dist;
	}

	if(success && is_solid(u16(ray.medium))) {
		Ray inner;
		RayHit inner_hit;
		RayState inner_state;
		inner.region_data = region.blocks;
		inner.max_distance = 50; 
		inner.minimum = vec3(0);
		inner.maximum = vec3(BLOCK_DETAIL);
		inner.direction = normalize(focusPos - aperturePos);
		inner.origin = fract(initial_hit.ray.origin) * BLOCK_DETAIL;
		inner.medium = u16(1);
	
		ray_cast_start(inner, inner_state);

		while(true) {
			while(ray_cast_drive(inner_state)) {}

			hit = ray_cast_complete(inner_state, inner_hit);
			
			if(inner_state.id == RAY_STATE_OUT_OF_BOUNDS) {
				inner_state.id = RAY_STATE_INITIAL;
				inner.region_data = region.blocks;
				inner.max_distance = 50; 
				inner.minimum = vec3(0);
				inner.maximum = vec3(BLOCK_DETAIL);
				inner.direction = normalize(focusPos - aperturePos);
				inner.origin = wrap(inner_state.map_pos);
				inner.medium = u16(1);
	
				f32 d = inner_state.dist + inner_state.initial_dist;
				ray_cast_start(inner, inner_state);
				inner_state.initial_dist = d;

				continue;
			}

			break;
		};

		break;
	}

	if(!success) {
		tracing = false;
	}
	} while(tracing);
		
	if(hit) {
		color = vec4(1, 0, 1, 1);
	}

	result = color;
}

#endif
