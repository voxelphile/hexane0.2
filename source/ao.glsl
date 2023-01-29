float vertex_ao(vec2 side, float corner) {
	return (side.x + side.y + max(corner, side.x * side.y)) / 3.0;
}

vec4 voxel_ao(ImageId region_data, ivec3 pos, ivec3 d1, ivec3 d2) {
	VoxelQuery query;
	query.region_data = region_data;

	vec4 side;

	query.position = pos + d1;
	side.x = float(voxel_query(query) && is_solid(query.id)); 
	query.position = pos + d2;
	side.y = float(voxel_query(query) && is_solid(query.id)); 
	query.position = pos - d1;
	side.z = float(voxel_query(query) && is_solid(query.id)); 
	query.position = pos - d2;
	side.w = float(voxel_query(query) && is_solid(query.id));

	vec4 corner;

	query.position = pos + d1 + d2;
	corner.x = float(voxel_query(query) && is_solid(query.id)); 
	query.position = pos - d1 + d2;
	corner.y = float(voxel_query(query) && is_solid(query.id)); 
	query.position = pos - d1 - d2;
	corner.z = float(voxel_query(query) && is_solid(query.id)); 
	query.position = pos + d1 - d2;
	corner.w = float(voxel_query(query) && is_solid(query.id));

	vec4 ao;
	ao.x = vertex_ao(side.xy, corner.x);
	ao.y = vertex_ao(side.yz, corner.y);
	ao.z = vertex_ao(side.zw, corner.z);
	ao.w = vertex_ao(side.wx, corner.w);
	return 1.0 - ao;
}
