#define MAX_STEP_COUNT 512

struct Ray {
	BufferId region_id;
	vec3 origin;
	vec3 direction;
	vec3 minimum;
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
	vec3 map_pos;
	f32 dist;
	f32 initial_dist;
	bvec3 mask;	
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


void ray_cast_start(Ray ray, out RayState state) {
	ray.direction = normalize(ray.direction);

	vec3 delta_dist = abs(vec3(length(ray.direction)) / ray.direction);
	
	state.id = RAY_STATE_INITIAL;
	state.map_pos = ray.origin;
	state.mask = bvec3(false);
	state.dist = 0;
	state.initial_dist = 0;
	state.block_id = u16(0);
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
		hit.dist = state.dist;
		hit.total_dist = state.initial_dist + state.dist;
		hit.ray = state.ray;
		return true;
	}

	return false;
}

bool ray_cast_drive(inout RayState state) {
	Buffer(Region) region = get_buffer(Region, state.ray.region_id);
	
	vec3 s = sign(state.ray.direction);
	vec3 s01 = max(s, 0.);
	vec3 ird = 1.0 / state.ray.direction;

	bool in_chunk = all(greaterThanEqual(state.map_pos, vec3(state.ray.minimum))) && all(lessThan(state.map_pos, vec3(state.ray.maximum)));

	if(!in_chunk) {
		state.id = RAY_STATE_OUT_OF_BOUNDS;
		return false;
	}

	int lod = 0;
	float voxel = 1;
	VoxelQuery query;
	bool voxel_found = false;
	for(lod = LOD; lod >= 1; lod--) {
		query.region_data = region.lod[lod - 1];
		voxel = pow(2, lod);
		query.position = ivec3(state.map_pos / voxel);

		if(voxel_query(query)) {
			break;
		}
	}
		
	voxel = pow(2, lod);

	vec3 t_max = ird * (voxel * s01 - mod(state.map_pos, voxel));

	state.mask = lessThanEqual(t_max.xyz, min(t_max.yzx, t_max.zxy));

	float c_dist = min(min(t_max.x, t_max.y), t_max.z);
	state.map_pos += c_dist * state.ray.direction;
	state.dist += c_dist;
		
	state.map_pos += 4e-4 * s * vec3(state.mask);

	if(state.initial_dist + state.dist > state.ray.max_distance) {
		state.id = RAY_STATE_MAX_DIST_REACHED;
		return false;	
	}
	
	query.region_data = region.data;
	query.position = ivec3(state.map_pos);

	voxel_found = voxel_query(query);

	//1 is air
	if (voxel_found && query.id != state.ray.medium) {
		state.id = RAY_STATE_VOXEL_FOUND;
		state.block_id = query.id;
		return false;
	}

	return true;
}
