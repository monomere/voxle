//!use world

struct Input {
	@location(0) position: vec3<f32>,
	@location(1) data: u32,
	@builtin(vertex_index) vertex_index: u32,
}

struct Output {
	@builtin(position) clip_position: vec4<f32>,
	@location(1) normal: vec3<f32>,
	@location(2) color: vec3<f32>,
	@location(3) texcoord: vec2<f32>,
	@location(4) texture_id: u32,
}

@vertex
fn vs_main(in: Input) -> Output {
	var out: Output;

	// matches enum crate::game::Dir
	var normals = array<vec3<f32>, 6>(
		vec3<f32>( 1.0,  0.0,  0.0),
		vec3<f32>(-1.0,  0.0,  0.0),
		vec3<f32>( 0.0,  1.0,  0.0),
		vec3<f32>( 0.0, -1.0,  0.0),
		vec3<f32>( 0.0,  0.0,  1.0),
		vec3<f32>( 0.0,  0.0, -1.0),
	);

	var uvs = array<vec2<f32>, 4>(
		vec2<f32>(0.0, 1.0),
		vec2<f32>(1.0, 1.0),
		vec2<f32>(1.0, 0.0),
		vec2<f32>(0.0, 0.0),
	);

	// DATA
	// normal: 3 bits
	// uv: 2 bits
	// texid: 32 - 3 - 2 = 27 bits
	// an 8K screen has about 2^25 pixels.
	// 4x more texture ids than visible pixels on an 8K screen.
	// just to be sure.

	let normal_idx = (in.data >> 0u) & 7u; // 7 = 111b
	let uv_idx = (in.data >> 3u) & 3u; // 3 = 11b
	let tex_id = in.data >> 5u; // the rest

	out.normal = normals[normal_idx];
	out.texcoord = uvs[uv_idx];
	out.texture_id = tex_id;

	out.clip_position = world_camera.view_proj * vec4<f32>(in.position, 1.0);

	return out;
}

@group(1) @binding(0)
var in_texture: texture_2d_array<f32>;

@group(1) @binding(1)
var in_sampler: sampler;

struct PushConstants {
	color: vec4<f32>
}

var<push_constant> push_constants: PushConstants;

@fragment
fn fs_main(in: Output) -> @location(0) vec4<f32> {
	var shadow = clamp(dot(-world_lighting.sun_direction.xyz, in.normal), 0.1, 1.0);
	var color = textureSample(in_texture, in_sampler, in.texcoord, in.texture_id);
	// var color = vec4<f32>(in.texcoord, 1.0, 1.0);
	return push_constants.color * color * shadow;
}
