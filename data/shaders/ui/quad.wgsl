//!use ui

struct Input {
	@location(0) position: vec2<f32>,
	@location(1) texcoord: vec2<f32>,
}

struct Output {
	@builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(in: Input) -> Output {
	var out: Output;
	out.clip_position = ui_view.proj * vec4<f32>(push_constants.position + in.position, 1.0);
	return out;
}

@group(1) @binding(0)
var in_texture: texture_2d<f32>;

@group(1) @binding(1)
var in_sampler: sampler;

@fragment
fn fs_main(in: Output) -> @location(0) vec4<f32> {
	return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}

