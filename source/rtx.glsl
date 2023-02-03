#version 450
//Credit to Gabe Rundlett, original source from gvox engine

#define EULER 2.71828
#define MAX_TRACE 16

#include "hexane.glsl"
#include "region.glsl"
#include "voxel.glsl"
#include "ao.glsl"
#include "camera.glsl"
#include "raycast.glsl"
#include "transform.glsl"
#include "noise.glsl"
#include "luminosity.glsl"

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
};

decl_push_constant(RtxPush)

#ifdef compute

#define WARP_SIZE (32)
#define CACHE_SIZE (WARP_SIZE * 1)
#define BATCH_SIZE (CACHE_SIZE * 2)
#define STEPS_UNTIL_REORDER (32)
#define STALL_LIMIT 100

#define TRACE_STATE_CAMERA 0
#define TRACE_STATE_TRUE_SETUP 1
#define TRACE_STATE_TRUE 2
#define TRACE_STATE_LIGHT_SETUP 3
#define TRACE_STATE_LIGHT 4

layout (local_size_x = WARP_SIZE, local_size_y = 1, local_size_z = 1) in;

struct RayBatchSetupCache {
	i32 size;
	i32 start_index;
	bool need_regeneration;
	Ray rays[CACHE_SIZE];
};

struct RayBatch {
	i32 size;
	i32 start_index;
	RayBatchSetupCache setup_cache;
};

shared RayBatch ray_batch;

u32vec2 get_result_i(u32 result_index) {
	Image(2D, f32) prepass_image = get_image(2D, f32, push_constant.prepass_id);
    u32vec2 result;

#if 1
    result.y = result_index / u32(imageSize(prepass_image).x);
    result.x = result_index - result.y * u32(imageSize(prepass_image).x);
#else
    /*
    u32 block_index = result_index / 64;
    u32 block_sub_index = result_index - block_index * 64;
    u32vec2 block_i;
    block_i.y = block_index / (GLOBALS.padded_frame_dim.x / 8);
    block_i.x = block_index - block_i.y * (GLOBALS.padded_frame_dim.x / 8);
    result.y = block_sub_index / 8;
    result.x = block_sub_index - result.y * 8;
    result += block_i * 8;o
    */
#endif

    return result;
}

bool is_transparent(u16 id) {
	return id == u16(0) || id == u16(1) || id == u16(5);
}

f32 refraction_index(u16 id) {
	if(id == u16(5)) {
		return 1.001;
	}
	return 1.0;
}

vec3 medium_color(u16 id) {
	if(id == u16(BLOCK_ID_WATER)) {
		return vec3(.1, .2, .9);
	}
	if(id == u16(BLOCK_ID_AIR)) {
		return vec3(.2, .4, .6);
	}
	return vec3(1);
}

f32 medium_absorption(u16 id) {
	if(id == u16(BLOCK_ID_WATER)) {
		return 0.1;
	}
	return 0;
}

f32 medium_scatter(u16 id) {
	if(id == u16(BLOCK_ID_WATER)) {
		return 0.0003;
	}
	return 0;
}

struct TraceState {
	bool currently_tracing;
	bool has_ray_result;
	u32 rays;
	u32 id;
	vec4 color;
	vec3 true_dir;
	vec3 aperture_pos;
	bool reject;
	u16 prev_id;
	f32 prev_dist;
	RayHit final_hit;
	RayHit true_hit;
	Ray initial_ray;
	RayState ray_state;
};

i32 fast_atomic_decrement(inout i32 a) {
    u32vec4 exec = subgroupBallot(true);
    i32 active_thread_count_left_to_me = i32(subgroupBallotExclusiveBitCount(exec));
    i32 ret = a - active_thread_count_left_to_me;
    i32 active_thread_count = i32(subgroupBallotBitCount(exec));
    a = a - active_thread_count;
    return ret;
}

