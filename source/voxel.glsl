#define VOXEL_ID_VOID 0

struct VoxelQuery {
	//input
	ImageId region_data;
	ivec3 position;
	//output
	u16 id;
};

bool voxel_query(inout VoxelQuery query) {
	Image(3D, u16) region_data = get_image(
		3D, 
		u16,
		query.region_data
	);

	query.id = u16(imageLoad(
		region_data, 
		i32vec3(query.position)
	).r);
	
	if(any(lessThan(query.position, ivec3(0)))) {
		return false;
	}
	
	if(any(greaterThanEqual(query.position, imageSize(region_data)))) {
		return false;
	}

	return query.id != 0;
}

struct VoxelChange {
	//input
	ImageId region_data;
	ivec3 position;
	u16 id;
};

void voxel_change(inout VoxelChange change) {
	Image(3D, u16) region_data = get_image(
		3D, 
		u16,
		change.region_data
	);

	if(any(lessThan(change.position, ivec3(0)))) {
		return;
	}
	
	if(any(greaterThanEqual(change.position, imageSize(region_data)))) {
		return;
	}

	imageStore(
		region_data,
		i32vec3(change.position),
		u32vec4(change.id)
	);
}
