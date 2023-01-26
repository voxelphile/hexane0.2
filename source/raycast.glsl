#define MAX_STEP_COUNT 512

struct Ray {
	BufferId region_id;
	vec3 origin;
	vec3 direction;
	vec3 minimum;
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
	vec3 map_pos;
	vec3 side_dist;
	f32 dist;
	bvec3 mask;	
	u16 block_id;
	bool currently_tracing;
	bool has_ray_result;
	Ray ray;
};

struct RayHit {
	f32 dist;
	ivec3 normal;
	ivec3 back_step;
	vec3 destination;
	vec2 uv;
	u32 id;
};

void ray_cast_start(Ray ray, out RayState state) {
	ray.direction = normalize(ray.direction);

	vec3 delta_dist = abs(vec3(length(ray.direction)) / ray.direction);
	
	state.id = RAY_STATE_INITIAL;
	state.map_pos = vec3(floor(ray.origin + 0.));
	state.side_dist = (sign(ray.direction) * (vec3(state.map_pos) - ray.origin) + (sign(ray.direction) * 0.5) + 0.5) * delta_dist;
	state.mask = bvec3(false);
	state.dist = 0;
	state.block_id = u16(0);
	state.currently_tracing = true;
	state.has_ray_result = false;
	state.ray = ray;
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
		return true;
	}

	return false;
}

bool ray_cast_drive(inout RayState state) {
	vec3 delta_dist = abs(vec3(length(state.ray.direction)) / state.ray.direction);
	ivec3 ray_step = ivec3(sign(state.ray.direction));

	bool in_chunk = all(greaterThanEqual(state.map_pos, vec3(state.ray.minimum -EPSILON))) && all(lessThan(state.map_pos, vec3(state.ray.maximum + EPSILON)));

	if(!in_chunk) {
		state.id = RAY_STATE_OUT_OF_BOUNDS;
		return false;
	}
	
	state.mask = lessThanEqual(state.side_dist.xyz, min(state.side_dist.yzx, state.side_dist.zxy));
			
	state.side_dist += vec3(state.mask) * delta_dist;
	state.map_pos += ivec3(vec3(state.mask)) * ray_step;
	state.dist = length(vec3(state.mask) * (state.side_dist - delta_dist));

	if(state.dist > state.ray.max_distance) {
		state.id = RAY_STATE_MAX_DIST_REACHED;
		return false;	
	}

	Buffer(Region) region = get_buffer(Region, state.ray.region_id);
	
	VoxelQuery query;
	query.region_data = region.data;
	query.position = ivec3(state.map_pos);

	bool voxel_found = voxel_query(query);

	//1 is air
	if (voxel_found && query.id != 1) {
		state.id = RAY_STATE_VOXEL_FOUND;
		state.block_id = query.id;
		return false;
	}

	return true;
}
