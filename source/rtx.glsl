#include "ao.glsl"

#define MAX_TRACE 64
#define TRACE_STATE_INITIAL 0
#define TRACE_STATE_MAX_DIST_REACHED 1
#define TRACE_STATE_OUT_OF_BOUNDS 2
#define TRACE_STATE_VOXEL_FOUND 3
#define TRACE_STATE_MAX_TRACE_REACHED 4
#define TRACE_STATE_FAILED 5
#define MAX_STEP_COUNT 1024
#define RAY_STATE_INITIAL 0
#define RAY_STATE_OUT_OF_BOUNDS 1
#define RAY_STATE_MAX_DIST_REACHED 2
#define RAY_STATE_MAX_STEP_REACHED 3
#define RAY_STATE_VOXEL_FOUND 4

struct Ray {
	ImageId region_data;
	i32 max_lod;
	vec3 origin;
	vec3 direction;
	vec3 true_dir;
	vec3 minimum;
	f32 t;
	vec3 aperture_pos;
	u16 medium;
	vec3 maximum;
	f32 max_distance;
	u32vec2 result_i;
};

struct RayState {
	u32 id;
	ivec3 map_pos;
	f32 dist;
	f32 initial_dist;
	bvec3 mask;
	vec3 side_dist;
	vec3 delta_dist;
	ivec3 ray_step;
	u32 lod;
	u16 block_id;
	u32 count;
	Ray ray;
};

struct RayHit {
	f32 dist;
	f32 total_dist;
	ivec3 normal;
	ivec3 back_step;
	vec3 destination;
	bvec3 mask;
	vec2 uv;
	u32 id;
	Ray ray;
};

void ray_cast_body(inout RayState state) {
	state.mask = lessThanEqual(state.side_dist.xyz, min(state.side_dist.yzx, state.side_dist.zxy));
        state.side_dist += vec3(state.mask) * state.delta_dist;
        state.map_pos += ivec3(vec3(state.mask)) * state.ray_step;
	state.dist = length(vec3(state.mask) * (state.side_dist - state.delta_dist)) / length(state.ray.direction);
}


void ray_cast_start(inout Ray ray, out RayState state) {
	ray.direction = normalize(ray.direction);

	state.id = RAY_STATE_INITIAL;
	state.map_pos = ivec3(floor(ray.origin + 0.));
	state.mask = bvec3(false);
	state.dist = 0;
	state.initial_dist = 0;
	state.block_id = u16(0);
	state.ray = ray;
	state.delta_dist = 1.0 / abs(state.ray.direction);
	state.ray_step = ivec3(sign(state.ray.direction));
	state.side_dist = (sign(state.ray.direction) * (vec3(state.map_pos) - state.ray.origin) + (sign(state.ray.direction) * 0.5) + 0.5) * state.delta_dist;
	state.count = 0;
}

bool ray_cast_complete(inout RayState state, out RayHit hit) {
	ivec3 ray_step = ivec3(sign(state.ray.direction));
	
		vec3 destination = state.ray.origin + state.ray.direction * state.dist;
		ivec3 back_step = ivec3(state.map_pos - ray_step * vec3(state.mask));
		vec2 uv = mod(vec2(dot(vec3(state.mask) * destination.yzx, vec3(1.0)), dot(vec3(state.mask) * destination.zxy, vec3(1.0))), vec2(1.0));
		ivec3 normal = ivec3(vec3(state.mask) * sign(-state.ray.direction));

		hit.destination = destination;
		hit.mask = state.mask;
		hit.back_step = back_step;
		hit.uv = uv;
		hit.normal = normal;
		hit.id = state.block_id;
		hit.dist = state.dist;
		hit.total_dist = state.initial_dist + state.dist;
		hit.ray = state.ray;

	return state.id == RAY_STATE_VOXEL_FOUND;
}

bool ray_cast_check_over_count(inout RayState state) {
	if(state.count++ > MAX_STEP_COUNT) {
		state.id = RAY_STATE_MAX_STEP_REACHED;
		return true;
	}
	return false;
}

bool ray_cast_check_out_of_bounds(inout RayState state) {
	bool in_chunk = all(greaterThanEqual(state.map_pos, vec3(state.ray.minimum) )) && all(lessThan(state.map_pos, vec3(state.ray.maximum)));
	if(!in_chunk) {
		state.id = RAY_STATE_OUT_OF_BOUNDS;
		return true;
	}
	return false;
}

