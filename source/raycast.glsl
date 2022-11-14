DECL_BUFFER_STRUCT(
	OctreeBuffer,
	{
		u32 size;
		u32 len;
		Node data[1048576];
	}
)
float get_refraction(uint id) {
	float refraction;

	if (id == 1) {
		refraction = 1.5;
	} else if (id == 2) {
		refraction = 1.3;
	} else if (id == 42069) {
		refraction = 1.000;
	} else if (id == 3) {
		refraction = 1.5;
	}

	return refraction;
}

float get_reflectivity(uint id) {
	float reflectivity;

	if (id == 1) {
		reflectivity = 0.0;
	} else if (id == 2) {
		reflectivity = 0.2;
	} else if (id == 42069) {
		reflectivity = 0.0;
	} else if (id == 3) {
		reflectivity = 0.0;
	}

	return reflectivity;
}

vec4 get_albedo(uint id) {
	vec4 albedo = vec4(0);

	if (id == 2) {
		albedo = vec4(0.25, 1, 0.1, 1);
	} else if (id == 3) {
		albedo = vec4(0, 0.41, 18, 0.1);
	} else if (id == 4) {
		albedo = vec4(.72, .39, .12, 1);
	}

	return albedo;
}

struct Ray {
	vec3 origin;
	vec3 direction;
	float max_dist;
	uint medium;
	bool bounded;
};

struct RayHit {
	uint node;
	vec3 destination;
	vec3 back_step;
	vec3 normal;
	vec3 reflection;
	vec3 refraction;
	vec2 uv;
	float dist;
};

bool get_voxel(vec3 position, out uint node_index, out uint node_depth, bool ignore_transparent) {
	BufferRef(OctreeBuffer) octree_buffer = buffer_id_to_ref(OctreeBuffer, BufferRef, push_constant.octree_buffer_id);
	
	int size = int(pow(2, octree_buffer.size));

	ivec3 p = ivec3(floor(position + 0.0)) ;
	
	int s = size;
	int h = 0;
	int px,py,pz;
	int x = p.x;
	int y = p.y;
	int z = p.z;

	node_index = 0;

	for (node_depth = 0; node_depth < octree_buffer.size; node_depth++) {
		h = s / 2;

		px = int(x >= h);
		py = int(y >= h);
		pz = int(z >= h);
		uint k = px * 4 + py * 2 + pz;
		uint n = 1 << k;
		uint m = octree_buffer.data[node_index].valid & n;
		uint b = bitCount(octree_buffer.data[node_index].valid & (n - 1));

		if (octree_buffer.data[node_index].valid != 0 && m == n)
		{
			node_index = octree_buffer.data[node_index].child + b;
		} else {
			break;
		}

		x -= px * h;
		y -= py * h;
		z -= pz * h;

		s = h;
	}

	Node node = octree_buffer.data[node_index];

	return node.id != 0;
}

bool ray_cast(Ray ray, out RayHit hit) {
	BufferRef(OctreeBuffer) octree_buffer = buffer_id_to_ref(OctreeBuffer, BufferRef, push_constant.octree_buffer_id);
	
	ray.direction = normalize(ray.direction);
	ray.origin += ray.direction * pow(EPSILON, 3);

	vec3 p = ray.origin;
	vec3 s = sign(ray.direction);
	vec3 s01 = max(s, 0.);
	vec3 ird = 1.0 / ray.direction;
	
	bvec3 mask;

	uint size = uint(pow(2, octree_buffer.size));

	bool ignore_transparent = false;

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


		bool voxel_found = get_voxel(p, node_index, node_depth, true);

		int lod = int(octree_buffer.size) - int(node_depth) - 1;

		if (voxel_found) {
			Node current = octree_buffer.data[node_index];

				vec3 destination = ray.origin + ray.direction * (dist - 1e-4);
				vec3 back_step = p - s * vec3(mask);
				vec3 normal = vec3(mask) * sign(-ray.direction);
				vec2 uv = mod(vec2(dot(vec3(mask) * destination.yzx, vec3(1.0)), dot(vec3(mask) * destination.zxy, vec3(1.0))), vec2(1.0));
				vec3 reflection = reflect(ray.direction, normal);
				float eta = get_refraction(ray.medium) / get_refraction(current.id);
				vec3 refraction = refract(ray.direction, normal, eta);

				hit.node = node_index;	
				hit.destination = destination;
				hit.back_step = back_step;
				hit.normal = normal;
				hit.reflection = reflection;
				hit.refraction = refraction;
				hit.uv = uv;
				hit.dist = dist;
				return true;
		}

		float voxel = exp2(lod);
		vec3 t_max = ird * (voxel * s01 - mod(p, voxel));

		mask = lessThanEqual(t_max.xyz, min(t_max.yzx, t_max.zxy));

		float c_dist = min(min(t_max.x, t_max.y), t_max.z);
		p += c_dist * ray.direction;
		dist += c_dist;
	
		if(dist > ray.max_dist) {
			break;
		}

		p += 4e-4 * s * vec3(mask);
	}

	return false;
}


float vertex_ao(vec2 side, float corner) {
	return (side.x + side.y + max(corner, side.x * side.y)) / 3.0;
}

vec4 voxel_ao(vec3 pos, vec3 d1, vec3 d2) {
	uint _;

	vec4 side = vec4(
			float(get_voxel(pos + d1, _, _, true)), 
			float(get_voxel(pos + d2, _, _, true)), 
			float(get_voxel(pos - d1, _, _, true)), 
			float(get_voxel(pos - d2, _, _, true))
			);

	vec4 corner = vec4(
			float(get_voxel(pos + d1 + d2, _, _, true)), 
			float(get_voxel(pos - d1 + d2, _, _, true)), 
			float(get_voxel(pos - d1 - d2, _, _, true)), 
			float(get_voxel(pos + d1 - d2, _, _, true))
			);

	vec4 ao;
	ao.x = vertex_ao(side.xy, corner.x);
	ao.y = vertex_ao(side.yz, corner.y);
	ao.z = vertex_ao(side.zw, corner.z);
	ao.w = vertex_ao(side.wx, corner.w);
	return 1.0 - ao;
}
