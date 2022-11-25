#define MAX_STEP_COUNT 512
#define SIZE 10

struct Ray {
	BufferId bitset_id;
	vec3 origin;
	vec3 direction;
	f32 max_distance;
};

struct RayHit {
	f32 dist;
	vec3 normal;
	vec3 back_step;
	vec3 destination;
};

bool ray_cast(inout Ray ray, out RayHit hit) {
	ray.direction = normalize(ray.direction);
	ray.origin += ray.direction * pow(EPSILON, 3);

	vec3 p = ray.origin;
	vec3 s = sign(ray.direction);
	vec3 s01 = max(s, 0.);
	vec3 ird = 1.0 / ray.direction;
	
	bvec3 mask;

	uint size = uint(pow(2, SIZE));

	float pre_dist = 0;
	vec3 post;

	uint node_index;
	uint node_depth;

	float dist = 0;

	
	//vec3 chunk_min = vec3(in_chunk_position * CHUNK_SIZE);
	//vec3 chunk_max = chunk_min + vec3(CHUNK_SIZE);

	#pragma unroll 
	for (int step_count = 0; step_count < MAX_STEP_COUNT; step_count++) {
		bool in_object = all(greaterThanEqual(p, vec3(0))) && all(lessThan(p, vec3(size)));
		bool rough_in_object = all(greaterThanEqual(p, vec3(-1))) && all(lessThan(p, vec3(size + 1)));

		if (!rough_in_object) {
			break;
		}

		OctreeBitsetQuery query;
		query.bitset_id = ray.bitset_id;
		query.size = 10;
		query.position = p;

		bool voxel_found = query_octree_bitset(query);

		int lod = int(SIZE) - int(node_depth) - 1;

		if (voxel_found) {
			vec3 destination = ray.origin + ray.direction * (dist - 1e-4);
			vec3 normal = vec3(mask) * sign(-ray.direction);
			vec3 back_step = p - s * vec3(mask);

			hit.dist = dist;
			hit.back_step = back_step;	
			hit.normal = normal;
			hit.destination = destination;
			return true;
		}

		float voxel = exp2(lod);
		vec3 t_max = ird * (voxel * s01 - mod(p, voxel));

		mask = lessThanEqual(t_max.xyz, min(t_max.yzx, t_max.zxy));

		float c_dist = min(min(t_max.x, t_max.y), t_max.z);
		p += c_dist * ray.direction;
		dist += c_dist;
	
		if(dist > ray.max_distance) {
			break;
		}

		p += 4e-4 * s * vec3(mask);
	}

	return false;
}
