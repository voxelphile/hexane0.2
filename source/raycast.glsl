#define MAX_STEP_COUNT 512

struct Ray {
	Buffer(Region) region;
	vec3 origin;
	vec3 direction;
	vec3 minimum;
	vec3 maximum;
	f32 max_distance;
};

struct RayHit {
	f32 dist;
	ivec3 normal;
	ivec3 back_step;
	vec3 destination;
	vec2 uv;
	u32 id;
};

bool ray_cast(inout Ray ray, out RayHit hit) {
	ray.direction = normalize(ray.direction);
	
	vec3 map_pos = vec3(floor(ray.origin + 0.));
	vec3 delta_dist = abs(vec3(length(ray.direction)) / ray.direction);
	ivec3 ray_step = ivec3(sign(ray.direction));
	vec3 side_dist = (sign(ray.direction) * (vec3(map_pos) - ray.origin) + (sign(ray.direction) * 0.5) + 0.5) * delta_dist;
	bvec3 mask;
	
	f32 dist = 0;

	[[unroll]] for(int i = 0; i < MAX_STEP_COUNT; i++) {
		bool in_chunk = all(greaterThanEqual(map_pos, vec3(ray.minimum -EPSILON))) && all(lessThan(map_pos, vec3(ray.maximum + EPSILON)));

		if(!in_chunk) {
			return false;
		}
		
		mask = lessThanEqual(side_dist.xyz, min(side_dist.yzx, side_dist.zxy));
			
		side_dist += vec3(mask) * delta_dist;
		map_pos += ivec3(vec3(mask)) * ray_step;
		dist = length(vec3(mask) * (side_dist - delta_dist));

		VoxelQuery query;
		query.region_data = ray.region.data;
		query.position = ivec3(map_pos);

		bool voxel_found = voxel_query(query);

		//1 is air
		if (voxel_found && query.id != 1) {
			vec3 destination = ray.origin + ray.direction * dist;
			ivec3 back_step = ivec3(map_pos - ray_step * vec3(mask));
			vec2 uv = mod(vec2(dot(vec3(mask) * destination.yzx, vec3(1.0)), dot(vec3(mask) * destination.zxy, vec3(1.0))), vec2(1.0));
			ivec3 normal = ivec3(vec3(mask) * sign(-ray.direction));

			hit.destination = destination;
			hit.back_step = back_step;
			hit.uv = uv;
			hit.normal = normal;
			hit.id = query.id;
			return true;
		}


		if(dist > ray.max_distance) {
			return false;	
		}
	}

	return false;
}
