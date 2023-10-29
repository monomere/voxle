struct LightingUniform {
	sun_direction: vec4<f32>,
}

struct CameraUniform {
	view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> world_camera: CameraUniform;

@group(0) @binding(1)
var<uniform> world_lighting: LightingUniform;
