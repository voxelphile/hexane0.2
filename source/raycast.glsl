#define MAX_STEP_COUNT 512

struct Ray {
	Buffer(Region) region;
	vec3 origin;
	vec3 direction;
};

struct RayHit {
	f32 dist;
	vec3 normal;
	vec3 back_step;
	vec3 destination;
	u32 id;
};

bool ray_cast(inout Ray ray, out RayHit hit) {
	ray.direction = normalize(ray.direction);
	
	u32 chunk = u32(ray.origin.x) / CHUNK_SIZE + u32(ray.origin.y) / CHUNK_SIZE * AXIS_MAX_CHUNKS + u32(ray.origin.z) / CHUNK_SIZE * AXIS_MAX_CHUNKS * AXIS_MAX_CHUNKS;

	vec3 map_pos = vec3(floor(ray.origin + 0.));
	vec3 delta_dist = abs(vec3(length(ray.direction)) / ray.direction);
	ivec3 ray_step = ivec3(sign(ray.direction));
	vec3 side_dist = (sign(ray.direction) * (vec3(map_pos) - ray.origin) + (sign(ray.direction) * 0.5) + 0.5) * delta_dist;
	bvec3 mask;

	uvec3 chunk_pos = uvec3(floor(ray.origin / CHUNK_SIZE)) * CHUNK_SIZE;

	uvec3 minimum = chunk_pos + ray.region.chunks[chunk].minimum;
	uvec3 maximum = chunk_pos + ray.region.chunks[chunk].maximum;

	for(int i = 0; i < MAX_STEP_COUNT; i++) {
		bool in_chunk = all(greaterThanEqual(map_pos, vec3(minimum -EPSILON))) && all(lessThan(map_pos, vec3(maximum + EPSILON)));

		if(!in_chunk) {
			return false;
		}

		VoxelQuery query;
		query.region_data = ray.region.data;
		query.position = uvec3(map_pos);

		bool voxel_found = voxel_query(query);

		//1 is air
		if (voxel_found && query.id != 1) {
			float dist = length(vec3(mask) * (side_dist - delta_dist));
			vec3 destination = ray.origin + ray.direction * dist;
			vec3 back_step = map_pos - ray_step * vec3(mask);
			vec3 normal = vec3(mask) * sign(-ray.direction);
			hit.destination = destination;
			hit.back_step = back_step;
			hit.normal = normal;
			hit.id = query.id;
			return true;
		}

		mask = lessThanEqual(side_dist.xyz, min(side_dist.yzx, side_dist.zxy));
			
		side_dist += vec3(mask) * delta_dist;
		map_pos += ivec3(vec3(mask)) * ray_step;
	}

	return false;
}
