struct Block {
	vec3 out_position;
	ImageId out_region;
};
struct Actor {
	i32 num;
};

decl_buffer(
	Entities,
	{
		Block blocks[1000];
		Actor actors[1000];
	}
)
