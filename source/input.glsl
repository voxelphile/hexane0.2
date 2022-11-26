#version 450

#include "hexane.glsl"
#include "rigidbody.glsl"
#include "transform.glsl"

struct InputPush {
	BufferId info_id;
	BufferId transform_id;
};

decl_push_constant(InputPush)

decl_buffer(
	Camera,
	{
		mat4 projection;
	}
)

struct EntityInput {
	bool up;
	bool down;
	bool left;
	bool right;
	bool forward;
	bool backward;
	vec4 look;
};

decl_buffer(
	Info,
	{
		f32 time;
		f32 delta_time;
		EntityInput entity_input;
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

#define COLLIDE_DELTA 0.09

void main() {
	if(gl_GlobalInvocationID.x != 0) {
		return;
	}

	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Info) info = get_buffer(Info, push_constant.info_id);

	f32 delta_time = info.delta_time;

	Transform transform = transforms.transform;
	EntityInput entity_input = info.entity_input;

	if(!transform.first){
		transform.position.xyz = vec3(32, 200, 32);
		transform.velocity.xyz = vec3(0);
		transform.rotation.xyz = vec3(-3.14 / 2.0 + 0.1, 0, 0);
		transform.jumping = false;
		transform.first = true;
	}
	if(entity_input.down && entity_input.up){
		transform.velocity.xyz = vec3(0);
	}
	
	f32 sens = 3.14 / 4;

	transform.rotation.xy -= (entity_input.look.yx * delta_time) * sens;

	transform.rotation.x = clamp(transform.rotation.x, -3.14 / 2.0 + 0.1, 3.14 / 2.0 - 0.1);

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

	transform.velocity.xyz += direction * 0.25;
	transform.position.xyz += transform.velocity.xyz * delta_time;
	
	transforms.transform = transform;
	/*
	{
		Rigidbody rigidbody = rigidbody_buffer.info[entity_input.entity_id];
patreon.com/user?u=82729947
		if(rigidbody.id != 0) {
			apply_force(rigidbody, vec3(0), 10);
		}
	}*/
}
#endif
