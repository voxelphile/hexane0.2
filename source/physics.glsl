#version 450

#include "hexane.glsl"
#include "rigidbody.glsl"
#include "info.glsl"
#include "transform.glsl"
#include "world.glsl"
#include "voxel.glsl"

struct PhysicsPush {
	BufferId info_id;
	BufferId transform_id;
	BufferId world_id;
};

decl_push_constant(PhysicsPush)

#ifdef compute

layout (local_size_x = 256) in;

struct Box {
	vec3 position;
	vec3 dimensions;
	vec3 velocity;
};

struct CollisionResponse {
	vec3 normal;
	f32 entry_time;
	f32 exit_time;
};

bool swept_aabb(Box a, Box b, inout CollisionResponse response) {
	Buffer(Info) info = get_buffer(Info, push_constant.info_id);

	f32 delta_time = info.delta_time;

	vec3 inv_entry, inv_exit;

	bvec3 i = greaterThan(a.velocity, vec3(0));
	bvec3 j = equal(a.velocity, vec3(0));

	f32vec3 i_gt = f32vec3(i);
	f32vec3 i_lte = f32vec3(not(i));
	f32vec3 j_eq = f32vec3(j);
	f32vec3 j_neq = f32vec3(not(j));

	f32vec3 m = b.position - (a.position + a.dimensions);
	f32vec3 n = (b.position + b.dimensions) - a.position;

	inv_entry = i_gt * m + i_lte * n;
	inv_exit = i_gt * n + i_lte * m;

	vec3 entry, exit;

	entry = j_eq * -10000 + j_neq * (inv_entry / a.velocity); 
	exit = j_eq * 10000 + j_neq * (inv_exit / a.velocity); 

	response.entry_time = clamp(max(entry.x, max(entry.y, entry.z)), 0, 1);
	response.exit_time = clamp(min(exit.x, min(exit.y, exit.z)),0, 1);

	if(response.entry_time > response.exit_time || all(lessThan(entry, vec3(0))) || any(greaterThan(entry, vec3(delta_time)))) {
		response.normal = vec3(0);
		response.entry_time = 1.0;
		response.exit_time = 0.0;
		return false;
	}

	if(entry.x == response.entry_time) {
		if(inv_entry.x < 0) {
			response.normal = vec3(1, 0, 0);
		} else {
			response.normal = vec3(-1, 0, 0);
		}

	}
	if(entry.y == response.entry_time) {
		if(inv_entry.y < 0) {
			response.normal = vec3(0, 1, 0);
		} else {
			response.normal = vec3(0, -1, 0);
		}
	}
	if(entry.z == response.entry_time) {
		if(inv_entry.z < 0) {
			response.normal = vec3(0, 0, 1);
		} else {
			response.normal = vec3(0, 0, -1);
		}
	}
	return true;
}

bool aabb_check(Box a, Box b) {
	return !(a.position.x + a.dimensions.x < b.position.x 
		|| a.position.x > b.position.x + b.dimensions.x
		|| a.position.y + a.dimensions.y < b.position.y 
		|| a.position.y > b.position.y + b.dimensions.y
		|| a.position.z + a.dimensions.z < b.position.z 
		|| a.position.z > b.position.z + b.dimensions.z
	);
}

Box get_swept_broadphase_box(Box a) {
	Box b;
	b.position.x = a.velocity.x > 0 ? 
		a.position.x 
		: a.position.x + a.velocity.x;

	b.position.y = a.velocity.y > 0 ? 
		a.position.y 
		: a.position.y + a.velocity.y;

	b.position.z = a.velocity.z > 0 ? 
		a.position.z 
		: a.position.z + a.velocity.z;

	b.velocity.x = a.velocity.x > 0 ? 
		a.velocity.x + a.dimensions.x 
		: a.dimensions.x - a.velocity.x; 

	b.velocity.y = a.velocity.y > 0 ? 
		a.velocity.y + a.dimensions.y 
		: a.dimensions.y - a.velocity.y; 

	b.velocity.z = a.velocity.z > 0 ? 
		a.velocity.z + a.dimensions.z 
		: a.dimensions.z - a.velocity.z; 
	return b;
}


void main() {
	if(gl_GlobalInvocationID.x != 0) {
		return;
	}

	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Info) info = get_buffer(Info, push_constant.info_id);

	f32 delta_time = info.delta_time;
	
	f32 mag = ceil(length(transforms.transform.velocity.xyz));

	i32 h_mag = 5;

	bool collided_y = false;
	vec3 normals = vec3(0);
	f32 entry_time = 10000;
	for(i32 x = -h_mag; x < h_mag; x++) {
	for(i32 y = -h_mag; y < h_mag; y++) {
	for(i32 z = -h_mag; z < h_mag; z++) {
		Box block;
		block.position = floor(transforms.transform.position.xyz) + vec3(x, y, z);
		block.dimensions = vec3(1);
		block.velocity = vec3(0);

		Box player;
		player.dimensions = vec3(1, 2, 1);
		player.position = transforms.transform.position.xyz - vec3(0.5, 1.8, 0.5);
		player.velocity = transforms.transform.velocity.xyz;

		Box broadphase = get_swept_broadphase_box(player);	

		if(aabb_check(broadphase, block)) {

		VoxelQuery query;
		query.world_id = push_constant.world_id;
		query.position = block.position;
		
		if(!voxel_query(query)) {
			continue;
		}

		

		CollisionResponse response;
		if(swept_aabb(player, block, response)) {
			transforms.transform.position.xyz += player.velocity.xyz * delta_time * response.entry_time;

			normals += response.normal.xyz;
		}
		}
	}
	}
	}
		f32vec3 factor = clamp(normals, -1, 1);

		f32vec3 factor_x = vec3(factor.x, 0, 0);
		f32vec3 factor_y = vec3(0, factor.y, 0);
		f32vec3 factor_z = vec3(0, 0, factor.z);

		f32 dot_prod_x = dot(transforms.transform.velocity.xyz, factor_x);
		f32 dot_prod_y = dot(transforms.transform.velocity.xyz, factor_y);
		f32 dot_prod_z = dot(transforms.transform.velocity.xyz, factor_z);

		vec3 undesired_velocity = factor_x * dot_prod_x + factor_y * dot_prod_y + factor_z * dot_prod_z;

		transforms.transform.velocity.xyz -= undesired_velocity;

		transforms.transform.position.xyz += transforms.transform.velocity.xyz * delta_time;
}

#endif
