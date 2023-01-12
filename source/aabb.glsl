struct Box {
	vec3 position;
	vec3 dimensions;
	vec3 velocity;
};

bool aabb_check(Box a, Box b) {
	return !(a.position.x + a.dimensions.x < b.position.x 
		|| a.position.x > b.position.x + b.dimensions.x
		|| a.position.y + a.dimensions.y < b.position.y 
		|| a.position.y > b.position.y + b.dimensions.y
		|| a.position.z + a.dimensions.z < b.position.z 
		|| a.position.z > b.position.z + b.dimensions.z
	);
}
