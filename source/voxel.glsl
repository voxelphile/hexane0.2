struct VoxelQuery {
	//input
	BufferId world_id;
	f32vec3 position;
	//output
	u16 id;
};

bool voxel_query(inout VoxelQuery query) {
	Buffer(World) world = get_buffer(World, query.world_id);

	query.position = floor(query.position);
	
	i32vec3 chunk_position = i32vec3(query.position) / CHUNK_SIZE;
	i32vec3 internal_position = i32vec3(query.position) % CHUNK_SIZE;

	if(any(lessThanEqual(chunk_position, i32vec3(0)))) {
		return false;
	}

	if(any(greaterThanEqual(chunk_position, u32vec3(AXIS_MAX_CHUNKS)))) {
		return false;
	}

	Image(3D, u16) chunk_image = get_image(
		3D, 
		u16,
		world.chunks
			[chunk_position.x] 
			[chunk_position.y]
			[chunk_position.z]
	);

	query.id = u16(imageLoad(
		chunk_image, 
		internal_position
	).r);

	return query.id != 0;
}

struct VoxelChange {
	//input
	BufferId world_id;
	f32vec3 position;
	u16 id;
};

void voxel_change(inout VoxelChange change) {
	Buffer(World) world = get_buffer(World, change.world_id);
	
	change.position = floor(change.position);
	
	i32vec3 chunk_position = i32vec3(change.position) / CHUNK_SIZE;
	i32vec3 internal_position = i32vec3(change.position) % CHUNK_SIZE;
	
	if(any(lessThanEqual(chunk_position, i32vec3(0)))) {
		return;
	}
	
	if(any(greaterThanEqual(chunk_position, u32vec3(AXIS_MAX_CHUNKS)))) {
		return;
	}

	Image(3D, u16) chunk_image = get_image(
		3D, 
		u16,
		world.chunks
			[chunk_position.x] 
			[chunk_position.y]
			[chunk_position.z]
	);
	
	imageStore(
		chunk_image,
		internal_position,
		u32vec4(change.id)
	);
}
