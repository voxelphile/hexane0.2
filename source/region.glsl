#define CHUNK_SIZE 64
#define AXIS_MAX_CHUNKS 4
#define MAX_CHUNKS 64
#define REGION_SIZE 512
#define VIEW_DISTANCE 128

struct Chunk {
	uvec3 minimum;
	uvec3 maximum;
};

decl_buffer(
	Region,
	{
		ImageId data;
		ImageId reserve;
		Chunk chunks[MAX_CHUNKS];
		ivec3 observer_position;
		ivec3 floating_origin;
		bool dirty;
		bool first;
	}
)

