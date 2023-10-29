//!use world

struct PushConstants {
	position: vec3<f32>
}

var<push_constant> push_constants: PushConstants;

struct Input {
	@location(0) position: vec3<f32>,
}

struct Output {
	@builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(in: Input) -> Output {
	var out: Output;
	out.clip_position = world_camera.view_proj * vec4<f32>(push_constants.position + in.position, 1.0);
	return out;
}

@fragment
fn fs_main(in: Output) -> @location(0) vec4<f32> {
	return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