void write_ray_result(in out TraceState trace_state) {
	Image(2D, f32) prepass_image = get_image(2D, f32, push_constant.prepass_id);
	Image(2D, f32) dir_image = get_image(2D, f32, push_constant.dir_id);
	Image(2D, f32) pos_image = get_image(2D, f32, push_constant.pos_id);
				
	i32vec2 pos = i32vec2(trace_state.initial_ray.result_i);

	pos = imageSize(prepass_image) - pos;

	imageStore(prepass_image, pos, trace_state.color);
	imageStore(dir_image, pos, vec4(trace_state.true_dir, 0));
	imageStore(pos_image, pos, vec4(trace_state.true_hit.destination, 1));
}

void reset_trace_state(in out TraceState trace_state) {
	trace_state.currently_tracing = false;
	trace_state.has_ray_result = false;
	trace_state.color = vec4(1);
	trace_state.prev_id = u16(0);
	trace_state.prev_dist = 0;
	trace_state.rays = 0;
	trace_state.true_dir = vec3(0);
	trace_state.aperture_pos = vec3(0);
	trace_state.id = TRACE_STATE_CAMERA;

	RayState state;
    	trace_state.ray_state = state;
	{
	RayHit hit;
	trace_state.final_hit = hit;
	}
	{
	RayHit hit;
	trace_state.true_hit = hit;
	}
	Ray ray;
	trace_state.initial_ray = ray;
	trace_state.reject = false;
}

