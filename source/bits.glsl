#define U32_bits 32

decl_buffer(
	Bitset,
	{
		u32 data[250000000];
	}
)

struct BitsetSet {
	BufferId bitset_id;
	f32vec3 position;
};

struct BitsetGet {
	BufferId bitset_id;
	f32vec3 position;
};

bool bitset_get(inout BitsetGet params) {
	Buffer(Bitset) bitset = get_buffer(Bitset, params.bitset_id);
	
	u32 axis_blocks = AXIS_MAX_CHUNKS * CHUNK_SIZE;

	u32vec3 pos = u32vec3(floor(params.position));
	
	u32 bit_index = u32(pos.x) + axis_blocks * (pos.y + axis_blocks * pos.z);

	return (bitset.data[bit_index / U32_bits] & (1 << bit_index % U32_bits)) != 0;
}

void bitset_set(in BitsetSet params) {
	Buffer(Bitset) bitset = get_buffer(Bitset, params.bitset_id);
	
	u32 axis_blocks = AXIS_MAX_CHUNKS * CHUNK_SIZE;

	u32vec3 pos = u32vec3(floor(params.position));

	u32 bit_index = u32(pos.x) + axis_blocks * (pos.y + axis_blocks * pos.z);

	atomicOr(bitset.data[bit_index / U32_bits], (1 << bit_index % U32_bits));
}
