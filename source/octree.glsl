/*struct Node {
	u32 id;
	u32 valid;
	u32 child[8];
};

decl_buffer(
	Octree,
	{
		u32 size;
		u32 len;
		Node nodes[100000000];
	}
)

struct OctreeQuery {
	//input
	BufferId octree_id;
	f32vec3 position;
	//output
	u32 node_index;
	u32 node_depth;
};

bool octree_query(inout OctreeQuery query) {
	Buffer(Octree) octree = get_buffer(Octree, query.octree_id);
	
	u32 size_cursor = u32(pow(2, octree.size));

	u32vec3 position_cursor = u32vec3(floor(query.position));
	
	for(query.node_index = 0, query.node_depth = 0; query.node_depth < octree.size; query.node_depth++) {
		size_cursor /= 2;

		u32vec3 compare = u32vec3(greaterThanEqual(position_cursor, u32vec3(size_cursor)));

		u32 octant = compare.x * 4 + compare.y * 2 + compare.z;

		u32 mask = 1 << octant;

		Node current_node = octree.nodes[query.node_index];

		if(current_node.valid != 0 && (current_node.valid & mask) == mask) {
			u32 child_offset = bitCount(current_node.valid & (mask - 1));
			query.node_index = current_node.child[octant];
		} else {
			break;
		}

		position_cursor -= compare * size_cursor;
	}

	return octree.nodes[query.node_index].valid == 0; 
}

struct OctreeBuild {
	//input
	BufferId octree_id;
	f32vec3 position;
	//output
	u32 node_index;
	u32 node_depth;
};

bool octree_build(inout OctreeBuild build) {
	Buffer(Octree) octree = get_buffer(Octree, build.octree_id);
	
	u32 size_cursor = u32(pow(2, octree.size));

	u32vec3 position_cursor = u32vec3(floor(build.position));
	
	for(build.node_index = 0, build.node_depth = 0; build.node_depth < octree.size; build.node_depth++) {
		size_cursor /= 2;

		u32vec3 compare = u32vec3(greaterThanEqual(position_cursor, u32vec3(size_cursor)));

		u32 octant = compare.x * 4 + compare.y * 2 + compare.z;

		u32 mask = 1 << octant;

		Node current_node = octree.nodes[build.node_index];

		memoryBarrier();

		if((current_node.valid & mask) == mask) {
			u32 child_offset = bitCount(current_node.valid & (mask - 1));
			build.node_index = current_node.child[octant];
		} else {	
		memoryBarrier();
			atomicOr(octree.nodes[build.node_index].valid, mask);
		memoryBarrier();
			u32 child = atomicAdd(octree.len, 1);
		memoryBarrier();
			u32 prev = atomicCompSwap(octree.nodes[build.node_index].child[octant], current_node.child[octant], child);

		memoryBarrier();
			if(prev == current_node.child[octant]) {	
				build.node_index = child;
			} else {
				build.node_index = octree.nodes[build.node_index].child[octant];
			}
		}

		position_cursor -= compare * size_cursor;
	}

	return octree.nodes[build.node_index].valid == 0; 
}
*/