bool ray_cast_check_over_dist(inout RayState state) {
	if(state.initial_dist + state.dist > state.ray.max_distance) {
		state.id = RAY_STATE_MAX_DIST_REACHED;
		return true;	
	}
	return false;
}

bool ray_cast_check_failure(inout RayState state) {
	return ray_cast_check_over_dist(state) || ray_cast_check_out_of_bounds(state) || ray_cast_check_over_count(state);
}

bool ray_cast_check_success(inout RayState state) {
	VoxelQuery query;
	query.region_data = state.ray.region_data;
	query.position = state.map_pos;

	bool voxel_found = voxel_query(query);

	if (voxel_found && query.id != state.ray.medium) {
		state.id = RAY_STATE_VOXEL_FOUND;
		state.block_id = query.id;
		return true;
	}

	return false;
}

bool ray_cast_drive(inout RayState state) {
	if(state.id != RAY_STATE_INITIAL) {
		return false;
	}
	
	if(ray_cast_check_over_count(state)) {
		return false;
	}

	if(ray_cast_check_failure(state)) {
		return false;
	}

	if(ray_cast_check_success(state)) {
		return false;
	}
	
	ray_cast_body(state);

	return true;
}



struct Trace {
	vec3 origin;
	vec3 direction;
	f32 max_distance;
	ImageId region_data;
	ImageId block_data;
};

struct TraceState {
	i32 id;
	i32 count;
	RayState ray_state;
	RayHit block_hit;
	RayHit approach_hit;
	RayHit voxel_hit;
	ImageId region_data;
	ImageId block_data;
};

struct TraceHit {
	RayHit block_hit;
	RayHit voxel_hit;
	RayHit approach_hit;
}; 

f32 wrap(f32 o, f32 n) {
	const float m = BLOCK_DETAIL;
	n -= o;
	return n >= 0 ? mod(n, m) : mod(mod(n, m + m), m) + o;
}

vec3 wrap(vec3 o, vec3 n) {
	return vec3(wrap(o.x, n.x), wrap(o.y, n.y), wrap(o.z, n.z));
}

void ray_trace_start(Trace trace, out TraceState state) {
	Ray ray;
	ray.region_data = trace.region_data;
	ray.max_distance = trace.max_distance; 
	ray.minimum = vec3(0);
	ray.maximum = vec3(REGION_SIZE);
	ray.direction = trace.direction;
	ray.medium = u16(BLOCK_ID_AIR);
	ray.origin = trace.origin;
	
	state.id = TRACE_STATE_INITIAL;
	state.count = 0;
	state.block_hit.id = u16(BLOCK_ID_AIR);
	state.block_hit.destination = ray.origin;
	state.block_hit.normal = ivec3(0);
	state.region_data = trace.region_data;
	state.block_data = trace.block_data;

	state.ray_state.ray = ray;
	state.ray_state.dist = 0;
}

bool ray_trace_complete(in TraceState state, out TraceHit hit) {
	hit.block_hit = state.block_hit;
	hit.approach_hit = state.approach_hit;
	hit.voxel_hit = state.voxel_hit;

	return state.id == TRACE_STATE_VOXEL_FOUND;
}

