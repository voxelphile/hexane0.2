#version 450

#include "hexane.glsl"
#include "region.glsl"
#include "transform.glsl"
#include "voxel.glsl"

struct BuildStructPush {
	BufferId region_id;
	u32 lod;
};

decl_push_constant(BuildStructPush)

#ifdef compute

layout (local_size_x = 8, local_size_y = 8, local_size_z = 8) in;

void main() {
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);

	if(!region.rebuild) {
		return;	
	}

	ivec3 local_position = ivec3(gl_GlobalInvocationID);
	ivec3 lod_position = local_position * 2; 

	bool is_uniform = true;
		
	VoxelQuery query;
	i32 lod = i32(log2(push_constant.lod)) - 1;
	if(lod == 0) {
		query.region_data = region.data;
	} else { 
		query.region_data = region.lod[lod - 1]; 
	}
	query.position = lod_position;

	voxel_query(query);

	u16 id = query.id;

	for(int x = 0; x <= 1; x++) {
	for(int y = 0; y <= 1; y++) {
	for(int z = 0; z <= 1; z++) {
		query.position = lod_position + ivec3(x, y, z);
		voxel_query(query);

		if(id != query.id) {
			is_uniform = false;
			break;
		}
	}
	}
	}

	VoxelChange change;
	change.region_data = region.lod[lod];
	change.position = local_position;
	change.id = u16(is_uniform);

	voxel_change(change);
}

#endif

