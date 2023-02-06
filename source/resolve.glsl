#version 450

#define EULER 2.71828
#define MAX_TRACE 16

#include "hexane.glsl"
#include "region.glsl"
#include "voxel.glsl"
#include "ao.glsl"
#include "camera.glsl"
#include "raycast.glsl"
#include "transform.glsl"
#include "noise.glsl"
#include "luminosity.glsl"
#include "rigidbody.glsl"

struct ResolvePush {
	ImageId prepass_id;
	ImageId history_id;
	ImageId resolve_id;
	ImageId dir_id;
	ImageId pos_id;
	ImageId history_dir_id;
	ImageId history_pos_id;
	BufferId rigidbody_id;
};

decl_push_constant(ResolvePush)

#ifdef compute

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

void main() {
	Image(2D, f32) prepass_image = get_image(2D, f32, push_constant.prepass_id);
	Image(2D, f32) history_image = get_image(2D, f32, push_constant.history_id);
	Image(2D, f32) resolve_image = get_image(2D, f32, push_constant.resolve_id);
	Image(2D, f32) dir_image = get_image(2D, f32, push_constant.dir_id);
	Image(2D, f32) pos_image = get_image(2D, f32, push_constant.pos_id);
	Image(2D, f32) history_dir_image = get_image(2D, f32, push_constant.history_dir_id);
	Image(2D, f32) history_pos_image = get_image(2D, f32, push_constant.history_pos_id);
	Buffer(Rigidbodies) rigidbodies = get_buffer(Rigidbodies, push_constant.rigidbody_id);

	i32vec2 size = i32vec2(imageSize(prepass_image));
	i32vec2 pos = i32vec2(gl_GlobalInvocationID.xy);
	
	if(any(greaterThanEqual(pos, size))) {
		return;
	}

	Rigidbody rigidbody = rigidbodies.data[0];

	vec4 current_color = imageLoad(prepass_image, pos);
	
	vec4 history_color = imageLoad(history_image, pos);

	vec4 dir = imageLoad(dir_image, pos);
	vec4 history_dir = imageLoad(history_dir_image, pos);

	vec4 color;

	if(dir == history_dir && all(lessThan(rigidbody.velocity, vec3(0.5))) && all(greaterThan(rigidbody.velocity, vec3(-0.5)))) {
		color = mix(current_color, history_color, 0.9); 
	} else {
		color = current_color;
	}
		
	imageStore(resolve_image, pos, color);
	imageStore(history_dir_image, pos, dir);
}

#endif

