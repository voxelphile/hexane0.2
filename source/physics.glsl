#version 450

#include "hexane.glsl"
#include "rigidbody.glsl"
#include "info.glsl"
#include "transform.glsl"
#include "region.glsl"
#include "voxel.glsl"
#include "aabb.glsl"
#include "bounding.glsl"

struct PhysicsPush {
	f32 fixed_time;
	BufferId info_id;
	BufferId transform_id;
	BufferId rigidbody_id;
	BufferId region_id;
	BufferId bounding_id;
};

decl_push_constant(PhysicsPush)

#ifdef compute

layout (local_size_x = 256) in;

#define DIMENSIONS 3

struct CollisionResponse {
	vec3 normal;
	f32 entry_time;
	f32 exit_time;
};


bool swept_aabb(Box a, Box b, vec3 velocity, inout CollisionResponse response) {
	Buffer(Info) info = get_buffer(Info, push_constant.info_id);

	f32 fixed_time = push_constant.fixed_time;

	vec3 inv_entry, inv_exit;

	bvec3 i = greaterThan(velocity, vec3(0));
	bvec3 j = equal(velocity, vec3(0));

	f32vec3 i_gt = f32vec3(i);
	f32vec3 i_lte = f32vec3(not(i));
	f32vec3 j_eq = f32vec3(j);
	f32vec3 j_neq = f32vec3(not(j));

	f32vec3 m = b.position - (a.position + a.dimensions);
	f32vec3 n = (b.position + b.dimensions) - a.position;

	inv_entry = i_gt * m + i_lte * n;
	inv_exit = i_gt * n + i_lte * m;

	vec3 entry, exit;

	entry = j_eq * -10000 + j_neq * (inv_entry / velocity); 
	exit = j_eq * 10000 + j_neq * (inv_exit / velocity); 

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

Box get_swept_broadphase_box(Box a, vec3 velocity) {
	Box b;
	b.position.x = velocity.x > 0 ? 
		a.position.x 
		: a.position.x + velocity.x;

	b.position.y = velocity.y > 0 ? 
		a.position.y 
		: a.position.y + velocity.y;

	b.position.z = velocity.z > 0 ? 
		a.position.z 
		: a.position.z + velocity.z;

	b.dimensions.x = velocity.x > 0 ? 
		velocity.x + a.dimensions.x 
		: a.dimensions.x - velocity.x; 

	b.dimensions.y = velocity.y > 0 ? 
		velocity.y + a.dimensions.y 
		: a.dimensions.y - velocity.y; 

	b.dimensions.z = velocity.z > 0 ? 
		velocity.z + a.dimensions.z 
		: a.dimensions.z - velocity.z; 
	return b;
}

bool inside_of(Box subject, Box outer) {
	return all(greaterThanEqual(subject.position, outer.position)) && all(lessThanEqual(subject.position + subject.dimensions * 0.5, outer.position + outer.dimensions));
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

#define ENABLE_PHYSICS false

void main() {
	if(gl_GlobalInvocationID != uvec3(0) || !ENABLE_PHYSICS) 
		return;

	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Rigidbodies) rigidbodies = get_buffer(Rigidbodies, push_constant.rigidbody_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Buffer(Bounding) bounding = get_buffer(Bounding, push_constant.bounding_id);

	if(transforms.physics) {
		return;
	}

	Transform transform = transforms.data[0]; 
	Transform eye_transform = transform;
	ivec3 diff = region.floating_origin - region.observer_position;
	eye_transform.position.xyz = vec3(REGION_SIZE / 2) - vec3(diff);
	eye_transform.position.xyz += transforms.data[0].position.xyz - region.observer_position;
	eye_transform.position.xyz -= vec3(0.4, 1.8, 0.4);
	Rigidbody rigidbody = rigidbodies.data[0];

	rigidbody.on_ground = false;
	rigidbody.hit_something = false;

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
	player.dimensions = vec3(0.8, 1.9, 0.8);
	player.position = eye_transform.position.xyz;

	for(i32 i = 0; i < 3; i++) {
	for(i32 x = -h_mag; x < h_mag; x++) {
	for(i32 y = -h_mag; y < h_mag; y++) {
	for(i32 z = -h_mag; z < h_mag; z++) {
		
		Box block;
		block.position = floor(player.position) + vec3(x, y, z);
		block.dimensions = vec3(1);
		vec3 velocity = data[i].velocity + data[i].acceleration * fixed_time;
		

		Box broadphase = get_swept_broadphase_box(player, velocity);	

		if(aabb_check(broadphase, block)) {

		VoxelQuery query;
		query.region_data = region.data;
		query.position = ivec3(block.position);

		bool voxel_found = voxel_query(query);

		if (!voxel_found || !is_solid(query.id)) {
			continue;
		}
		
		Box bounding_box = bounding.bounds[query.id].boxes[0]; 
		bounding_box.position += block.position;

		f32 clip = 0.05;
		Box inner_clip;
		inner_clip.position = bounding_box.position + clip;
		inner_clip.dimensions = bounding_box.dimensions - 2 * clip;
		
		while(aabb_check(inner_clip, player) && velocity.y <= 0) {
			player.position.y += 1e-1;
			transform.position.y += 1e-1;
		}
		
		CollisionResponse response;
		if(swept_aabb(player, bounding_box, velocity, response)) {
			query.position = ivec3(query.position) + ivec3(response.normal);
			bool voxel_found = voxel_query(query);

			if(voxel_found && is_solid(query.id)) {
				continue;
			}
			if(response.entry_time > fixed_time) {
				continue;
			}

			if(response.entry_time >= data[i].entry_time) {
			data[i].colliding = true;
			
			data[i].normals = response.normal.xyz;

			data[i].entry_time = max(data[i].entry_time, response.entry_time);

			data[i].block = bounding_box;
			}	
		}
		}
	}
	}
	}
	}
	
	player.velocity = vec3(0);

	if(data[0].entry_time < data[1].entry_time) swap(order[0], order[1]);
	if(data[1].entry_time < data[2].entry_time) swap(order[1], order[2]);
	if(data[0].entry_time < data[1].entry_time) swap(order[0], order[1]);

	for(i32 i = DIMENSIONS - 1; i >= 0; i--) {
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
		if(abs(data[o].normals.z) + abs(data[0].normals.x) >= 1) {
			rigidbody.hit_something = true;
		}

	}

	rigidbody.acceleration.y -= 4;

	VoxelQuery query;
	query.region_data = region.data;
	query.position = i32vec3(player.position.xyz);

	voxel_query(query);
	
	VoxelQuery query2;
	query2.region_data = region.data;
	query2.position = i32vec3(player.position.xyz) + i32vec3(0, 1, 0);

	voxel_query(query2);

	Box portal;
	portal.dimensions = vec3(1, 2, 1);
	portal.position = vec3(i32vec3(player.position.xyz));

	if(query.id == u16(6) && query2.id == u16(6) && inside_of(player, portal)) {
		transform.position.xyz = region.floating_origin + fract(player.position.xyz);	
	}

	f32 rot_rate = exp2(1);
	
	rigidbody.velocity.xyz = mix(rigidbody.velocity.xyz, min(rigidbody.velocity.xyz, vec3(terminal_velocity(query.id))), exp2(-rot_rate * push_constant.fixed_time));

	transforms.data[0] = transform;
	rigidbodies.data[0] = rigidbody;
}

#endif
