//!use world

struct Input {
	@location(0) data0: u32, // uv:2 z:10 y:10 x:10
	@location(1) data1: u32, // tid:24 ao:8
	@builtin(vertex_index) vertex_index: u32,
}

struct Output {
	@builtin(position) pos: vec4f,
	@location(1) norm: vec3f,
	@location(2) col: vec3f,
	@location(3) tex: vec2f,
	@location(4) tid: u32,
	@location(5) @interpolate(flat) qao: vec4f,
}

fn unpack(in: Input, out: ptr<function, Output>) -> vec3f {

	var aos = array<f32, 4>(
		0.75,
		0.75,
		0.75,
		1.0
	);

	// xyzw, 2 bits each.
	(*out).qao.x = aos[(in.data1 >> 0u) & 3u]; // 2 bits
	(*out).qao.y = aos[(in.data1 >> 2u) & 3u];
	(*out).qao.z = aos[(in.data1 >> 4u) & 3u];
	(*out).qao.w = aos[(in.data1 >> 6u) & 3u];

	var uvs = array<vec2f, 4>(
		vec2f(0.0, 1.0),
		vec2f(1.0, 1.0),
		vec2f(1.0, 0.0),
		vec2f(0.0, 0.0),
	);

	// 2^24 values, an 8K screen has just 2x more pixels, should be enough.
	(*out).tid = in.data1 >> 8u;

	(*out).tex = uvs[in.data0 >> 30u]; // 2 bits

	return vec3f(
		f32(extractBits(i32(in.data0 >>  0u), 0u, 10u)) * 0.5,
		f32(extractBits(i32(in.data0 >> 10u), 0u, 10u)) * 0.5,
		f32(extractBits(i32(in.data0 >> 20u), 0u, 10u)) * 0.5,
	);
}

struct VertPushConsts {
	chunk_pos: vec3i
}

var<push_constant> pushed: VertPushConsts;

@vertex
fn vs_main(in: Input) -> Output {
	var out: Output;

	let pos = unpack(in, &out);
	out.pos = world_camera.view_proj * vec4f(vec3f(pushed.chunk_pos) + pos, 1.0);

	return out;
}



@group(1) @binding(0)
var in_tex: texture_2d_array<f32>;

@group(1) @binding(1)
var in_samp: sampler;

const is_black: bool = /*!const(is_black)*/;

@fragment
fn fs_main(in: Output) -> @location(0) vec4f {
	if is_black {
		return vec4f(0.0, 0.0, 0.0, 1.0);
	} else {
		var col = textureSample(in_tex, in_samp, in.tex, in.tid);

		let ao0 = mix(in.qao.x, in.qao.y, in.tex.x);
		let ao1 = mix(in.qao.z, in.qao.w, in.tex.x);
		let ao = mix(ao1, ao0, in.tex.y);
		
		return col * ao;
	}
}
