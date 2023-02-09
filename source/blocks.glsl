#define HASH_START 2166136261

uint tumble_fnv(uint hash, uint data) {
	hash ^= data;
	hash = hash * 0x01000193;
	return hash;
}

struct VoxelData {
	u16 voxels[BLOCK_DETAIL][BLOCK_DETAIL][BLOCK_DETAIL];
};

uint hashfn(uint x) {
    x = ((x >> 16) ^ x) * 0x45d9f3b;
    x = ((x >> 16) ^ x) * 0x45d9f3b;
    x = (x >> 16) ^ x;
    return x;
}

uint voxel_hash(in VoxelData data) {
	uint hash = HASH_START;

	for(int x = 0; x < BLOCK_DETAIL; x++) {
	for(int y = 0; y < BLOCK_DETAIL; y++) {
	for(int z = 0; z < BLOCK_DETAIL; z++) {
		hash = hashfn(hash + u32(data.voxels[x][y][z])); 
	}
	}
	}

	return hash;
}

VoxelData voxel_data(BufferId region_id, uint slot) {
	Buffer(Region) region = get_buffer(Region, region_id);
	Image(3D, u16) block_data = get_image(3D, u16, region.blocks);

	VoxelData data;
	for(int x = 0; x < BLOCK_DETAIL; x++) {
	for(int y = 0; y < BLOCK_DETAIL; y++) {
	for(int z = 0; z < BLOCK_DETAIL; z++) {
		data.voxels[x][y][z] = u16(imageLoad(block_data, ivec3(x,y,z) + ivec3(0, 0, slot * BLOCK_DETAIL)).r);  
	}
	}
	}

	return data;
}

u16 block_hashtable_insert(
	BufferId region_id,
	in VoxelData data
) {
	Buffer(Region) region = get_buffer(Region, region_id);
	Image(3D, u16) block_data = get_image(3D, u16, region.blocks);
	
	uint hash = voxel_hash(data);
	uint slot = hash & (MAX_BLOCKS - 1);

		while(slot <= 1) {
			slot = (slot + 1) & (MAX_BLOCKS - 1);
		}
	while(true) {
		uint prev = atomicCompSwap(region.hash_table[slot].hash, 0, hash);
		if(prev == 0 || prev == hash) {
			for(int x = 0; x < BLOCK_DETAIL; x++) {
			for(int y = 0; y < BLOCK_DETAIL; y++) {
			for(int z = 0; z < BLOCK_DETAIL; z++) {
				ivec3 pos = ivec3(x, y, z) + ivec3(0, 0, slot * BLOCK_DETAIL);

				imageStore(block_data, pos, u32vec4(data.voxels[x][y][z]));
			}
			}
			}
			break;
		}
			slot = (slot + 1) & (MAX_BLOCKS - 1);
		while(slot <= 1) {
			slot = (slot + 1) & (MAX_BLOCKS - 1);
		}
	}

	return u16(slot);
}

VoxelData block_hashtable_delete(
	BufferId region_id,
	u16 slot
) {
	VoxelData data = voxel_data(region_id, slot);

	Buffer(Region) region = get_buffer(Region, region_id);
	Image(3D, u16) block_data = get_image(3D, u16, region.blocks);

	for(int x = 0; x < BLOCK_DETAIL; x++) {
	for(int y = 0; y < BLOCK_DETAIL; y++) {
	for(int z = 0; z < BLOCK_DETAIL; z++) {
		imageStore(block_data, ivec3(x,y,z) + ivec3(0, 0, slot * BLOCK_DETAIL), u32vec4(0));  
	}
	}
	}

	region.hash_table[slot].hash = 0;

	return data;
}
