struct VoxelQuery {
	//input
	ImageId chunk_id;
	f32vec3 position;
	//output
	u16 id;
};

bool voxel_query(inout VoxelQuery query) {
	Image(3D, u16) chunk_image = get_image(
		3D, 
		u16,
		query.chunk_id
	);

	query.id = u16(imageLoad(
		chunk_image, 
		i32vec3(query.position)
	).r);
	
	if(any(lessThan(query.position, vec3(0)))) {
		return false;
	}
	
	if(any(greaterThanEqual(query.position, vec3(CHUNK_SIZE)))) {
		return false;
	}

	return query.id != 0;
}

struct VoxelChange {
	//input
	ImageId chunk_id;
	f32vec3 position;
	u16 id;
};

void voxel_change(inout VoxelChange change) {
	Image(3D, u16) chunk_image = get_image(
		3D, 
		u16,
		change.chunk_id
	);

	if(any(lessThan(change.position, vec3(0)))) {
		return;
	}
	
	if(any(greaterThanEqual(change.position, vec3(CHUNK_SIZE)))) {
		return;
	}

	imageStore(
		chunk_image,
		i32vec3(change.position),
		u32vec4(change.id)
	);
}
