#define MAX_STEP_COUNT 512

struct Ray {
	ImageId chunk_id;
	vec3 origin;
	vec3 direction;
};

struct RayHit {
	f32 dist;
	vec3 normal;
	vec3 back_step;
	vec3 destination;
	bvec3 mask;
	u32 id;
};

bool ray_cast(inout Ray ray, out RayHit hit) {
	ray.direction = normalize(ray.direction);


	vec3 map_pos = ivec3(floor(ray.origin + 0.));
	vec3 delta_dist = abs(vec3(length(ray.direction)) / ray.direction);
	ivec3 ray_step = ivec3(sign(ray.direction));
	vec3 side_dist = (sign(ray.direction) * (vec3(map_pos) - ray.origin) + (sign(ray.direction) * 0.5) + 0.5) * delta_dist;
	bvec3 mask;

	for(int i = 0; i < MAX_STEP_COUNT; i++) {
		bool in_chunk = all(greaterThanEqual(map_pos, vec3(-EPSILON))) && all(lessThan(map_pos, vec3(CHUNK_SIZE + EPSILON)));

		if(!in_chunk) {
			return false;
		}

		VoxelQuery query;
		query.chunk_id = ray.chunk_id;
		query.position = map_pos;

		bool voxel_found = voxel_query(query);

		if (voxel_found) {
			float dist = length(vec3(mask) * (side_dist - delta_dist));
			vec3 destination = ray.origin + ray.direction * dist;
			vec3 back_step = map_pos - ray_step * vec3(mask);

			hit.destination = destination;
			hit.back_step = back_step;
			hit.mask = mask;
			hit.id = query.id;
			return true;
		}

		mask = lessThanEqual(side_dist.xyz, min(side_dist.yzx, side_dist.zxy));
			
		side_dist += vec3(mask) * delta_dist;
		map_pos += ivec3(vec3(mask)) * ray_step;
	}

	return false;
}
