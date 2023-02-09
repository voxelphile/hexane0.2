#version 450

#include "hexane.glsl"
#include "region.glsl"
#include "blocks.glsl"
#include "transform.glsl"
#include "voxel.glsl"
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

layout (local_size_x = 1) in;



void main() {
	Image(3D, u32) perlin_image = get_image(3D, u32, push_constant.perlin_id);
	Image(3D, u32) worley_image = get_image(3D, u32, push_constant.perlin_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);

	Image(3D, u16) block_data = get_image(3D, u16, region.blocks);

	VoxelData data;
	for(int x = 0; x < BLOCK_DETAIL; x++) {
	for(int y = 0; y < BLOCK_DETAIL / 2; y++) {
	for(int z = 0; z < BLOCK_DETAIL; z++) {
		data.voxels[x][y][z] = u16(2);
	}
	}
	}

	region.rando_id = block_hashtable_insert(push_constant.region_id, data);
}

#endif

