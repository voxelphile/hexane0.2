float vertex_ao(vec2 side, float corner) {
	return (side.x + side.y + max(corner, side.x * side.y)) / 3.0;
}

bool is_in_bounds(ivec3 position) {
	return all(greaterThan(position, ivec3(0))) && all(lessThan(position, ivec3(BLOCK_DETAIL)));
}

struct Ao {
	ImageId region_data;
	ivec3 pos;
	ivec3 d1;
	ivec3 d2;
};

vec4 voxel_ao(Ao ao) {
	VoxelQuery query;
	query.region_data = ao.region_data;

	vec4 side;

	query.position = ao.pos + ao.d1;
	side.x = float(voxel_query(query) && is_in_bounds(query.position)); 
	query.position = ao.pos + ao.d2;
	side.y = float(voxel_query(query) && is_in_bounds(query.position)); 
	query.position = ao.pos - ao.d1;
	side.z = float(voxel_query(query) && is_in_bounds(query.position)); 
	query.position = ao.pos - ao.d2;
	side.w = float(voxel_query(query) && is_in_bounds(query.position));

	vec4 corner;

	query.position = ao.pos + ao.d1 + ao.d2;
	corner.x = float(voxel_query(query) && is_in_bounds(query.position)); 
	query.position = ao.pos - ao.d1 + ao.d2;
	corner.y = float(voxel_query(query) && is_in_bounds(query.position)); 
	query.position = ao.pos - ao.d1 - ao.d2;
	corner.z = float(voxel_query(query) && is_in_bounds(query.position)); 
	query.position = ao.pos + ao.d1 - ao.d2;
	corner.w = float(voxel_query(query) && is_in_bounds(query.position));

	vec4 ret;
	ret.x = vertex_ao(side.xy, corner.x);
	ret.y = vertex_ao(side.yz, corner.y);
	ret.z = vertex_ao(side.zw, corner.z);
	ret.w = vertex_ao(side.wx, corner.w);
	return 1.0 - ret;
}


