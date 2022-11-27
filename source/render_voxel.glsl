#version 450

#include "hexane.glsl"
#include "world.glsl"
#include "vertex.glsl"
#include "transform.glsl"
#include "bits.glsl"
#include "voxel.glsl"
#include "raycast.glsl"

#define VERTICES_PER_CUBE 6

struct DrawPush {
	BufferId info_id;
	BufferId camera_id;
	BufferId vertex_id;
	BufferId transform_id;
	BufferId world_id;
};

decl_push_constant(DrawPush)
	
#ifdef vertex

vec3 offsets[8] = vec3[](
        vec3(0, 0, 1),
        vec3(0, 1, 1),
        vec3(1, 1, 1),
        vec3(1, 0, 1),
        vec3(0, 0, 0),
        vec3(0, 1, 0),
        vec3(1, 1, 0),
	vec3(1, 0, 0)
);

decl_buffer(
	Camera,
	{
		mat4 projection;
	}
)

layout(location = 0) out vec4 position;
layout(location = 1) out vec4 eye_position;
layout(location = 2) flat out i32vec4 normal;
layout(location = 3) flat out vec4 color;
layout(location = 4) flat out vec4 ambient;
layout(location = 5) out vec4 uv;

void main() {
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Vertices) verts = get_buffer(Vertices, push_constant.vertex_id);

	u32 i = gl_VertexIndex / VERTICES_PER_CUBE;
	u32 j = gl_VertexIndex % VERTICES_PER_CUBE;
	
	position = verts.data[i].position;
	normal = verts.data[i].normal;
	color = verts.data[i].color;
	ambient = verts.data[i].ambient;
	
	vec2 uvs[6] = vec2[](
		vec2(0, 0),
		vec2(0, 1),
		vec2(1, 1),
		vec2(0, 0),
		vec2(1, 1),
		vec2(1, 0)
	);

	if(normal.xyz == i32vec3(0, 0, 1)) {
		u32 i[6] = u32[](1, 0, 3, 1, 3, 2);
	
		position.xyz += offsets[i[j]];	
		uv.xy = vec2(1 - uvs[j].y, uvs[j].x);
	}
	
	if(normal.xyz == i32vec3(0, 0, -1)) {
		u32 i[6] = u32[](4, 5, 6, 4, 6, 7);
		
		position.xyz += offsets[i[j]];	
		uv.xy = uvs[j].yx;
	}
	
	if(normal.xyz == i32vec3(1, 0, 0)) {
		u32 i[6] = u32[](2, 3, 7, 2, 7, 6);
		
		position.xyz += offsets[i[j]];	
		uv.xy = 1 - uvs[j].xy;
	}
	
	if(normal.xyz == i32vec3(-1, 0, 0)) {
		u32 i[6] = u32[](5, 4, 0, 5, 0, 1);
		
		position.xyz += offsets[i[j]];
		uv.xy = vec2(uvs[j].x, 1 - uvs[j].y);
	}
	
	if(normal.xyz == i32vec3(0, 1, 0)) {
		u32 i[6] = u32[](6, 5, 1, 6, 1, 2);
		
		position.xyz += offsets[i[j]];	
		uv.xy = vec2(1 - uvs[j].y, uvs[j].x);
	}
	
	if(normal.xyz == i32vec3(0, -1, 0)) {
		u32 i[6] = u32[](3, 0, 4, 3, 4, 7);
		
		position.xyz += offsets[i[j]];	
		//TODO
		uv.xy = uvs[j].yx;
	}
	
	Transform transform = transforms.transform;
			
	eye_position = inverse(compute_transform_matrix(transform)) * position;
	gl_Position =  camera.projection * eye_position;
}

#elif defined fragment

#define SHOW_UV false
#define SHOW_RGB_STRIATION false
#define SHOW_NORMALS false
#define SHOW_AO true
#define SHOW_FOG true
#define SHOW_DOF true

layout(location = 0) in vec4 position;
layout(location = 1) in vec4 eye_position;
layout(location = 2) flat in i32vec4 normal;
layout(location = 3) flat in vec4 color;
layout(location = 4) flat in vec4 ambient;
layout(location = 5) in vec4 uv;

layout(location = 0) out vec4 result;

void main() {
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
    	
	result = color;
		
	float dist = abs(eye_position.z / eye_position.w);
	
	float ao = 0;

	ao = mix(mix(ambient.z, ambient.w, uv.y), mix(ambient.y, ambient.x, uv.y), uv.x);

	vec3 sun_position = vec3(1000, 2000, 100);
	
	Ray ray;
	ray.world_id = push_constant.world_id;
	ray.origin = position.xyz + normal.xyz * 1e-3;
	ray.direction = normalize(sun_position - ray.origin);
	ray.max_distance = 2;

	RayHit ray_hit;

	bool success = ray_cast(ray, ray_hit);
	
	if(SHOW_UV) {
		result = vec4(0, 0, 0, 1);
		result.xy = uv.xy;
	}

	if(SHOW_RGB_STRIATION) {
#define STRIATE 8
		result.xyz = mod(position.xyz, STRIATE) / STRIATE;
	}
	
	if(SHOW_NORMALS) {
		result.xyz = normal.xyz;
		if(normal.x < -1 + EPSILON || normal.y < -1 + EPSILON || normal.z < -1 + EPSILON ){
			result.xyz = 1 - result.xyz;
		}
	}
	
	if(SHOW_AO) {
		result.xyz = result.xyz - vec3(1 - ao) * 0.25;
	}

	if(success) {
		result.xyz *= 0.5;
	}

	if(SHOW_FOG) {
		vec4 fog_color = vec4(0.1, 0.4, 0.8, 1.0); 
		
		float fog_density = 0.005;

		float fog_factor = exp(-pow(fog_density * dist, 4.0));

		fog_factor = 1.0 - clamp(fog_factor, 0.0, 1.0);

		result = mix(result, fog_color, fog_factor);
	}
}

#endif
