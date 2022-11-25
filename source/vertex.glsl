struct Vertex {
	vec4 position;
	i32vec4 normal;
	vec4 color;
	vec4 ambient;
};

decl_buffer(
	Vertices,
	{
		Vertex data[15000000];
		u32 count;
	}
)
