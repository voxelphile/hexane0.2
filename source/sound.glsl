#define MAX_SOUNDS 100
#define GRASS_SOUND 1
#define GRASS_SOUND_COUNT 10
#define DIRT_SOUND 11
#define DIRT_SOUND_COUNT 11
#define STONE_SOUND 22
#define STONE_SOUND_COUNT 11

decl_buffer(
	Sound,
	{
		u32 sound_len;
		u32 sounds[MAX_SOUNDS];
	}
)

void play_sound(BufferId sound_id, u32 id) {
	Buffer(Sound) sound = get_buffer(Sound, sound_id);

	sound.sounds[sound.sound_len] = id;
	sound.sound_len += 1;
}

void play_sound_for_block_id(BufferId sound_id, BufferId mersenne_id, u32 id) {
	if(id == 2) {
		u32 add = u32(f32(random(mersenne_id)) / f32(~0u) * f32(GRASS_SOUND_COUNT - 1));
		play_sound(sound_id, GRASS_SOUND + add);
	}
	if(id == 3) {
		u32 add = u32(f32(random(mersenne_id)) / f32(~0u) * f32(STONE_SOUND_COUNT - 1));
		play_sound(sound_id, STONE_SOUND + add);
	}
	if(id == 4) {
		u32 add = u32(f32(random(mersenne_id)) / f32(~0u) * f32(DIRT_SOUND_COUNT - 1));
		play_sound(sound_id, DIRT_SOUND + add);
	}

}
