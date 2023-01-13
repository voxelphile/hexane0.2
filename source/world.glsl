#define CHUNK_SIZE 16
#define AXIS_MAX_CHUNKS 8

struct Chunk {
	vec3 minimum;
	vec3 maximum;
	ImageId data;
};

decl_buffer(
	World,
	{
		Chunk chunks[1000];
	}
)

