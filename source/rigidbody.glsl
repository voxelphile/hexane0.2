struct Rigidbody {
	bool on_ground;
	bool hit_something;
	bool colliding;
	vec3 velocity;
	vec3 acceleration;
	f32 mass;
};

decl_buffer(
	Rigidbodies,
	{
		Rigidbody data[1000];
	}
)
