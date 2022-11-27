#version 450

#include "hexane.glsl"
#include "world.glsl"
#include "bits.glsl"
#include "voxel.glsl"

struct BuildBitsetPush {
	BufferId world_id;
	BufferId bitset_id;
};

decl_push_constant(BuildBitsetPush)

#ifdef compute

layout (local_size_x = 8, local_size_y = 8, local_size_z = 8) in;

void main() {
	VoxelQuery query;
	query.world_id = push_constant.world_id;
	query.position = f32vec3(gl_GlobalInvocationID);

	if(voxel_query(query)) {
		BitsetSet set;
		set.bitset_id = push_constant.bitset_id;
		set.position = query.position;

		bitset_set(set);
	}
}

#endif
