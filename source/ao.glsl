float vertex_ao(vec2 side, float corner) {
	return (side.x + side.y + max(corner, side.x * side.y)) / 3.0;
}

struct AoQuery {
	ImageId region_data;
	ImageId block_data;
	ivec3 block_pos;
	ivec3 voxel_pos;	
	u16 block_id;
};

bool ao_query(AoQuery ao_query) {
	ao_query.block_pos += ao_query.voxel_pos / BLOCK_DETAIL; 
	ao_query.voxel_pos %= BLOCK_DETAIL;

	VoxelQuery block_query;
	block_query.region_data = ao_query.region_data;
	block_query.position = ao_query.block_pos;

	bool block_found = voxel_query(block_query);

	if(block_found && block_query.id != u16(1)) {
		VoxelQuery query;
		query.region_data = ao_query.block_data;
		query.position = ao_query.voxel_pos + ivec3(0, 0, block_query.id * BLOCK_DETAIL); 
		bool voxel_found = voxel_query(query);

		if(voxel_found) {
			return true;
		}
	}

	return false;
}

struct Ao {
	ImageId region_data;
	ImageId block_data;
	ivec3 block_pos;
	ivec3 voxel_pos;
	ivec3 d1;
	ivec3 d2;
	u16 block_id;
};

vec4 voxel_ao(Ao ao) {
	ao.block_pos += ao.voxel_pos / BLOCK_DETAIL; 
	ao.voxel_pos %= BLOCK_DETAIL;

	AoQuery query;
	query.region_data = ao.region_data;
	query.block_data = ao.block_data;
	query.block_pos = ao.block_pos;
	query.block_id = ao.block_id;

	vec4 side;

	query.voxel_pos = ao.voxel_pos + ao.d1;
	side.x = float(ao_query(query)); 
	query.voxel_pos = ao.voxel_pos + ao.d2;
	side.y = float(ao_query(query)); 
	query.voxel_pos = ao.voxel_pos - ao.d1;
	side.z = float(ao_query(query)); 
	query.voxel_pos = ao.voxel_pos - ao.d2;
	side.w = float(ao_query(query));

	vec4 corner;

	query.voxel_pos = ao.voxel_pos + ao.d1 + ao.d2;
	corner.x = float(ao_query(query)); 
	query.voxel_pos = ao.voxel_pos - ao.d1 + ao.d2;
	corner.y = float(ao_query(query)); 
	query.voxel_pos = ao.voxel_pos - ao.d1 - ao.d2;
	corner.z = float(ao_query(query)); 
	query.voxel_pos = ao.voxel_pos + ao.d1 - ao.d2;
	corner.w = float(ao_query(query));

	vec4 ret;
	ret.x = vertex_ao(side.xy, corner.x);
	ret.y = vertex_ao(side.yz, corner.y);
	ret.z = vertex_ao(side.zw, corner.z);
	ret.w = vertex_ao(side.wx, corner.w);
	return 1.0 - ret;
}


