#version 450

#include "hexane.glsl"
#include "region.glsl"
#include "transform.glsl"
#include "voxel.glsl"
#include "blocks.glsl"
#include "noise.glsl"
#include "worldgen.glsl"

struct BuildRegionPush {
	BufferId region_id;
	BufferId transform_id;
	ImageId perlin_id;
	ImageId worley_id;
	BufferId mersenne_id;
};

decl_push_constant(BuildRegionPush)

#ifdef compute

layout (local_size_x = 8, local_size_y = 8, local_size_z = 8) in;



void main() {
	Image(3D, u32) perlin_image = get_image(3D, u32, push_constant.perlin_id);
	Image(3D, u32) worley_image = get_image(3D, u32, push_constant.perlin_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);

	if(!region.dirty) 
	{
		return;
	}
	
	region.floating_origin = region.observer_position;
	
	ivec3 local_position = ivec3(gl_GlobalInvocationID);
	ivec3 world_position = region.floating_origin - ivec3(vec3(REGION_SIZE / 2)) + local_position;

	VoxelQuery query;
	query.region_data = region.reserve;
	query.position = local_position;

	if(voxel_query(query)) {
		return;
	}

	VoxelChange change;
	change.region_data = region.reserve;
	change.id = world_gen(world_position, push_constant.region_id, push_constant.perlin_id, push_constant.worley_id);
	change.position = local_position;
	
	voxel_change(change);
	

}

#endif

