#define CHUNK_SIZE 64
#define AXIS_MAX_CHUNKS 4
#define MAX_CHUNKS 64

struct Chunk {
	vec3 minimum;
	vec3 maximum;
	ImageId data;
};

decl_buffer(
	Region,
	{
		Chunk chunks[MAX_CHUNKS];
		Chunk reserve[MAX_CHUNKS];
		ivec3 observer_position;
		bool dirty;
		bool first;
	}
)