bool ray_trace(in out TraceState trace_state) {
	if(trace_state.rays > MAX_TRACE) {
		return true;
	}
	
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Image(3D, u32) perlin_img = get_image(3D, u32, push_constant.perlin_id);
	Image(2D, f32) prepass_image = get_image(2D, f32, push_constant.prepass_id);

;
	
	
	bool traveling_through_water = trace_state.prev_id == u16(5) || trace_state.ray_state.ray.medium == u16(5);
	
	if(trace_state.id == TRACE_STATE_CAMERA && !ray_cast_drive(trace_state.ray_state)) {	
	RayHit ray_hit;
    	
	bool success = ray_cast_complete(trace_state.ray_state, ray_hit);
	
	f32 extinction_coef = medium_absorption(trace_state.ray_state.ray.medium)
		+ medium_scatter(trace_state.ray_state.ray.medium);
	
	f32 dist = VIEW_DISTANCE;
	if (success) {
		dist = ray_hit.dist;
	}
	

	trace_state.color.rgb = mix(trace_state.color.rgb, medium_color(trace_state.ray_state.ray.medium), 1 - exp(-dist * extinction_coef));

	if(!success) {
		trace_state.id = TRACE_STATE_TRUE_SETUP;
		trace_state.reject = true;
		return false;
		
	}

	if (success) {
		trace_state.final_hit = ray_hit;
	
		
		f32 noise_factor = f32(imageLoad(perlin_img, i32vec3(abs(round(vec3(region.floating_origin) - vec3(REGION_SIZE / 2) + ray_hit.destination + vec3(0.5)))) % i32vec3(imageSize(perlin_img))).r) / f32(~0u);

		if(ray_hit.id == 0) {
			trace_state.color.xyz = vec3(1, 0, 1);
		}
		if(ray_hit.id == 2) {
			trace_state.color.xyz *= mix(vec3(170, 255, 21) / 256, vec3(34, 139, 34) / 256, noise_factor);
		}
		if(ray_hit.id == 3) {
			trace_state.color.xyz *= mix(vec3(135) / 256, vec3(80) / 256, noise_factor);
		}

		if(ray_hit.id == 4) {
			trace_state.color.xyz *= mix(vec3(107, 84, 40) / 256, vec3(64, 41, 5) / 256, noise_factor);
		}

		vec4 ambient = voxel_ao(
			region.data,
			ray_hit.back_step, 
			abs(ray_hit.normal.zxy), 
			abs(ray_hit.normal.yzx)
			);

		trace_state.color.xyz *= 0.75 + 0.25 * mix(mix(ambient.z, ambient.w, ray_hit.uv.x), mix(ambient.y, ambient.x, ray_hit.uv.x), ray_hit.uv.y);
		
		trace_state.color.a = ray_hit.dist;


		if(is_transparent(u16(ray_hit.id))) {
			Ray ray;
			ray.direction = refract(normalize(trace_state.ray_state.ray.direction), ray_hit.normal, refraction_index(u16(ray_hit.id)));
			f32 prod = dot(trace_state.ray_state.ray.direction, ray_hit.normal);
			bool should_reflect = 
				ray.direction == vec3(0)
				|| f32(random(push_constant.mersenne_id)) / f32(~0u) > 1 - exp(min(1 * prod, 0)) + f32(ray_hit.id != u16(BLOCK_ID_WATER));
			if(should_reflect) {
				ray.direction = reflect(normalize(trace_state.ray_state.ray.direction), ray_hit.normal);
				trace_state.color.xyz *= medium_color(u16(ray_hit.id));
			}
				ray.region_id = push_constant.region_id;
				ray.origin = ray_hit.destination;
				ray.result_i = trace_state.ray_state.ray.result_i;
				ray.medium = u16(ray_hit.id);
				ray.max_distance = VIEW_DISTANCE;
				ray.true_dir = trace_state.ray_state.ray.true_dir;
				ray.aperture_pos = trace_state.ray_state.ray.aperture_pos;
				ray.minimum = vec3(0);
				ray.maximum = vec3(REGION_SIZE);
				trace_state.prev_id = u16(trace_state.ray_state.ray.medium);
				trace_state.prev_dist = ray_hit.dist;
				ray_cast_start(ray, trace_state.ray_state);
				trace_state.ray_state.initial_dist = ray_hit.total_dist;
				trace_state.rays++;
				return false;
			
		}

	}
		trace_state.id = TRACE_STATE_TRUE_SETUP;
		return false;
	}
	
	if(trace_state.id == TRACE_STATE_TRUE_SETUP) {
		Ray ray = trace_state.initial_ray;
		ray.direction = -ray.true_dir; 
		ray.region_id = push_constant.region_id;
		ray.max_distance = 10000;
		ray.minimum = vec3(0);
		ray.maximum = vec3(REGION_SIZE);
		ray_cast_start(ray, trace_state.ray_state);
		trace_state.id = TRACE_STATE_TRUE;
		return false;
	}
	
	if(trace_state.id == TRACE_STATE_TRUE && !ray_cast_drive(trace_state.ray_state)) {	
	RayHit ray_hit;
    	
	bool success = ray_cast_complete(trace_state.ray_state, ray_hit);
	
	if (success) {
		trace_state.true_hit = ray_hit;

		if(is_transparent(u16(ray_hit.id))) {
			Ray ray;
			ray.direction = refract(normalize(trace_state.ray_state.ray.direction), ray_hit.normal, refraction_index(u16(ray_hit.id)));
			f32 prod = dot(trace_state.ray_state.ray.direction, ray_hit.normal);
			bool should_reflect = 
				ray.direction == vec3(0);
			if(should_reflect) {
				ray.direction = reflect(normalize(trace_state.ray_state.ray.direction), ray_hit.normal);
			}
				ray.region_id = push_constant.region_id;
				ray.origin = ray_hit.destination;
				ray.medium = u16(ray_hit.id);
				ray.max_distance = VIEW_DISTANCE;
				ray.true_dir = trace_state.ray_state.ray.true_dir;
				ray.minimum = vec3(0);
				ray.maximum = vec3(REGION_SIZE);
				trace_state.prev_id = u16(trace_state.ray_state.ray.medium);
				trace_state.prev_dist = ray_hit.dist;
				ray_cast_start(ray, trace_state.ray_state);
				trace_state.ray_state.initial_dist = ray_hit.total_dist;
				return false;
			
		}

	}

	if(trace_state.reject) {
		return true;
	}

	trace_state.id = TRACE_STATE_LIGHT_SETUP;
	return false;
	}

	vec3 sun_pos = vec3(10000);
	vec3 sun_color = vec3(0.8, 0.9, 1.0);

	if(trace_state.id == TRACE_STATE_LIGHT_SETUP) {
		Ray ray;
		ray.direction = normalize(sun_pos - trace_state.final_hit.destination); 
		ray.region_id = push_constant.region_id;
		ray.origin = trace_state.final_hit.destination + trace_state.final_hit.normal * EPSILON;
		ray.result_i = trace_state.initial_ray.result_i;
		ray.medium = u16(trace_state.ray_state.ray.medium);
		ray.max_distance = 10000;
		ray.minimum = vec3(0);
		ray.maximum = vec3(REGION_SIZE);
		trace_state.prev_id = u16(trace_state.ray_state.ray.medium);
		trace_state.prev_dist = trace_state.final_hit.dist;
		ray_cast_start(ray, trace_state.ray_state);
		trace_state.rays++;
		trace_state.id = TRACE_STATE_LIGHT;
		return false;
	}

	if(trace_state.id == TRACE_STATE_LIGHT && !ray_cast_drive(trace_state.ray_state)) {
		RayHit ray_hit;

    		bool success = ray_cast_complete(trace_state.ray_state, ray_hit);

		if(trace_state.ray_state.id == RAY_STATE_VOXEL_FOUND && !is_solid(u16(ray_hit.id))) {
			Ray ray;
			ray.direction = trace_state.ray_state.ray.direction; 
			ray.region_id = push_constant.region_id;
			ray.origin = trace_state.ray_state.map_pos;
			ray.result_i = trace_state.ray_state.ray.result_i;
			ray.medium = u16(ray_hit.id);
			ray.max_distance = 10000;
			ray.minimum = vec3(0);
			ray.maximum = vec3(REGION_SIZE);
			trace_state.prev_id = u16(trace_state.ray_state.ray.medium);
			trace_state.prev_dist = ray_hit.dist;
			ray_cast_start(ray, trace_state.ray_state);
			trace_state.rays++;
			trace_state.id = TRACE_STATE_LIGHT;
			return false;
		}

		bool in_light = trace_state.ray_state.id != RAY_STATE_VOXEL_FOUND;
		
		if(in_light) {	
			trace_state.color.xyz *= 20 * (0.5 + 0.5 * dot(trace_state.ray_state.ray.direction, trace_state.final_hit.normal));
		} else {
			trace_state.color.xyz *= 0.3;
		}

		return true;
	}

	return false;
}

