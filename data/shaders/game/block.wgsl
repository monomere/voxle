//!use world

struct Input {
	@location(0) position: vec3<f32>,
	@location(1) data: u32,
	@location(2) texcoord: vec2<f32>,
	@builtin(vertex_index) vertex_index: u32,
}

struct Output {
	@builtin(position) clip_position: vec4<f32>,
	@location(1) normal: vec3<f32>,
	@location(2) color: vec3<f32>,
	@location(3) texcoord: vec2<f32>,
}

@vertex
fn vs_main(in: Input) -> Output {
	var out: Output;

	var faces: array<vec3<f32>, 6> = array<vec3<f32>, 6>(
		vec3<f32>( 1.0,  0.0,  0.0),
		vec3<f32>(-1.0,  0.0,  0.0),
		vec3<f32>( 0.0,  1.0,  0.0),
		vec3<f32>( 0.0, -1.0,  0.0),
		vec3<f32>( 0.0,  0.0,  1.0),
		vec3<f32>( 0.0,  0.0, -1.0),
	);

	out.normal = faces[in.data >> 16u];
	out.texcoord = in.texcoord;

	out.clip_position = world_camera.view_proj * vec4<f32>(in.position, 1.0);

	return out;
}

@group(1) @binding(0)
var in_texture: texture_2d<f32>;

@group(1) @binding(1)
var in_sampler: sampler;

struct PushConstants {
	color: vec4<f32>
}

var<push_constant> push_constants: PushConstants;

@fragment
fn fs_main(in: Output) -> @location(0) vec4<f32> {
	var shadow = clamp(dot(-world_lighting.sun_direction.xyz, in.normal), 0.1, 1.0);
	var color = textureSample(in_texture, in_sampler, in.texcoord);
	return push_constants.color * color * shadow;
}
