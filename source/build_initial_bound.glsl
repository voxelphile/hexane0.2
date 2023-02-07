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
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Buffer(Bounding) bounding = get_buffer(Bounding, push_constant.bounding_id);

	i32 id = i32(gl_GlobalInvocationID.x);

	ivec3 block_start = ivec3(0, 0, BLOCK_DETAIL * id);
	ivec3 block_end = block_start + BLOCK_DETAIL;

	bool visited[BLOCK_DETAIL][BLOCK_DETAIL][BLOCK_DETAIL];

	ivec3 start = ivec3(0);

	bool in_bounds = true;
	do {
		ivec3 end = start;

		for(int i = 0; i < 3; i++) {
			int j = (i + 1) % 3;
			int k = (i + 2) % 3;

			bool proceed = true;
			
			do {
				for(int x = start[j]; x <= end[j] && proceed; x++) {
				for(int y = start[k]; y <= end[k] && proceed; y++) {
	
				VoxelQuery query;
				query.region_data = region.blocks;
				query.position = block_start + end;
		
				voxel_query(query);

				if(query.id != u16(2) || visited
					[query.position.x]
					[query.position.y]
					[query.position.z]
						== true) {
					proceed = false;
					break;
				} else {
					visited
						[query.position.x]
						[query.position.y]
						[query.position.z]
						= true;
				}
			}
			}
		
			proceed = proceed && end[i] < BLOCK_DETAIL - 1;

			if(proceed) {
				end[i]++;
			}	
			} while(proceed);
		}

		Box box;
		box.position = vec3(start);
		box.dimensions = vec3(end - start);

		i32 box_id = atomicAdd(bounding.bounds[id].box_count, 1);
		bounding.bounds[id].boxes[box_id] = box;

		bool voxel_found = false;

		do {
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
				break;
			}
			VoxelQuery query;
			query.region_data = region.blocks;
			query.position = start;

			voxel_query(query);

			voxel_found = query.id == 2;
		} while(!voxel_found);
	} while(in_bounds);
}

#endif