void main() {
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Image(3D, u32) perlin_img = get_image(3D, u32, push_constant.perlin_id);

	if(subgroupElect()) {
		ray_batch.start_index = 0;
        	ray_batch.size = 0;
      	  	ray_batch.setup_cache.start_index = 0;
        	ray_batch.setup_cache.size = 0;
        	ray_batch.setup_cache.need_regeneration = true;
	}

	TraceState trace_state;
	reset_trace_state(trace_state);
	
	Image(2D, f32) prepass_image = get_image(2D, f32, push_constant.prepass_id);

    	const i32 TOTAL_RAY_COUNT = i32(imageSize(prepass_image).x * imageSize(prepass_image).y);

	Transform region_transform = transforms.data[0];
	ivec3 diff = region.floating_origin - region.observer_position;
	region_transform.position.xyz = vec3(REGION_SIZE / 2) - vec3(diff);
	region_transform.position.xyz += transforms.data[0].position.xyz - region.observer_position;

	VoxelQuery query;
	query.region_data = region.data;
	query.position = ivec3(region_transform.position.xyz);

	voxel_query(query);

	int count = 0;

	while(count < STALL_LIMIT) {
		count++;
		if(subgroupAny(!trace_state.currently_tracing)) {
			if(!trace_state.currently_tracing) {
				if(trace_state.has_ray_result) {
					write_ray_result(trace_state);
					reset_trace_state(trace_state);
				}

				i32 ray_cache_index = CACHE_SIZE - fast_atomic_decrement(ray_batch.setup_cache.size);

				if(ray_cache_index < CACHE_SIZE) {
					ray_cast_start(ray_batch.setup_cache.rays[ray_cache_index], trace_state.ray_state);
					trace_state.true_dir = ray_batch.setup_cache.rays[ray_cache_index].true_dir;
					trace_state.initial_ray = ray_batch.setup_cache.rays[ray_cache_index];
					trace_state.aperture_pos = ray_batch.setup_cache.rays[ray_cache_index].aperture_pos;
					trace_state.currently_tracing = true;
					trace_state.has_ray_result = false;
					trace_state.rays = 1;
				}

				if (ray_cache_index >= CACHE_SIZE) {
                    			if (subgroupElect()) {
                        			ray_batch.setup_cache.need_regeneration = true;
                 			}
               	 		}
			}
			subgroupMemoryBarrierShared();
			if(ray_batch.setup_cache.need_regeneration) {
				  if (subgroupElect()) {
			                    ray_batch.setup_cache.need_regeneration = false;
        			            if (ray_batch.size == 0) {
                      			    i32 global_remaining_rays = atomicAdd(region.ray_count, -BATCH_SIZE);
                    			    ray_batch.size = clamp(global_remaining_rays, 0, BATCH_SIZE);
                        ray_batch.start_index = TOTAL_RAY_COUNT - global_remaining_rays;
                    }
                    ray_batch.setup_cache.size = clamp(ray_batch.size, 0, CACHE_SIZE);
                    ray_batch.size -= ray_batch.setup_cache.size;
                    ray_batch.setup_cache.start_index = ray_batch.start_index + BATCH_SIZE - ray_batch.size;
                }

				subgroupMemoryBarrierShared();
				if (ray_batch.setup_cache.size > 0) {
					for (u32 i = 0; i < ray_batch.setup_cache.size; i += WARP_SIZE) {
						u32 ray_cache_index = i + gl_SubgroupInvocationID.x;
						if (ray_cache_index < ray_batch.setup_cache.size) {
					
					u32 result_index = ray_batch.setup_cache.start_index + ray_cache_index;
					u32vec2 result_i = get_result_i(result_index);

					vec2 screenPos = (vec2(result_i) / vec2(camera.resolution.xy)) * 2.0 - 1.0;
					vec4 far = camera.inv_projection * vec4(screenPos, 1, 1);
					far /= far.w;

					vec3 dir = (compute_transform_matrix(region_transform) * vec4(normalize(far.xyz), 0)).xyz;

					Buffer(Luminosity) luminosity = get_buffer(Luminosity, push_constant.luminosity_id);
					f32 c_pi = 3.1415;
					f32 DOFApertureRadius = 0.1;
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

					Ray ray;
					ray.region_id = push_constant.region_id;
					ray.origin = aperturePos.xyz;
					ray.direction = normalize(focusPos - aperturePos);
					ray.true_dir = dir;
					ray.aperture_pos = aperturePos;
					ray.medium = query.id;
					ray.result_i = result_i;
					ray.max_distance = VIEW_DISTANCE; 
					ray.minimum = vec3(0);
					ray.maximum = vec3(REGION_SIZE);
						ray_batch.setup_cache.rays[ray_cache_index] = ray;
						}
					}
					subgroupMemoryBarrierShared();
					if (!trace_state.currently_tracing) {
					i32 ray_cache_index = CACHE_SIZE - fast_atomic_decrement(ray_batch.setup_cache.size);

					ray_cast_start(ray_batch.setup_cache.rays[ray_cache_index], trace_state.ray_state);
					trace_state.true_dir = ray_batch.setup_cache.rays[ray_cache_index].true_dir;
					trace_state.initial_ray = ray_batch.setup_cache.rays[ray_cache_index];
					trace_state.aperture_pos = ray_batch.setup_cache.rays[ray_cache_index].aperture_pos;
					trace_state.currently_tracing = true;
					trace_state.has_ray_result = false;
					trace_state.rays = 1;
					}
				}
			}
		}

		if (subgroupAll(!trace_state.currently_tracing))
            		break;

        	[[unroll]] for (u32 i = 0; (i < STEPS_UNTIL_REORDER); ++i) {
            		if(!trace_state.currently_tracing)
                		break;
            		if(ray_trace(trace_state)) {
				trace_state.currently_tracing = false;
				trace_state.has_ray_result = true;
			}
        	}
	}
}

#endif

