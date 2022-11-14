#pragma once

#include "hexane.glsl"

struct Node {
	u32 child;
	u32 valid;
	u64 morton;
}

struct Octree {
	u32 size;
	u32 len;
	Node nodes[1000000];
}

struct OctreeQuery {
	//input
	Octree octree;
	f32vec3 position;
	//output
	u32 node_index;
	u32 node_depth;
}

bool octree_query(inout OctreeQuery query) {
	u32 size_cursor = u32(pow(2, query.octree.size));

	u32vec3 position_cursor = u32vec3(floor(query.position));
	
	for(query.node_index = 0, query.node_depth = 0; query.node_depth < query.octree.size; query.node_depth++) {
		size_cursor /= 2;

		u32vec3 compare = u32vec3(greaterThanEqual(position_cursor, u32vec3(size_cursor)));

		u32 octant = compare.x * 4 + compare.y * 2 + compare.z;

		u32 mask = 1 << octant;

		Node current_node = query.octree.nodes[query.node_index];

		if(current_node.valid != 0 && current_node.valid & mask == mask) {
			u32 child_offset = bitCount(current_node.valid & (mask - 1));
			node_index = current_node.child + child_offset;
		} else {
			break;
		}

		position_cursor -= compare * size_cursor;
	}

	//TODO this might not be right
	return query.octree.nodes[query.node_index].valid == 0; 
}

