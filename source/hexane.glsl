#extension GL_EXT_scalar_block_layout : require
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require
#extension GL_EXT_buffer_reference : require

#define b32 bool
#define i32 int
#define u32 uint
#define f32 float

#define b32vec2 bvec2
#define b32vec3 bvec3
#define b32vec4 bvec4
#define f32 float
#define f32vec2 vec2
#define f32mat2x2 mat2x2
#define f32mat2x3 mat2x3
#define f32mat2x4 mat2x4
#define f32vec3 vec3
#define f32mat3x2 mat3x2
#define f32mat3x3 mat3x3
#define f32mat3x4 mat3x4
#define f32vec4 vec4
#define f32mat4x2 mat4x2
#define f32mat4x3 mat4x3
#define f32mat4x4 mat4x4
#define i32 int
#define u32 uint
#define i64 int64_t
#define u64 uint64_t
#define i32vec2 ivec2
#define u32vec2 uvec2
#define i32vec3 ivec3
#define u32vec3 uvec3
#define i32vec4 ivec4
#define u32vec4 uvec4

#define DEVICE_ADDRESS_BUFFER_BINDING 4
#define STORAGE_BUFFER_BINDING 3

struct BufferId {
	u32 buffer_id_value;
};

layout(scalar, binding = DEVICE_ADDRESS_BUFFER_BINDING, set = 0) readonly buffer BufferDeviceAddressBuffer
{
    u64 addresses[];
} buffer_device_address_buffer;

#define DECL_BUFFER_STRUCT(NAME, BODY) 											\
	struct NAME BODY; 												\
	layout(scalar, binding = STORAGE_BUFFER_BINDING, set = 0) buffer BufferTableObject##NAME { 			\
		NAME value;												\
	}														\
	BufferTable##NAME [];												\
	layout(scalar, binding = STORAGE_BUFFER_BINDING, set = 0) buffer CoherentBufferTableBlock##NAME {		\
		NAME value;												\
	}														\
	CoherentBufferTable##NAME []; 											\
	layout(scalar, buffer_reference, buffer_reference_align = 4) buffer NAME##BufferRef BODY;			\
	layout(scalar, buffer_reference, buffer_reference_align = 4) coherent buffer NAME##CoherentBufferRef BODY;	\
	layout(scalar, buffer_reference, buffer_reference_align = 4) buffer NAME##WrappedBufferRef			\
    	{														\
        	NAME value; 												\
    	};														\
    	layout(scalar, buffer_reference, buffer_reference_align = 4) coherent buffer NAME##WrappedCoherentBufferRef	\
    	{														\
        	NAME value;												\
    	};

#define USE_PUSH_CONSTANT(NAME)												\
	layout(scalar, push_constant) uniform _PUSH_CONSTANT								\
	{														\
		NAME push_constant;											\
	};

#define BufferRef(STRUCT_TYPE) STRUCT_TYPE##BufferRef
#define WrappedBufferRef(STRUCT_TYPE) STRUCT_TYPE##WrappedBufferRef
#define CoherentBufferRef(STRUCT_TYPE) STRUCT_TYPE##CoherentBufferRef
#define WrappedCoherentBufferRef(STRUCT_TYPE) STRUCT_TYPE##WrappedCoherentBufferRef

#define buffer_ref_to_address(buffer_reference) u64(buffer_reference)
#define buffer_id_to_address(id) buffer_device_address_buffer.addresses[id.buffer_id_value]
#define buffer_address_to_ref(STRUCT_TYPE, REFERENCE_TYPE, address) STRUCT_TYPE##REFERENCE_TYPE(address)
#define buffer_id_to_ref(STRUCT_TYPE, REFERENCE_TYPE, id) buffer_address_to_ref(STRUCT_TYPE, REFERENCE_TYPE, buffer_id_to_address(id))
