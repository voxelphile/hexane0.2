#version 450

#include "hexane.glsl"
#include "rigidbody.glsl"
#include "transform.glsl"

struct PhysicsPush {
	BufferId transform_id;
	BufferId rigidbody_id;
};

decl_push_constant(PhysicsPush)

#ifdef compute

layout (local_size_x = 256) in;

void main() {
	if(gl_GlobalInvocationID.x != 0) {
		return;
	}
}

#endif
