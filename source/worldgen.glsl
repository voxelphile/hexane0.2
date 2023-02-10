u16 world_gen(ivec3 world_position, BufferId region_id, ImageId perlin_id, ImageId worley_id) {
	Image(3D, u32) perlin_image = get_image(3D, u32, perlin_id);
	Image(3D, u32) worley_image = get_image(3D, u32, perlin_id);
	Buffer(Region) region = get_buffer(Region, region_id);
	
	u16 id = u16(BLOCK_ID_VOID);

	f32 height = 20;
	f32 water_height = 30;

	const int octaves = 8;
	float lacunarity = 2.0;
	float gain = 0.5;
	float amplitude = 100;
	float frequency = 0.1;
	for (int i = 0; i < octaves; i++) {
		f32 perlin_noise_factor = f32(imageLoad(perlin_image, abs(i32vec3(frequency * world_position.x, 32, frequency * world_position.z)) % i32vec3(imageSize(perlin_image))).r) / f32(~0u);
		height += amplitude * perlin_noise_factor;
		water_height += amplitude * 0.45;
		frequency *= lacunarity;
		amplitude *= gain;
	}


	f32 vertical_compression = 4;

	f32 worley_noise_factor = f32(imageLoad(worley_image, abs(i32vec3(world_position.x, world_position.y * vertical_compression, world_position.z)) % i32vec3(imageSize(worley_image))).r) / f32(~0u);

	f32 cave_frequency = 5e-3;
	vec3 cave_offset = vec3(100, 200, 300);
	f32 cave_smudge = 1e-7;
	f32 cave_noise_factor = f32(imageLoad(perlin_image, abs(i32vec3(vec3(world_position.x * cave_frequency, 32, world_position.z * cave_frequency) + cave_offset)) % i32vec3(imageSize(perlin_image))).r) / f32(~0u);

	//dunno why this is bugged.. if this statement isnt made like this
	//then grass spawns on chunk corners
	bool is_cave = false;
	if(false && worley_noise_factor > 1 && cave_noise_factor > 0.5 - cave_smudge && height < water_height) {
		id = u16(BLOCK_ID_AIR);
		is_cave = true;
	}

	if(id == u16(BLOCK_ID_VOID)) {
		if(world_position.y == i32(height)) {
			VoxelData data;
			for(int x = 0; x < BLOCK_DETAIL; x++) {
			for(int y = 0; y < BLOCK_DETAIL / 3; y++) {
			for(int z = 0; z < BLOCK_DETAIL; z++) {
				data.voxels[x][y][z] = u16(2);
			}
			}
			}

			id = block_hashtable_insert(region_id, data);
		} else if(world_position.y > height - 10 && world_position.y < height) {
			VoxelData data;
			for(int x = 0; x < BLOCK_DETAIL; x++) {
			for(int y = 0; y < BLOCK_DETAIL; y++) {
			for(int z = 0; z < BLOCK_DETAIL; z++) {
				data.voxels[x][y][z] = u16(4);
			}
			}
			}

			id = block_hashtable_insert(region_id, data);
		} else if(world_position.y < height) {
			VoxelData data;
			for(int x = 0; x < BLOCK_DETAIL; x++) {
			for(int y = 0; y < BLOCK_DETAIL; y++) {
			for(int z = 0; z < BLOCK_DETAIL; z++) {
				data.voxels[x][y][z] = u16(3);
			}
			}
			}

			id = block_hashtable_insert(region_id, data);
		} else {
			id = u16(BLOCK_ID_AIR);
		}
	}
	

	return id;
}

