#define BUILD_BITSET_OCTREE_REGION_COUNT_MAX 8	
#define U32_bits 32

decl_buffer(
	Bitset,
	{
		u32 len;
		u32 data[100000000];
	}
)

struct BitsetQuery {
	BufferId bitset_id;
	u32 size;
	u32 bit_index;		
};

struct OctreeBitsetQuery {
	BufferId bitset_id;
	u32 size;
	f32vec3 position;
};

bool query_bitset(inout BitsetQuery query) {
	Buffer(Bitset) bitset = get_buffer(Bitset, query.bitset_id);
	
	if(bitset.len <= query.bit_index) {
		return false;
	}

	return (bitset.data[query.bit_index / U32_bits] & (1 << query.bit_index % U32_bits)) != 0;
}

bool query_octree_bitset(inout OctreeBitsetQuery query) {
	
	u32 size_cursor = u32(pow(2, query.size));
	
	u32vec3 position_cursor = u32vec3(floor(query.position));
	
	u32 bit_index = 0; 

	for(u32 depth = 0; depth < query.size; depth++) {
		size_cursor /= 2;

		u32vec3 compare = u32vec3(greaterThanEqual(position_cursor, u32vec3(size_cursor)));

		u32 octant = compare.x * 4 + compare.y * 2 + compare.z;
		
		bit_index += u32(pow(8, depth)) + u32(pow(8, query.size - depth - 1)) * octant;
	
		position_cursor -= compare * size_cursor;
	}

	BitsetQuery subquery;

	subquery.bitset_id = query.bitset_id;
	subquery.bit_index = bit_index;

	return query_bitset(subquery);
}

/*
bool set_octree_bit(in SetOctreeBit params) {
	if(params.region_count > BUILD_BITSET_OCTREE_REGION_COUNT_MAX) {
		return false;	
	}	

	u32 size_cursor = u32(pow(2, params.octree.size));

	u32vec3 position_cursor = u32vec3(floor(params.position));

	uint bit_index = 0;

	uint node_index = 0;
	uint node_depth = 0;
	
	for(; node_depth < query.src.size; node_depth++) {
		size_cursor /= 2;

		u32vec3 compare = u32vec3(greaterThanEqual(position_cursor, u32vec3(size_cursor)));

		u32 octant = compare.x * 4 + compare.y * 2 + compare.z;

		Node current_node = query.octree.nodes[query.node_index];

		if(current_node.valid != 0 && current_node.valid & mask == mask) {
			u32 child_offset = bitCount(current_node.valid & (mask - 1));
			node_index = current_node.child + child_offset;

			bit_index += pow(8, node_depth) + pow(8, node_depth) * octant;
		} else {
			break;
		}

		position_cursor -= compare * size_cursor;
	}

	query.dst.len = max(query.dst.len, bit_index);

	query.dst.data[bit_index / U32_bits] |= 1 << bit_index % U32_bits;
}*/
