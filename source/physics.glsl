#version 450

#include "hexane.glsl"
#include "octree.glsl"
#include "transform.glsl"
#include "bits.glsl"
#include "raycast.glsl"

struct Push {
	BufferId info_buffer_id;
	BufferId camera_buffer_id;
	BufferId vertex_buffer_id;
	BufferId transform_buffer_id;
	BufferId bitset_buffer_id;
};

USE_PUSH_CONSTANT(Push)

DECL_BUFFER_STRUCT(
	CameraBuffer,
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

DECL_BUFFER_STRUCT(
	InfoBuffer,
	{
		f32 time;
		f32 delta_time;
		EntityInput entity_input;
	}
)
	
#ifdef compute

layout (local_size_x = 256) in;

#define HEIGHT 1.778
#define GRAVITY -9000

void main() {
	if(gl_GlobalInvocationID.x != 0) {
		return;
	}

	BufferRef(CameraBuffer) camera_buffer = buffer_id_to_ref(CameraBuffer, BufferRef, push_constant.camera_buffer_id);
	BufferRef(TransformBuffer) transform_buffer = buffer_id_to_ref(TransformBuffer, BufferRef, push_constant.transform_buffer_id);
	BufferRef(InfoBuffer) info_buffer = buffer_id_to_ref(InfoBuffer, BufferRef, push_constant.info_buffer_id);

	f32 delta_time = info_buffer.delta_time;

	Transform transform = transform_buffer.transform;
	EntityInput entity_input = info_buffer.entity_input;

	if(!transform.first){
		transform.position.xyz = vec3(32, 80, 32);
		transform.jumping = false;
		transform.first = true;
	}
	
	f32 sens = 1.0;

	transform.rotation.xy -= (entity_input.look.yx * delta_time) / sens;

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

#define SPEED 100

	direction.xz = lateral_direction;

	bool on_ground = false;
	vec3 adjusted_velocity = vec3(0);

	for(i32 x = -1; x <= 1; x++) {
			for(i32 z = -1; z <= 1; z++) {
				Ray ray;
				ray.bitset_buffer_id = push_constant.bitset_buffer_id;
				ray.origin = transform.position.xyz + vec3(f32(x) / 4, -HEIGHT, f32(z)/4);
				ray.direction = vec3(0, -1, 0);
				ray.max_distance = 1;

				RayHit ray_hit;

				bool success = ray_cast(ray, ray_hit);

				if(success) {
					on_ground = true;
					adjusted_velocity = normalize(transform.velocity.xyz) * min(length(transform.velocity.xyz), ray_hit.dist + EPSILON);
					break;
				}
			}
	}

	if(!transform.was_on_ground) {
		transform.velocity.y += GRAVITY * delta_time;
	}
	
	if(on_ground) {
		transform.velocity.y = max(adjusted_velocity.y, 0);
		transform.jumping = false;
		
		transform.was_on_ground = true;
	} else {
		transform.was_on_ground = false;
	}

	if(entity_input.up && on_ground && !transform.jumping) {
		transform.velocity.y += 100;
		transform.jumping = true;
	}
		
	transform.position.xyz += transform.velocity.xyz * delta_time;
	
	vec3 motion = SPEED * direction * delta_time;
	vec3 desired_motion = motion;

	bool colliding = false;

	for(i32 x = -1; x <= 1; x++) {
		for(i32 y = 0; y <= 5; y++) {
			for(i32 z = -1; z <= 1; z++) {
	Ray ray;
	ray.bitset_buffer_id = push_constant.bitset_buffer_id;
	ray.origin = transform.position.xyz + vec3(f32(x) / 4, -mix(HEIGHT - HEIGHT / 1.1, HEIGHT + HEIGHT / 1.1, f32(y) / 5), f32(z)/4);
	ray.direction = direction;
	ray.max_distance = 10;

	RayHit ray_hit;

	bool success = ray_cast(ray, ray_hit);

	if(success) {
		colliding = true;
		
		vec3 undesired_motion = ray_hit.normal * dot(desired_motion, ray_hit.normal);
		desired_motion -= undesired_motion;
	}

			}
		}
	}

	transform.was_colliding = colliding;

	if(length(desired_motion) > 0) {
		desired_motion = normalize(desired_motion) * length(motion); 
	}

	transform.position.xyz += desired_motion;

	transform_buffer.transform = transform;
}

#endif
