struct ViewUniform {
	proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> ui_view: ViewUniform;
