struct Vertex {
	vec4 position;
	i32vec4 normal;
	vec4 color;
	vec4 ambient;
};

decl_buffer(
	Vertices,
	{
		u32 count;
		Vertex data[15000000];
	}
)
