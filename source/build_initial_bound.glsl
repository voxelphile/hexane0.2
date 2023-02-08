#version 450

#include "hexane.glsl"
#include "region.glsl"
#include "aabb.glsl"
#include "voxel.glsl"
#include "bounding.glsl"

struct BuildRegionPush {
	BufferId region_id;
	BufferId bounding_id;
};

decl_push_constant(BuildRegionPush)

#ifdef compute

layout (local_size_x = 1) in;

void main() {
	return;
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Buffer(Bounding) bounding = get_buffer(Bounding, push_constant.bounding_id);

	i32 id = i32(gl_GlobalInvocationID.x);

	ivec3 block_start = ivec3(0, 0, BLOCK_DETAIL * id);
	ivec3 block_end = block_start + BLOCK_DETAIL;

	bool visited[BLOCK_DETAIL][BLOCK_DETAIL][BLOCK_DETAIL];

	//set all visited to false
	for(int x = 0; x < BLOCK_DETAIL; x++) {
	for(int y = 0; y < BLOCK_DETAIL; y++) {
	for(int z = 0; z < BLOCK_DETAIL; z++) {
		visited[x][y][z] = false;		
	}
	}
	}

	ivec3 start = ivec3(0);

	bool in_bounds = true;
	do {
		//find the first block that is solid and has not been visited
		while(in_bounds) {
			VoxelQuery query;
			query.region_data = region.blocks;
			query.position = block_start + start;

			voxel_query(query);

			if(query.id == u16(2) && visited
					[start.x]
					[start.y]
					[start.z]
						== false) {
				break;
			}
			start[0]++;
			if(start[0] >= BLOCK_DETAIL) {
				start[0] = 0;
				start[1]++;
			}
			if(start[1] >= BLOCK_DETAIL) {
				start[1] = 0;
				start[2]++;
			}
			if(start[2] >= BLOCK_DETAIL) {
				in_bounds = false;
			}
		};

		//if in bounds, continue
		if(in_bounds) {
		ivec3 end = start;

		//for every dimension, walk along that dimension and "consume" solid blocks that have not been visited.
		//if a block is not solid, go to the next dimension
		for(int i = 0; i < 3; i++) {
			int j = (i + 1) % 3;
			int k = (i + 2) % 3;

			bool proceed = true;
			
			while(proceed) {
				for(int x = start[j]; x <= end[j] && proceed; x++) {
				for(int y = start[k]; y <= end[k] && proceed; y++) {
	
				ivec3 probe = end;
				probe[j] = x;
				probe[k] = y;
				VoxelQuery query;
				query.region_data = region.blocks;
				query.position = block_start + probe;
		
				voxel_query(query);

				if(query.id != u16(2) || visited
					[probe.x]
					[probe.y]
					[probe.z]
						== true) {
					proceed = false;
					break;
				}
				}
				}
			
				proceed = proceed && end[i] < BLOCK_DETAIL - 1;
				
				if(!proceed) {
					continue;
				}

				ivec3 probe = end;
				probe[i]++;

				VoxelQuery query;
				query.region_data = region.blocks;
				query.position = block_start + probe;

				voxel_query(query);

				proceed = proceed && query.id == 2;

				if(proceed) {
					end = probe;
				}
			}
		}

		//mark all blocks between start and end as visited
		for(int x = start.x; x <= end.x; x++) {
		for(int y = start.y; y <= end.y; y++) {
		for(int z = start.z; z <= end.z; z++) {
			visited[x][y][z] = true;		
		}
		}
		}

		//create an AABB out of the data
		Box box;
		box.position = vec3(start);
		box.dimensions = vec3((end - start) + 1);

		//add the AABB to the "database" for this block
		i32 box_id = bounding.bounds[id].box_count;
		bounding.bounds[id].box_count++;
		bounding.bounds[id].boxes[box_id] = box;
		}
	} while(in_bounds);
}

#endif

