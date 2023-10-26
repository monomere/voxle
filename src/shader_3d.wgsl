struct CameraUniform {
	view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
	@location(0) position: vec3<f32>,
	@location(1) data: u32,
	@location(2) texcoord: vec2<f32>,
	@builtin(vertex_index) vertex_index: u32,
}

struct VertexOutput {
	@builtin(position) clip_position: vec4<f32>,
	@location(1) normal: vec3<f32>,
	@location(2) color: vec3<f32>,
	@location(3) texcoord: vec2<f32>,
}

fn rand1(xx: f32) -> f32 {
	let x0: f32 = floor(xx);
	let x1: f32 = x0 + 1.0;
	let v0: f32 = fract(sin(x0 * 0.014686) * 31718.927 + x0);
	let v1: f32 = fract(sin(x1 * 0.014686) * 31718.927 + x1);

	return fract((v0 * (1.0 - fract(xx)) + v1 * (fract(xx))) * 2.0 - sin(xx));
}

fn rand_color(x: f32) -> vec3<f32> {
	return vec3<f32>(rand1(x), rand1(x + 12.3), rand1(x - 89.2));
}

@vertex
fn vs_main(
	in: VertexInput,
) -> VertexOutput {
	var out: VertexOutput;

	var faces: array<vec3<f32>, 6> = array<vec3<f32>, 6>(
		vec3<f32>( 1.0,  0.0,  0.0),
		vec3<f32>(-1.0,  0.0,  0.0),
		vec3<f32>( 0.0,  1.0,  0.0),
		vec3<f32>( 0.0, -1.0,  0.0),
		vec3<f32>( 0.0,  0.0,  1.0),
		vec3<f32>( 0.0,  0.0, -1.0),
	);

	var block_id = in.data & 0xFFu;

	// var face_vertex_index = in.vertex_index % 6u;

	out.normal = faces[in.data >> 16u];
	out.texcoord = in.texcoord;

	// if block_id < 4u {
	// 	out.color = colors[block_id];
	// } else {
	// 	out.color = rand_color(f32(block_id) * 123.321);
	// }

	out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
	return out;
}


struct PushConstants {
	color: vec4<f32>
}

var<push_constant> pushed: PushConstants;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	// var sun_dir = normalize(vec3<f32>(2.0, 4.0, 1.0));
	// var shadow = clamp(dot(sun_dir, in.normal), 0.1, 1.0);
	
	return pushed.color * vec4<f32>(in.texcoord, 1.0, 1.0);// * shadow;
}
