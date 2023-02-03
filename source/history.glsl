#version 450
//Credit to Gabe Rundlett, original source from gvox engine

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

struct HistoryPush {
	ImageId resolve_id;
	ImageId history_id;
};

decl_push_constant(HistoryPush)

#ifdef compute

layout (local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

void main() {
	Image(2D, f32) resolve_image = get_image(2D, f32, push_constant.resolve_id);
	Image(2D, f32) history_image = get_image(2D, f32, push_constant.history_id);

	i32vec2 size = i32vec2(imageSize(resolve_image));
	i32vec2 pos = i32vec2(gl_GlobalInvocationID.xy);
	
	if(any(greaterThanEqual(pos, size))) {
		return;
	}
	

	imageStore(history_image, pos, imageLoad(resolve_image, pos));
}

#endif

