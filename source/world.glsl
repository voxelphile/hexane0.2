#define AXIS_MAX_CHUNKS 8
#define CHUNK_SIZE 64

decl_buffer(
	World,
	{
		ImageId chunks[AXIS_MAX_CHUNKS][AXIS_MAX_CHUNKS][AXIS_MAX_CHUNKS];
	}
)

