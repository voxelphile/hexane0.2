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
	u32 bit_index;		
};

struct HierarchyBitsetQuery {
	BufferId bitset_id;
	f32vec3 position;
};

struct SetHierarchyBit {
	BufferId bitset_id;
	f32vec3 position;
};

bool query_bitset(inout BitsetQuery query) {
	Buffer(Bitset) bitset = get_buffer(Bitset, query.bitset_id);
	
	if(bitset.len <= query.bit_index) {
		return true;
	}

	return (bitset.data[query.bit_index / U32_bits] & (1 << query.bit_index % U32_bits)) != 0;
}

bool query_hierarchical_bitset(inout HierarchyBitsetQuery query) {
	u32 axis_blocks = AXIS_MAX_CHUNKS * CHUNK_SIZE;

	u32 axis_magnitude = u32(ceil(log2(f32(axis_blocks))));
	
	u32 size_cursor = u32(axis_blocks);
	
	u32vec3 position_cursor = u32vec3(floor(query.position));
	
	u32 bit_index = 0; 

	for(u32 depth = 0; depth <= axis_magnitude; depth++) {
		size_cursor /= 2;

		u32vec3 compare = u32vec3(greaterThanEqual(position_cursor, u32vec3(size_cursor)));

		u32 octant = compare.x * 4 + compare.y * 2 + compare.z;
		
		bit_index += u32(pow(8, depth)) + u32(pow(8, axis_magnitude - depth - 1)) * octant;
	
		BitsetQuery subquery;

		subquery.bitset_id = query.bitset_id;
		subquery.bit_index = bit_index;
	
		if(!query_bitset(subquery)) {
			return false;
		}

		position_cursor -= compare * size_cursor;
	}

	return true;
}

void set_hierarchy_bit(in SetHierarchyBit params) {
	Buffer(Bitset) bitset = get_buffer(Bitset, params.bitset_id);
	atomicExchange(bitset.len, 1000000000);
	
	u32 axis_blocks = AXIS_MAX_CHUNKS * CHUNK_SIZE;

	u32 axis_magnitude = u32(ceil(log2(f32(axis_blocks))));

	u32 size_cursor = u32(axis_blocks);
	
	u32vec3 position_cursor = u32vec3(floor(params.position));
	
	u32 bit_index = 0;

	for(u32 depth = 0; depth <= axis_magnitude; depth++) {
		size_cursor /= 2;

		u32vec3 compare = u32vec3(greaterThanEqual(position_cursor, u32vec3(size_cursor)));

		u32 octant = compare.x * 4 + compare.y * 2 + compare.z;
		
		bit_index += u32(pow(8, depth)) + u32(pow(8, axis_magnitude - depth - 1)) * octant;
		atomicOr(bitset.data[bit_index / U32_bits], 1 << bit_index % U32_bits);
	
		position_cursor -= compare * size_cursor;
	}


}
