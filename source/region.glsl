#define CHUNK_SIZE 64
#define AXIS_MAX_CHUNKS 4
#define REGION_SIZE 512
#define VIEW_DISTANCE 128
#define LOD 6

decl_buffer(
	Region,
	{
		ImageId data;
		ImageId reserve;
		ImageId block_entity;
		ImageId lod[LOD];
		ivec3 observer_position;
		ivec3 floating_origin;
		bool dirty;
		bool rebuild;
		bool first;
		i32 ray_count;
	}
)

