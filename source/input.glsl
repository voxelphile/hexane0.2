#version 450

#include "hexane.glsl"
#include "rigidbody.glsl"
#include "info.glsl"
#include "noise.glsl"
#include "sound.glsl"
#include "region.glsl"
#include "voxel.glsl"
#include "transform.glsl"

struct InputPush {
	BufferId info_id;
	BufferId transform_id;
	BufferId rigidbody_id;
	BufferId input_id;
	BufferId sound_id;
	BufferId mersenne_id;
	BufferId region_id;
};

decl_push_constant(InputPush)

decl_buffer(
	Camera,
	{
		mat4 projection;
	}
)

decl_buffer(
	Input,
	{
		bool first;
		vec4 target_rotation;
		vec2 target_lateral_velocity;
		vec3 last_position;
		bool jumping;
		f32 target_rotation_time;
		f32 coyote_counter;
	}
)
	
#ifdef compute

#define SPEED 1
#define SPEED_OF_LIGHT 1000000000

layout (local_size_x = 256) in;

#define SAMPLES 6

#define HEIGHT 1.778
#define WIDTH 1
#define DEPTH 1

#define GRAVITY -9
#define ENABLE_FLIGHT false
#define COLLIDE_DELTA 0.09

void main() {
	if(gl_GlobalInvocationID.x != 0) {
		return;
	}

	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Rigidbodies) rigidbodies = get_buffer(Rigidbodies, push_constant.rigidbody_id);
	Buffer(Info) info = get_buffer(Info, push_constant.info_id);
	Buffer(Input) inp = get_buffer(Input, push_constant.input_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);

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

	f32 step_distance = 4;

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

	inp.target_lateral_velocity = direction.xz * 20;

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
