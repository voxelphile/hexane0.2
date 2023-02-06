#define CHUNK_SIZE 64
#define AXIS_MAX_CHUNKS 4
#define REGION_SIZE 512
#define VIEW_DISTANCE 128
#define LOD 3
#define BLOCK_DETAIL 8

decl_buffer(
	Region,
	{
		ImageId data;
		ImageId reserve;
		ImageId blocks;
		ImageId lod[LOD];
		ivec3 observer_position;
		ivec3 floating_origin;
		bool dirty;
		bool rebuild;
		bool first;
		bool block_set;
		i32 ray_count;
	}
)

