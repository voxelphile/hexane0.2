#define AXIS_MAX_CHUNKS 6
#define CHUNK_SIZE 128

decl_buffer(
	World,
	{
		ImageId chunks[AXIS_MAX_CHUNKS][AXIS_MAX_CHUNKS][AXIS_MAX_CHUNKS];
	}
)

