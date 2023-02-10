#include "ao.glsl"

#define MAX_TRACE 64
#define TRACE_STATE_INITIAL 0
#define TRACE_STATE_MAX_DIST_REACHED 1
#define TRACE_STATE_OUT_OF_BOUNDS 2
#define TRACE_STATE_VOXEL_FOUND 3
#define TRACE_STATE_MAX_TRACE_REACHED 4
#define TRACE_STATE_FAILED 5

struct Trace {
	vec3 origin;
	vec3 direction;
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
	ray.max_distance = VIEW_DISTANCE; 
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

	if(state.count > MAX_TRACE) {
		state.id = TRACE_STATE_MAX_TRACE_REACHED;
		return false;
	}

	state.count++;

	state.ray_state.ray.origin = state.block_hit.destination - vec3(state.block_hit.normal) * EPSILON;
	state.ray_state.ray.medium = u16(state.block_hit.id);

	ray_cast_start(state.ray_state.ray, state.ray_state);

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

				ray_cast_check(sub_state);
				
				bool success = ray_cast_complete(sub_state, sub_hit);

				if(success && sub_hit.id != state.block_hit.id) {
					break;
				}

				origin = sub_hit.destination - vec3(sub_hit.normal) * EPSILON;
				inner.origin = inner.minimum + fract(origin) * BLOCK_DETAIL;

				f32 d = state.voxel_hit.total_dist;
				ray_cast_start(inner, inner_state);
				inner_state.initial_dist = d;

				continue;
			}

			if(!hit) {
				state.ray_state = sub_state;
				state.block_hit = sub_hit;
			}

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
};

vec3 path_trace(Path path) {
	TraceHit hit;
	
	TraceState state;

	Trace trace;
	trace.origin = path.origin;
	trace.direction = path.direction;
	trace.region_data = path.region_data;
	trace.block_data = path.block_data;

	ray_trace_start(trace, state);

	while(ray_trace_drive(state)) {}

	bool success = ray_trace_complete(state, hit);

	vec3 color = vec3(1);

	if(success) {
		u32 id = u32(hit.voxel_hit.id);
		f32 noise_factor = 0.5;
		if(id == 2) {
			color *= mix(vec3(170, 255, 21) / 256, vec3(34, 139, 34) / 256, noise_factor);
		}
		if(id == 3) {
			color *= mix(vec3(135) / 256, vec3(80) / 256, noise_factor);
		}

		if(id == 4) {
			color *= mix(vec3(107, 84, 40) / 256, vec3(64, 41, 5) / 256, noise_factor);
		}

		Ao ao;
		ao.region_data = path.block_data;
		ao.pos = hit.voxel_hit.back_step; 
		ao.d1 = abs(hit.voxel_hit.normal.zxy); 
		ao.d2 = abs(hit.voxel_hit.normal.yzx);

		vec4 ambient = voxel_ao(ao);
		
		color *= 0.75 + 0.25 * mix(mix(ambient.z, ambient.w, hit.voxel_hit.uv.x), mix(ambient.y, ambient.x, hit.voxel_hit.uv.x), hit.voxel_hit.uv.y);
	} else {
		color = vec3(0.1, 0.2, 1.0);
	}

	return color;
}
