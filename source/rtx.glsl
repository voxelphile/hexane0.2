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

struct RtxPush {
	BufferId info_id;
	BufferId camera_id;
	BufferId transform_id;
	BufferId region_id;
	BufferId mersenne_id;
	ImageId perlin_id;
	ImageId prepass_id;
};

decl_push_constant(RtxPush)

#ifdef compute

#define WARP_SIZE (32)
#define CACHE_SIZE (WARP_SIZE * 1)
#define BATCH_SIZE (CACHE_SIZE * 2)
#define STEPS_UNTIL_REORDER (32)
#define STALL_LIMIT 100

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

struct TraceState {
	bool currently_tracing;
	bool has_ray_result;
	u32 rays;
	vec4 color;
	u16 prev_id;
	f32 prev_dist;
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
				
	imageStore(prepass_image, i32vec2(trace_state.ray_state.ray.result_i), trace_state.color);
}

void reset_trace_state(in out TraceState trace_state) {
	trace_state.currently_tracing = false;
	trace_state.has_ray_result = false;
	trace_state.color = vec4(1);
	trace_state.prev_id = u16(0);
	trace_state.prev_dist = 0;
	trace_state.rays = 0;
	RayState state;
    	trace_state.ray_state = state;
}

bool ray_trace(in out TraceState trace_state) {
	
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Image(3D, u32) perlin_img = get_image(3D, u32, push_constant.perlin_id);
	Image(2D, f32) prepass_image = get_image(2D, f32, push_constant.prepass_id);

;
	
	if(!ray_cast_drive(trace_state.ray_state)) {
	RayHit ray_hit;

    	bool success = ray_cast_complete(trace_state.ray_state, ray_hit);

	if(trace_state.rays > MAX_TRACE) {
		return true;
	}

	bool traveling_through_water = trace_state.prev_id == u16(5) || trace_state.ray_state.ray.medium == u16(5);
		

	if (success) {
		if(is_transparent(u16(ray_hit.id))) {
			Ray ray;
			ray.direction = refract(normalize(trace_state.ray_state.ray.direction), ray_hit.normal, refraction_index(u16(ray_hit.id)));
			if(ray.direction == vec3(0)) {
				ray.direction = reflect(normalize(trace_state.ray_state.ray.direction), ray_hit.normal);
			}
				ray.region_id = push_constant.region_id;
				ray.origin = ray_hit.destination;
				ray.result_i = trace_state.ray_state.ray.result_i;
				ray.medium = u16(ray_hit.id);
				ray.max_distance = 100;
				ray.minimum = vec3(0);
				ray.maximum = vec3(REGION_SIZE);
				trace_state.prev_id = u16(trace_state.ray_state.ray.medium);
				trace_state.prev_dist = ray_hit.dist;
				ray_cast_start(ray, trace_state.ray_state);
				trace_state.ray_state.initial_dist = ray_hit.total_dist;
				trace_state.rays++;
				return false;
			
		}
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
		if(traveling_through_water) {
			trace_state.color.xyz *= vec3(0.42, 0.95, 1.0) * exp(-ray_hit.dist * 0.05 + 0.0);;
		}
	

		vec4 ambient = voxel_ao(
			region.data,
			ray_hit.back_step, 
			abs(ray_hit.normal.zxy), 
			abs(ray_hit.normal.yzx)
			);

		float ao = 0;

		if (true) {
			ao = mix(mix(ambient.z, ambient.w, ray_hit.uv.x), mix(ambient.y, ambient.x, ray_hit.uv.x), ray_hit.uv.y);
		}

		trace_state.color.xyz = trace_state.color.xyz - vec3(1 - ao) * 0.25;
	} else {

		if(traveling_through_water) {
			trace_state.color.xyz *= vec3(0.42, 0.95, 1.0) * exp(-10000 * 0.05 + 0.0);;
		} else {
			trace_state.color += vec4(1.0, 0.2, 1.0, 1.0);
		}
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

					Ray ray;
					ray.region_id = push_constant.region_id;
					ray.origin = region_transform.position.xyz;
					ray.direction = dir;
					ray.medium = query.id;
					ray.result_i = result_i;
					ray.max_distance = 100; 
					ray.minimum = vec3(0);
					ray.maximum = vec3(REGION_SIZE);
						ray_batch.setup_cache.rays[ray_cache_index] = ray;
						}
					}
					subgroupMemoryBarrierShared();
					if (!trace_state.currently_tracing) {
					i32 ray_cache_index = CACHE_SIZE - fast_atomic_decrement(ray_batch.setup_cache.size);

					ray_cast_start(ray_batch.setup_cache.rays[ray_cache_index], trace_state.ray_state);
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

