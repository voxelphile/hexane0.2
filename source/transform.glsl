struct Transform {
	vec4 position;
	vec4 rotation;
};

decl_buffer(
	Transforms,
	{
		Transform data[1000];
		bool physics;
	}
)

mat4 compute_transform_matrix(inout Transform transform) {
	vec3 position = transform.position.xyz;
	vec3 rotation = transform.rotation.xyz;

	return mat4(
		cos(rotation.y) * cos(rotation.z),
		cos(rotation.y) * sin(rotation.z),
		-sin(rotation.y),
		0,
		sin(rotation.x) * sin(rotation.y) * cos(rotation.z) - cos(rotation.x) * sin(rotation.z),
		sin(rotation.x) * sin(rotation.y) * sin(rotation.z) + cos(rotation.x) * cos(rotation.z),
		sin(rotation.x) * cos(rotation.y),
		0,
		cos(rotation.x) * sin(rotation.y) * cos(rotation.z) + sin(rotation.x) * sin(rotation.z),
		cos(rotation.x) * sin(rotation.y) * sin(rotation.z) - sin(rotation.x) * cos(rotation.z),
		cos(rotation.x) * cos(rotation.y),
		0,
		position.xyz,
		1
	);
}

