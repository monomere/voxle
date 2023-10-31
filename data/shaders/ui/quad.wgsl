//!use ui

struct Input {
	@location(0) position: vec2<f32>,
	@location(1) texcoord: vec2<f32>,
}

struct Output {
	@builtin(position) clip_position: vec4<f32>,
	@location(0) texcoord: vec2<f32>
}

@vertex
fn vs_main(in: Input) -> Output {
	var out: Output;
	out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
	out.texcoord = in.texcoord;
	return out;
}

@group(0) @binding(1)
var in_texture: texture_2d<f32>;

@group(0) @binding(2)
var in_sampler: sampler;

@fragment
fn fs_main(in: Output) -> @location(0) vec4<f32> {
	return textureSample(in_texture, in_sampler, in.texcoord);
}

