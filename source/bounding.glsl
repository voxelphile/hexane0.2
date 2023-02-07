#define MAX_BOXES 10

struct Bound {
	i32 box_count;
	Box boxes[MAX_BOXES];
};

decl_buffer(
	Bounding,
	{
		Bound bounds[MAX_BLOCKS];
	}
)
