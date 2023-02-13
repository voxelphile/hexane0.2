#version 450

#include "hexane.glsl"
#include "region.glsl"
#include "voxel.glsl"
#include "blocks.glsl"
#include "camera.glsl"
#include "raycast.glsl"
#include "transform.glsl"
#include "noise.glsl"
#include "rtx.glsl"
#include "luminosity.glsl"

#define VERTICES_PER_CUBE 6

struct RtxPush {
	BufferId info_id;
	BufferId camera_id;
	BufferId transform_id;
	BufferId region_id;
	BufferId mersenne_id;
	ImageId dir_id;
	ImageId pos_id;
	BufferId luminosity_id;
	BufferId entity_id;
};

decl_push_constant(RtxPush)

#ifdef fragment

layout(location = 0) out vec4 result;
void main() {
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Image(2D, f32) dir_img = get_image(2D, f32, push_constant.dir_id);
	Image(2D, f32) pos_img = get_image(2D, f32, push_constant.pos_id);

	Transform region_transform = transforms.data[0];
	ivec3 diff = region.floating_origin - region.observer_position;
	region_transform.position.xyz = vec3(REGION_SIZE / 2) - vec3(diff);
	region_transform.position.xyz += transforms.data[0].position.xyz - region.observer_position;
	
	vec2 screenPos = ((vec2(gl_FragCoord) / vec2(camera.resolution.xy))) * 2.0 - 1.0;
	vec4 far = camera.inv_projection * vec4(screenPos, 1, 1);
	far /= far.w;

	vec3 dir = (compute_transform_matrix(region_transform) * vec4(normalize(far.xyz), 0)).xyz;

	Path path;
	path.origin = region_transform.position.xyz;
	path.direction = dir;
	path.region_data = region.data;
	path.block_data = region.blocks;

	PathInfo info = path_trace(path);

	result = info.color;
}

#endif