bool ray_trace_drive(inout TraceState state) {
	if(state.id != TRACE_STATE_INITIAL) {
		return false;
	}

	if(state.count++ > MAX_TRACE) {
		state.id = TRACE_STATE_MAX_TRACE_REACHED;
		return false;
	}

	state.approach_hit = state.block_hit;

	state.ray_state.ray.origin = state.block_hit.destination - vec3(state.block_hit.normal) * EPSILON;
	state.ray_state.ray.medium = u16(state.block_hit.id);

	f32 dist = state.ray_state.dist;
	ray_cast_start(state.ray_state.ray, state.ray_state);
	state.ray_state.initial_dist = dist;

	while(ray_cast_drive(state.ray_state)) {}
	
	bool success = ray_cast_complete(state.ray_state, state.block_hit);

	switch(state.ray_state.id) {
		case RAY_STATE_MAX_DIST_REACHED:
			state.id = TRACE_STATE_MAX_DIST_REACHED;
			break;
		case RAY_STATE_OUT_OF_BOUNDS:
			state.id = TRACE_STATE_OUT_OF_BOUNDS;
			break;
	}
	
	if(!success) {
		return false;
	}

	bool hit = false;
	f32 smudge = 1e-1;

	if(is_solid(u16(state.block_hit.id))) {
		RayState sub_state = state.ray_state;
		sub_state.ray.medium = u16(state.block_hit.id);
		RayHit sub_hit = state.block_hit;

		vec3 origin = sub_hit.destination - vec3(sub_hit.normal) * EPSILON;
		f32 block_start = sub_hit.id * BLOCK_DETAIL;
		Ray inner;
		RayState inner_state;
		inner.region_data = state.block_data;
		inner.max_distance = 100; 
		inner.minimum = vec3(0, 0, block_start);
		inner.maximum = inner.minimum + BLOCK_DETAIL;
		inner.direction = state.ray_state.ray.direction;
		inner.origin = inner.minimum + fract(origin) * BLOCK_DETAIL;
		inner.medium = u16(0);
	
		ray_cast_start(inner, inner_state);

		while(true)  {
			while(ray_cast_drive(inner_state)) {}

			hit = ray_cast_complete(inner_state, state.voxel_hit);
			
			if(inner_state.id == RAY_STATE_OUT_OF_BOUNDS) {
				ray_cast_body(sub_state);
				ray_cast_check_success(sub_state);
				ray_cast_complete(sub_state, sub_hit);

				if(sub_hit.id != state.block_hit.id) {
					break;
				}

				if(ray_cast_check_failure(sub_state)) {
					break;
				}

				origin = sub_hit.destination - vec3(sub_hit.normal) * EPSILON;
				inner.origin = inner.minimum + fract(origin) * BLOCK_DETAIL;

				f32 d = state.voxel_hit.total_dist;
				ray_cast_start(inner, inner_state);
				inner_state.initial_dist = d;

				continue;
			}

			state.ray_state = sub_state;
			state.block_hit = sub_hit;

			break;
		};

	} 
	
	if(hit) {
		state.id = TRACE_STATE_VOXEL_FOUND;
		return false;
	}

	return true;
}

struct Path {
	vec3 origin;
	vec3 direction;
	ImageId region_data;
	ImageId block_data;
	BufferId mersenne_id;
};

struct PathInfo {
	vec4 color;
	f32 dist;
	ivec3 normal;
	u16 voxel_id;
};

PathInfo path_trace(Path path) {
	PathInfo info;
	info.normal = ivec3(0);

	TraceHit hit;
	
	TraceState state;

	Trace trace;
	trace.origin = path.origin;
	trace.direction = path.direction;
	trace.region_data = path.region_data;
	trace.block_data = path.block_data;
	trace.max_distance = VIEW_DISTANCE;

	ray_trace_start(trace, state);

	while(ray_trace_drive(state)) {}

	bool success = ray_trace_complete(state, hit);

	vec4 color = vec4(1);
	vec3 sun_pos = vec3(100000);

	if(success) {
		u32 id = u32(hit.voxel_hit.id);
		f32 noise_factor = 0.5;
		if(id == 2) {
			color.xyz *= mix(vec3(170, 255, 21) / 256, vec3(34, 139, 34) / 256, noise_factor);
		}
		if(id == 3) {
			color.xyz *= mix(vec3(135) / 256, vec3(80) / 256, noise_factor);
		}

		if(id == 4) {
			color.xyz *= mix(vec3(107, 84, 40) / 256, vec3(64, 41, 5) / 256, noise_factor);
		}
		
		ivec3 normal = hit.voxel_hit.normal;
		if(hit.voxel_hit.mask == bvec3(false)) {
			normal = hit.block_hit.normal;
		}
		info.normal = normal;

		TraceHit light_hit;
		TraceState light_state;
		Trace light_trace;
		light_trace.max_distance = 20;
		light_trace.origin = hit.block_hit.destination.xyz
			+ path.direction * ((hit.voxel_hit.dist / BLOCK_DETAIL) - 1e-4);
		light_trace.direction = normalize(sun_pos - light_trace.origin);
		light_trace.region_data = path.region_data;
		light_trace.block_data = path.block_data;


		ray_trace_start(light_trace, light_state);

		while(ray_trace_drive(light_state)) {}

		bool in_shadow = ray_trace_complete(light_state, light_hit);
	
		if(in_shadow) {
			color.xyz *= 0.3;
		}

		info.dist = hit.block_hit.total_dist + hit.voxel_hit.dist / BLOCK_DETAIL;
		info.voxel_id = u16(hit.voxel_hit.id);
	} else {
		if(dot(normalize(sun_pos - path.origin), path.direction) > 0.99) {
			color.xyz = vec3(1, 1, 0);
		} else {
			color.xyz = vec3(0.1, 0.2, 1.0);
		}
		info.dist = VIEW_DISTANCE;
	}


	info.color = color;
	return info;
}
