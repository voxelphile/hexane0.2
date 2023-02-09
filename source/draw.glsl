#version 450

#include "hexane.glsl"
#include "region.glsl"
#include "voxel.glsl"
#include "blocks.glsl"
#include "camera.glsl"
#include "raycast.glsl"
#include "transform.glsl"
#include "noise.glsl"
#include "rtx.glsl"
#include "luminosity.glsl"

#define VERTICES_PER_CUBE 6

struct RtxPush {
	BufferId info_id;
	BufferId camera_id;
	BufferId transform_id;
	BufferId region_id;
	BufferId mersenne_id;
	ImageId perlin_id;
	ImageId prepass_id;
	ImageId dir_id;
	ImageId pos_id;
	BufferId luminosity_id;
	BufferId entity_id;
};

decl_push_constant(RtxPush)

#ifdef fragment

layout(location = 0) out vec4 result;
void main() {
	Buffer(Camera) camera = get_buffer(Camera, push_constant.camera_id);
	Buffer(Transforms) transforms = get_buffer(Transforms, push_constant.transform_id);
	Buffer(Region) region = get_buffer(Region, push_constant.region_id);
	Image(3D, u32) perlin_img = get_image(3D, u32, push_constant.perlin_id);
	Image(2D, f32) dir_img = get_image(2D, f32, push_constant.dir_id);

	Transform region_transform = transforms.data[0];
	ivec3 diff = region.floating_origin - region.observer_position;
	region_transform.position.xyz = vec3(REGION_SIZE / 2) - vec3(diff);
	region_transform.position.xyz += transforms.data[0].position.xyz - region.observer_position;
	
	vec2 screenPos = (1 - (vec2(gl_FragCoord) / vec2(camera.resolution.xy))) * 2.0 - 1.0;
	vec4 far = camera.inv_projection * vec4(screenPos, 1, 1);
	far /= far.w;

	vec3 dir = (compute_transform_matrix(region_transform) * vec4(normalize(far.xyz), 0)).xyz;

	Buffer(Luminosity) luminosity = get_buffer(Luminosity, push_constant.luminosity_id);
	f32 c_pi = 3.1415;
	f32 DOFApertureRadius = 0.04;
	//TODO fix this
	f32 DOFFocalLength = 10;

	vec3 fwdVector = dir;
        vec3 rightVector = (vec4(1, 0, 0, 0) * inverse(compute_transform_matrix(region_transform))).xyz;
        vec3 upVector = (vec4(0, 1, 0, 0) * inverse(compute_transform_matrix(region_transform))).xyz;
        vec3 cameraForward = (vec4(0, 0, 1, 0) * inverse(compute_transform_matrix(region_transform))).xyz;

	float f01 = f32(random(push_constant.mersenne_id)) / f32(~0u);
	float f02 = f32(random(push_constant.mersenne_id)) / f32(~0u);
	float angle = f01 * 2.0f * c_pi;
        float radius = sqrt(f02);
        vec2 off = vec2(cos(angle), sin(angle)) * radius * DOFApertureRadius;
        float shapeArea = c_pi * DOFApertureRadius * DOFApertureRadius;
	f32 lightMultiplier = shapeArea; 
	vec3 sensorPos;
	vec4 sensorPlane = vec4(cameraForward, 0);
	vec3 cameraSensorPlanePoint = region_transform.position.xyz - fwdVector;
	sensorPlane.w = -(sensorPlane.x * cameraSensorPlanePoint.x + sensorPlane.y * cameraSensorPlanePoint.y + sensorPlane.z * cameraSensorPlanePoint.z);
        {
        	float t = -(dot( region_transform.position.xyz, sensorPlane.xyz) + sensorPlane.w) / dot(dir, sensorPlane.xyz);
		sensorPos = region_transform.position.xyz + dir * t;

        	vec3 cameraSpacePos = (vec4(sensorPos, 1.0f) * inverse(compute_transform_matrix(region_transform))).xyz;

		cameraSpacePos.z *= DOFFocalLength;

		sensorPos = (vec4(cameraSpacePos, 1.0f) * compute_transform_matrix(region_transform)).xyz;
	}

	vec3 aperturePos = region_transform.position.xyz + rightVector * off.x + upVector.xyz * off.y;

	vec3 cameraPos = region_transform.position.xyz;

	vec3 cameraFocalPlanePoint = cameraPos + vec3(cameraForward) * DOFFocalLength;

        vec4 focalPlane = vec4(-cameraForward, 0);

        focalPlane.w = -(focalPlane.x * cameraFocalPlanePoint.x + focalPlane.y * cameraFocalPlanePoint.y + focalPlane.z * cameraFocalPlanePoint.z);


	vec3 rstart = cameraPos;
        vec3 rdir = -dir;
	float t = -(dot(rstart, focalPlane.xyz) + focalPlane.w) / dot(rdir, focalPlane.xyz);
        vec3 focusPos = rstart + rdir * t;


	Path path;
	path.origin = aperturePos.xyz;
	path.direction = normalize(focusPos - aperturePos);
	path.region_data = region.data;
	path.block_data = region.blocks;

	imageStore(dir_img, ivec2(gl_FragCoord.xy), vec4(-dir, 0));

	result = vec4(path_trace(path), 1);
}

#endif
