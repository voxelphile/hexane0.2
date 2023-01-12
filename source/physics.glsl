#version 450

#include "hexane.glsl"
#include "rigidbody.glsl"
#include "info.glsl"
#include "transform.glsl"
#include "world.glsl"
#include "voxel.glsl"

struct PhysicsPush {
	f32 fixed_time;
	BufferId info_id;
	BufferId transform_id;
	BufferId rigidbody_id;
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

	f32 fixed_time = push_constant.fixed_time;

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

	response.entry_time = max(entry.x, max(entry.y, entry.z));
	response.exit_time = min(exit.x, min(exit.y, exit.z));

	if(response.entry_time > response.exit_time 
			|| all(lessThan(entry, vec3(0))) 
			|| any(greaterThan(entry, vec3(1)))) {
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

	b.dimensions.x = a.velocity.x > 0 ? 
		a.velocity.x + a.dimensions.x 
		: a.dimensions.x - a.velocity.x; 

	b.dimensions.y = a.velocity.y > 0 ? 
		a.velocity.y + a.dimensions.y 
		: a.dimensions.y - a.velocity.y; 

	b.dimensions.z = a.velocity.z > 0 ? 
		a.velocity.z + a.dimensions.z 
		: a.dimensions.z - a.velocity.z; 
	return b;
}

struct AxisData {
	vec3 normals;
	f32 entry_time;
	bool colliding;
	vec3 velocity;
	vec3 acceleration;
	Box block;
};

void swap(inout i32 a, inout i32 b) {
	i32 temp = a;
	a = b;
	b = temp;
}

void main() {
	return;

	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Rigidbodies) rigidbodies = get_buffer(Rigidbodies, push_constant.rigidbody_id);
	Buffer(World) world = get_buffer(World, push_constant.world_id);

	Transform transform = transforms.data[0];
	Rigidbody rigidbody = rigidbodies.data[0];

	rigidbody.on_ground = false;

	f32 fixed_time = push_constant.fixed_time;
	
	f32 mag = ceil(length(rigidbody.velocity.xyz));

	i32 h_mag = 5;
	
	AxisData d;
	d.normals = vec3(0);
	d.entry_time = 0;
	d.colliding = false;
	d.velocity = vec3(0);

	AxisData data[] = AxisData[](d, d, d);
	
	data[0].velocity = vec3(rigidbody.velocity.x, 0, 0);
	data[1].velocity = vec3(0, rigidbody.velocity.y, 0);
	data[2].velocity = vec3(0, 0, rigidbody.velocity.z);

	data[0].acceleration = vec3(rigidbody.acceleration.x, 0, 0);
	data[1].acceleration = vec3(0, rigidbody.acceleration.y, 0);
	data[2].acceleration = vec3(0, 0, rigidbody.acceleration.z);

	i32 order[] = i32[](0, 1, 2);
		
	Box player;
	player.dimensions = vec3(0.8, 2, 0.8);
	player.position = transform.position.xyz;

	for(i32 i = 0; i < 3; i++) {
	for(i32 x = -h_mag; x < h_mag; x++) {
	for(i32 y = -h_mag; y < h_mag; y++) {
	for(i32 z = -h_mag; z < h_mag; z++) {
		
		Box block;
		block.position = floor(transform.position.xyz) + vec3(x, y, z);
		block.dimensions = vec3(1);
		player.velocity = data[i].velocity + data[i].acceleration * fixed_time;

		Box broadphase = get_swept_broadphase_box(player);	

		if(aabb_check(broadphase, block)) {
		
		u32 chunk = u32(block.position.x + block.position.y / CHUNK_SIZE + block.position.z / CHUNK_SIZE / CHUNK_SIZE);

		VoxelQuery query;
		query.chunk_id = world.chunks[chunk];
		query.position = block.position;
		
		if(!voxel_query(query)) {
			continue;
		}

		CollisionResponse response;
		if(swept_aabb(player, block, response)) {
			query.position += response.normal;
			if(voxel_query(query)) {
				continue;
			}
			if(response.entry_time > fixed_time) {
				continue;
			}

			if(response.entry_time >= data[i].entry_time) {
			data[i].colliding = true;
			
			data[i].normals = response.normal.xyz;

			data[i].entry_time = max(data[i].entry_time, response.entry_time);

			data[i].block = block;
			}	
		}
		}
	}
	}
	}
	}
	
	player.velocity = vec3(0);

	if(data[0].entry_time > data[1].entry_time) swap(order[0], order[1]);
	if(data[1].entry_time > data[2].entry_time) swap(order[1], order[2]);
	if(data[0].entry_time > data[1].entry_time) swap(order[0], order[1]);

	for(i32 i = 2; i >= 0; i--) {
		i32 o = order[i];

		if(data[o].colliding) {
			transform.position.xyz += data[o].velocity * data[o].entry_time;
			while(aabb_check(player, data[o].block)) {
				transform.position.xyz += data[o].normals * 1e-4;
				player.position = transform.position.xyz;
			}
			transform.position.xyz += data[o].normals * 1e-3;

		}
		
		data[o].normals = clamp(data[o].normals, -1, 1);
	
		vec3 undesired_velocity = data[o].normals * dot(rigidbody.velocity.xyz, data[o].normals);
	
		rigidbody.velocity.xyz -= undesired_velocity;

		data[o].velocity -= undesired_velocity;
		
		vec3 undesired_acceleration = data[o].normals * dot(rigidbody.acceleration.xyz, data[o].normals);

		rigidbody.acceleration.xyz -= undesired_acceleration;

		data[o].acceleration -= undesired_acceleration;

		transform.position.xyz += data[o].velocity * fixed_time
			+ 0.5 * data[o].acceleration * pow(fixed_time, 2);
		rigidbody.velocity.xyz += data[o].acceleration * fixed_time;

		if(data[o].normals == vec3(0,1,0)) {
			rigidbody.on_ground = true;
		}

	}

	rigidbody.acceleration.y -= 4;

	transforms.data[0] = transform;
	rigidbodies.data[0] = rigidbody;
}

#endif
