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
	BufferId info_id;
	BufferId camera_id;
	BufferId transform_id;
	BufferId region_id;
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
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);

	i32vec2 size = i32vec2(imageSize(prepass_image));
	i32vec2 pos = i32vec2(gl_GlobalInvocationID.xy);
	
	if(any(greaterThanEqual(pos, size))) {
		return;
	}

	vec4 color = imageLoad(prepass_image, pos);

	imageStore(resolve_image, pos, vec4(color.rgb, 1));
}

#endif

