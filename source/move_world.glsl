#version 450

#include "hexane.glsl"
#include "region.glsl"
#include "transform.glsl"
#include "voxel.glsl"

struct BuildRegionPush {
	BufferId region_id;
	BufferId transform_id;
	ImageId perlin_id;
	ImageId worley_id;
};

decl_push_constant(BuildRegionPush)

#ifdef compute

layout (local_size_x = 8, local_size_y = 8, local_size_z = 8) in;

void main() {
	Image(3D, u32) perlin_image = get_image(3D, u32, push_constant.perlin_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	
	region.observer_position = ivec3(vec3(transforms.data[0].position.xyz));

	if(distance(region.floating_origin, region.observer_position) > VIEW_DISTANCE) {
		region.dirty = true;
		region.rebuild = true;
	} else{
		return;
	}
	
	ivec3 diff = region.floating_origin - region.observer_position;

	ivec3 from_position = ivec3(gl_GlobalInvocationID.xyz);
	ivec3 to_position = from_position + diff;

	if(any(lessThan(to_position, ivec3(0))) || any(greaterThanEqual(to_position, ivec3(REGION_SIZE)))) {
		return;	
	}

	VoxelQuery query;
	query.region_data = region.data;
	query.position = from_position;
	
	if(!voxel_query(query)) {
		return;
	}

	VoxelChange change;
	change.region_data = region.reserve;
	change.position = to_position;
	change.id = query.id;

	voxel_change(change);
	
	VoxelChange change2;
	change2.region_data = region.data;
	change2.position = from_position;
	change2.id = u16(0);

	voxel_change(change2);
}

#endif

