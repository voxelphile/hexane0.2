decl_buffer(
	MersenneTwister,
	{
		u32 indx;
		u32 mt[624 - 1];
	}
)

void seed(BufferId mersenne_id, u32 seed) {
	Buffer(MersenneTwister) mstw = get_buffer(MersenneTwister, mersenne_id);

	u32 w = 32;
	u32 n = 642;
	
	u32 f = 1812433253;
	
	mstw.indx = n;
	mstw.mt[0] = seed;

	for(u32 i = 1; i < n - 1; i++) {
		mstw.mt[i] = f * (mstw.mt[i - 1] ^ (mstw.mt[i - 1] >> (w - 2))) + i; 
	}
}

void twist(BufferId mersenne_id) {
	Buffer(MersenneTwister) mstw = get_buffer(MersenneTwister, mersenne_id);
	
	u32 n = 642;
	
	u32 m = 397;
	
	u32 a = 0x9908B0DF;

	for(u32 i = 0; i < n - 1; i++) {
		u32 x = (mstw.mt[i] & ~((1 << 31) - 1)) | (mstw.mt[(i + 1) % n] & ((1 << 31) - 1));
		u32 xA = x >> 1;
		if(x % 2 != 0) {
			xA ^= a;
		}
		mstw.mt[i] = mstw.mt[(i + m) % n] ^ xA;
	}

	mstw.indx = 0;
}

u32 random(BufferId mersenne_id) {
	Buffer(MersenneTwister) mstw = get_buffer(MersenneTwister, mersenne_id);
	
	u32 n = 624;

	u32 indx = atomicAdd(mstw.indx, 1);

	if(indx == n) {
			twist(mersenne_id);
	}

	u32 u = 11;
	u32 d = 0xFFFFFFFF;

	u32 s = 7;
	u32 b = 0x9D2C5680;

	u32 t = 15;
	u32 c = 0xEFC60000;
	
	u32 l = 18;	

	u32 y = mstw.mt[indx % n];
	y ^= (y >> u) & d;
	y ^= (y << s) & b;
	y ^= (y << t) & c;
	y ^= (y >> l);

	return y;
}
