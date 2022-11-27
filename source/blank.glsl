#version 450

#include "hexane.glsl"

struct Push {
	BufferId info_id;
	BufferId camera_id;
	BufferId vertex_id;
	BufferId transform_id;
	BufferId world_id;
};

decl_push_constant(Push)
	
#ifdef vertex

layout(location = 0) out vec4 position;

void main() {
}

#elif defined fragment

layout(location = 0) in vec4 position;

layout(location = 0) out vec4 result;

void main() {
}

#endif
