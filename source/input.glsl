#version 450

#include "hexane.glsl"
#include "rigidbody.glsl"
#include "info.glsl"
#include "noise.glsl"
#include "sound.glsl"
#include "region.glsl"
#include "voxel.glsl"
#include "camera.glsl"
#include "raycast.glsl"
#include "transform.glsl"

struct InputPush {
	BufferId info_id;
	BufferId transform_id;
	BufferId rigidbody_id;
	BufferId input_id;
	BufferId sound_id;
	BufferId mersenne_id;
	BufferId region_id;
	BufferId camera_id;
};

decl_push_constant(InputPush)

decl_buffer(
	Input,
	{
		bool first;
		vec4 target_rotation;
		vec2 target_lateral_velocity;
		vec3 last_position;
		bool running;
		bool sprinting;
		bool jumping;
		f32 target_rotation_time;
		f32 last_action_time;
		f32 last_forward_time;
		u32 forward_counter;
		bool was_forward;
		f32 coyote_counter;
	}
)
	
#ifdef compute

#define HUMAN_FACTOR 7.3
#define ENABLE_FLIGHT false

layout (local_size_x = 256) in;

void main() {
	if(gl_GlobalInvocationID.x != 0) {
		return;
	}

	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Rigidbodies) rigidbodies = get_buffer(Rigidbodies, push_constant.rigidbody_id);
	Buffer(Info) info = get_buffer(Info, push_constant.info_id);
	Buffer(Input) inp = get_buffer(Input, push_constant.input_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);

	f32 delta_time = info.delta_time;

	Transform transform = transforms.data[0];
	Rigidbody rigidbody = rigidbodies.data[0];
	EntityInput entity_input = info.entity_input;

	if(!inp.first){
		transform.position.xyz = vec3(128, 150, 128);
		inp.last_position = transform.position.xyz;
		rigidbody.velocity.xyz = vec3(0);
		inp.target_rotation.xyz = vec3(-3.14 / 2.0 + 0.1, 0, 0);
		inp.first = true;
	}

	if(inp.last_action_time > 0.15 && (entity_input.action1 || entity_input.action2)) {
		Transform region_transform = transform;
		ivec3 diff = region.floating_origin - region.observer_position;
		region_transform.position.xyz = vec3(REGION_SIZE / 2) - vec3(diff);
		region_transform.position.xyz += transforms.data[0].position.xyz - region.observer_position;
	
		vec2 screenPos = vec2(0);
		vec4 far = camera.inv_projection * vec4(screenPos, 1, 1);
		far /= far.w;
		vec4 near = camera.inv_projection * vec4(screenPos, 0, 1);
		near /= near.w;
		vec3 origin = (compute_transform_matrix(region_transform) * near).xyz;
		vec3 dir = (compute_transform_matrix(region_transform) * vec4(normalize(far.xyz), 0)).xyz;
	
		Ray ray;
		ray.region = region;
		ray.origin = origin;
		ray.direction = dir;
		ray.max_distance = 10; 
		ray.minimum = vec3(0);
		ray.maximum = vec3(REGION_SIZE);

		RayHit hit;

		bool success = ray_cast(ray, hit);

		if(success) {
			inp.last_action_time = 0;

			VoxelChange change;
			change.region_data = region.data;

			if(entity_input.action1) {
				change.id = u16(1);
				change.position = ivec3(vec3(hit.destination.xyz - vec3(hit.normal.xyz) * 0.5));
			}
			if(entity_input.action2) {
				change.id = u16(3);
				change.position = ivec3(vec3(hit.destination.xyz + vec3(hit.normal.xyz) * 0.5));
			}

			voxel_change(change);
		}
	}
	
	inp.last_action_time += delta_time;

	f32 step_distance = HUMAN_FACTOR * mix(0.64, 0.74, f32(random(push_constant.mersenne_id)) / f32(~0u));

	if(distance(inp.last_position, transform.position.xyz) > step_distance && rigidbody.on_ground) {
		ivec3 diff = region.floating_origin - region.observer_position;
		vec3 region_position = vec3(REGION_SIZE / 2) - vec3(diff);
		region_position += transforms.data[0].position.xyz - region.observer_position;
		VoxelQuery query;
		query.region_data = region.data;
		query.position = ivec3(region_position) + ivec3(0, -2, 0);

		if(voxel_query(query)) {
			play_sound_for_block_id(push_constant.sound_id, push_constant.mersenne_id, query.id);
		}

		inp.last_position = transform.position.xyz;
	}

	f32 sens = 0.002;
	f32 rot_rate = exp2(0.01);

	inp.target_rotation.xy -= (entity_input.look.yx) * sens;

	inp.target_rotation.x = clamp(inp.target_rotation.x, -3.14 / 2.0 + 0.1, 3.14 / 2.0 - 0.1);
	if(entity_input.look.xy != vec2(0)) {
		inp.target_rotation_time = 0;
	}
	transform.rotation = mix(transform.rotation, inp.target_rotation, exp2(-rot_rate * delta_time));

	vec3 direction = vec3(0);

	i32vec3 input_axis = i32vec3(0);

	input_axis.x = i32(entity_input.left) - i32(entity_input.right);
	input_axis.y = i32(entity_input.up) - i32(entity_input.down);
	input_axis.z = i32(entity_input.forward) - i32(entity_input.backward);

	mat4 orientation = mat4(
			cos(transform.rotation.y),
			sin(transform.rotation.x) * sin(transform.rotation.y),
			-cos(transform.rotation.x) * sin(transform.rotation.y),
			0,
			0,
			cos(transform.rotation.x),
			sin(transform.rotation.x),
			0,
			sin(transform.rotation.y),
			-sin(transform.rotation.x) * cos(transform.rotation.y),
			cos(transform.rotation.x) * cos(transform.rotation.y),
			0,
			0,
			0,
			0,
			1
	);

	vec4 attitude = orientation * vec4(input_axis.x, 0, input_axis.z, 0);

	vec2 lateral_direction = -attitude.xz;
	
	if(length(lateral_direction) > 0) {
		lateral_direction = normalize(lateral_direction);
	}

	direction.xz = lateral_direction;
	direction.y = f32(input_axis.y);
	
	f32 move_rate = exp2(1);

	//TODO make this a game mechanic

	inp.last_forward_time += delta_time;

	if(entity_input.forward) {
		inp.last_forward_time = 0;
	} 

	if(entity_input.forward && !inp.was_forward) {
		inp.forward_counter++;
	}

	inp.was_forward = entity_input.forward;

	if(!entity_input.forward && inp.last_forward_time > 0.2 || (rigidbody.hit_something && rigidbody.velocity.y <= 0)) {
		inp.forward_counter = 0;
	}

	u32 double_tap = 2;
	u32 triple_tap = 3;

	inp.running = inp.forward_counter >= double_tap;
	inp.sprinting = inp.forward_counter >= triple_tap;

	float speed = inp.sprinting ? 3.0 : inp.running ? 2.3 : 1.3;

	inp.target_lateral_velocity = direction.xz * speed * HUMAN_FACTOR;

	rigidbody.velocity.xz = mix(rigidbody.velocity.xz, inp.target_lateral_velocity, exp2(-move_rate * delta_time));

	if(rigidbody.on_ground) {
		inp.coyote_counter = 0;	
		inp.jumping = false;
	} else {
		inp.coyote_counter += delta_time;
	}

	f32 coyote_time = 0.3;

	if(input_axis.y == 1 && !inp.jumping && inp.coyote_counter < coyote_time) {
		rigidbody.velocity.y += 10;
		inp.jumping = true;
	}

	if(ENABLE_FLIGHT) {
		transform.position.xyz += direction.xyz * 1000 * delta_time;
	}
	
	transforms.data[0] = transform;
	rigidbodies.data[0] = rigidbody;
	/*
	{
		Rigidbody rigidbody = rigidbody_buffer.info[entity_input.entity_id];

		if(rigidbody.id != 0) {
			apply_force(rigidbody, vec3(0), 10);
		}
	}*/
}
#endif
