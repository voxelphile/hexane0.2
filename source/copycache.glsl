#version 450

#include "hexane.glsl"
#include "world.glsl"
#include "vertex.glsl"
#include "transform.glsl"
#include "bits.glsl"
#include "voxel.glsl"
#include "raycast.glsl"

#define VERTICES_PER_CUBE 6

struct CachePush {
	BufferId camera_id;
	BufferId transform_id;
	BufferId cache_id;
	ImageId cache_pos_image;
	ImageId cache_color_image;
	ImageId write_cache_pos_image;
	ImageId write_cache_color_image;
};

decl_push_constant(CachePush)

decl_buffer(
	Camera,
	{
		mat4 projection;
		vec2 resolution;
	}
)

decl_buffer(
	Cache,
	{
		Transform last;
	}
)
	
#ifdef compute

layout (local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

void main() {
	Image(2D, f32) cache_color_image = get_image(
		2D, 
		f32,
		push_constant.cache_color_image
	);
	Image(2D, f32) cache_pos_image = get_image(
		2D, 
		f32,
		push_constant.cache_pos_image
	);
	Image(2D, f32) cache_color_image2 = get_image(
		2D, 
		f32,
		push_constant.write_cache_color_image
	);
	Image(2D, f32) cache_pos_image2 = get_image(
		2D, 
		f32,
		push_constant.write_cache_pos_image
	);
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Cache) cache = get_buffer(Cache, push_constant.cache_id);
	
	Transform transform = transforms.data[0];
	
	vec4 color = vec4(imageLoad(cache_color_image2, i32vec2(gl_GlobalInvocationID.xy)).rgba);
	vec4 pos = vec4(imageLoad(cache_pos_image2, i32vec2(gl_GlobalInvocationID.xy)).rgba);

	imageStore(
		cache_color_image, 
		i32vec2(gl_GlobalInvocationID.xy),
		vec4(color)
	);
	imageStore(
		cache_pos_image, 
		i32vec2(gl_GlobalInvocationID.xy),
		vec4(pos)
	);
}

#endif
