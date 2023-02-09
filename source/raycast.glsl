#define MAX_STEP_COUNT 512

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

#define RAY_STATE_INITIAL 0
#define RAY_STATE_OUT_OF_BOUNDS 1
#define RAY_STATE_MAX_DIST_REACHED 2
#define RAY_STATE_VOXEL_FOUND 3

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
	Ray ray;
};

struct RayHit {
	f32 dist;
	f32 total_dist;
	ivec3 normal;
	ivec3 back_step;
	vec3 destination;
	vec2 uv;
	u32 id;
	Ray ray;
};


void ray_cast_start(inout Ray ray, out RayState state) {
	ray.direction = normalize(ray.direction);

	state.id = RAY_STATE_INITIAL;
	state.map_pos = ivec3(floor(ray.origin + 0.));
	state.mask = bvec3(ivec3(-1 * clamp(ray.direction, -1, 0)));
	state.dist = 0;
	state.initial_dist = 0;
	state.block_id = u16(0);
	state.ray = ray;
	state.delta_dist = 1.0 / abs(state.ray.direction);
	state.ray_step = ivec3(sign(state.ray.direction));
	state.side_dist = (sign(state.ray.direction) * (vec3(state.map_pos) - state.ray.origin) + (sign(state.ray.direction) * 0.5) + 0.5) * state.delta_dist;

}

bool ray_cast_complete(inout RayState state, out RayHit hit) {
	ivec3 ray_step = ivec3(sign(state.ray.direction));

	if(state.id == RAY_STATE_VOXEL_FOUND) {
		vec3 destination = state.ray.origin + state.ray.direction * state.dist;
		ivec3 back_step = ivec3(state.map_pos - ray_step * vec3(state.mask));
		vec2 uv = mod(vec2(dot(vec3(state.mask) * destination.yzx, vec3(1.0)), dot(vec3(state.mask) * destination.zxy, vec3(1.0))), vec2(1.0));
		ivec3 normal = ivec3(vec3(state.mask) * sign(-state.ray.direction));

		hit.destination = destination;
		hit.back_step = back_step;
		hit.uv = uv;
		hit.normal = normal;
		hit.id = state.block_id;
		hit.dist = state.dist;
		hit.total_dist = state.initial_dist + state.dist;
		hit.ray = state.ray;
		return true;
	}

	return false;
}

bool ray_cast_drive(inout RayState state) {
	if(state.id != RAY_STATE_INITIAL) {
		return false;
	}

	if(state.initial_dist + state.dist > state.ray.max_distance) {
		state.id = RAY_STATE_MAX_DIST_REACHED;
		return false;	
	}
	
	bool in_chunk = all(greaterThanEqual(state.map_pos, vec3(state.ray.minimum) + EPSILON)) && all(lessThan(state.map_pos, vec3(state.ray.maximum) - EPSILON));
	bool rough_in_chunk = all(greaterThanEqual(state.map_pos, vec3(state.ray.minimum) - 1)) && all(lessThan(state.map_pos, vec3(state.ray.maximum) + 1));
	if(!rough_in_chunk) {
		state.id = RAY_STATE_OUT_OF_BOUNDS;
		return false;
	}

	VoxelQuery query;
	query.region_data = state.ray.region_data;
	query.position = state.map_pos;

	bool voxel_found = voxel_query(query);

	if (in_chunk && voxel_found && query.id != state.ray.medium) {
		state.id = RAY_STATE_VOXEL_FOUND;
		state.block_id = query.id;
		return false;
	}

	state.mask = lessThanEqual(state.side_dist.xyz, min(state.side_dist.yzx, state.side_dist.zxy));
        state.side_dist += vec3(state.mask) * state.delta_dist;
        state.map_pos += ivec3(vec3(state.mask)) * state.ray_step;
	state.dist = length(vec3(state.mask) * (state.side_dist - state.delta_dist));
	

	return true;
}
