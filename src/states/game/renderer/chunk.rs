use crate::gfx;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlockVertex {
	pub position: [f32; 3],
	pub data: u32,
	pub texcoord: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ChunkPushConstants {
	color: [f32; 4]
}

fn create_pipeline(
	gfx: &gfx::Gfx,
	layout: &wgpu::PipelineLayout,
	shader: &wgpu::ShaderModule,
	polymode: wgpu::PolygonMode,
	depth_format: wgpu::TextureFormat
) -> wgpu::RenderPipeline {
	gfx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: None,
		layout: Some(&layout),
		vertex: wgpu::VertexState {
			module: &shader,
			entry_point: "vs_main",
			buffers: &[wgpu::VertexBufferLayout {
				array_stride: std::mem::size_of::<BlockVertex>() as wgpu::BufferAddress,
				step_mode: wgpu::VertexStepMode::Vertex,
				attributes: &[
					// position
					wgpu::VertexAttribute {
						format: wgpu::VertexFormat::Float32x3,
						offset: 0,
						shader_location: 0
					},
					// data
					wgpu::VertexAttribute {
						format: wgpu::VertexFormat::Uint32,
						offset: 3 * 4,
						shader_location: 1
					},
					// texcoord
					wgpu::VertexAttribute {
						format: wgpu::VertexFormat::Float32x2,
						offset: 4 * 4,
						shader_location: 2
					}
				],
			}]
		},
		fragment: Some(wgpu::FragmentState {
			module: &shader,
			entry_point: "fs_main",
			targets: &[
				Some(wgpu::ColorTargetState {
					format: gfx.config.format,
					blend: None,
					write_mask: wgpu::ColorWrites::ALL
				})
			]
		}),
		primitive: wgpu::PrimitiveState {
			topology: wgpu::PrimitiveTopology::TriangleList,
			strip_index_format: None,
			front_face: wgpu::FrontFace::Cw,
			cull_mode: None, // TODO: fix face vertex ordering in states::game::chunk
			unclipped_depth: false,
			polygon_mode: polymode,
			conservative: false
		},
		depth_stencil: Some(wgpu::DepthStencilState {
			format: depth_format,
			depth_write_enabled: match polymode {
				wgpu::PolygonMode::Line => false,
				_ => true,
			},
			depth_compare: match polymode {
				wgpu::PolygonMode::Line => wgpu::CompareFunction::LessEqual,
				_ => wgpu::CompareFunction::Less,
			},
			stencil: wgpu::StencilState::default(),
			bias: wgpu::DepthBiasState::default(),
		}),
		multisample: wgpu::MultisampleState {
			count: 4,
			mask: !0,
			alpha_to_coverage_enabled: false
		},
		multiview: None
	})
}

pub(super) struct ChunkRenderer {
	render_pipeline: wgpu::RenderPipeline,
	wf_render_pipeline: wgpu::RenderPipeline,
}

impl ChunkRenderer {
	pub fn new(gfx: &gfx::Gfx, uniform_layout: &wgpu::BindGroupLayout) -> Self {
		let shader = gfx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: None,
			source: wgpu::ShaderSource::Wgsl(std::fs::read_to_string("src/shader_3d.wgsl").unwrap().into())
		});

		let layout = gfx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: None,
			bind_group_layouts: &[uniform_layout],
			push_constant_ranges: &[wgpu::PushConstantRange {
				range: 0..std::mem::size_of::<ChunkPushConstants>() as u32,
				stages: wgpu::ShaderStages::FRAGMENT
			}]
		});

		let render_pipeline = create_pipeline(gfx, &layout, &shader, wgpu::PolygonMode::Fill, super::GameRenderer::DEPTH_FORMAT);
		let wf_render_pipeline = create_pipeline(gfx, &layout, &shader, wgpu::PolygonMode::Line, super::GameRenderer::DEPTH_FORMAT);

		Self {
			render_pipeline,
			wf_render_pipeline
		}
	}
}

pub enum ChunkRenderMode {
	Normal,
	Wireframe
}

pub struct ChunkRenderContext<'a, 'b> {
	pub(super) renderer: &'a super::GameRenderer,
	pub(super) render_pass: &'b mut wgpu::RenderPass<'a>
}

impl<'a, 'b> ChunkRenderContext<'a, 'b> {
	pub(super) fn begin(
		mode: ChunkRenderMode,
		renderer: &'a super::GameRenderer,
		render_pass: &'b mut wgpu::RenderPass<'a>
	) -> ChunkRenderContext<'a, 'b> {
		let mut ctx = ChunkRenderContext {
			renderer,
			render_pass
		};
		ctx.set_mode(mode);
		ctx
	}
	
	fn set_mode(&mut self, mode: ChunkRenderMode) {
		self.render_pass.set_pipeline(match mode {
			ChunkRenderMode::Normal => &self.renderer.chunk_renderer.render_pipeline,
			ChunkRenderMode::Wireframe => &self.renderer.chunk_renderer.wf_render_pipeline,
		});

		let pushed = match mode {
			ChunkRenderMode::Normal => ChunkPushConstants { color: [1.0, 1.0, 1.0, 1.0] },
			ChunkRenderMode::Wireframe => ChunkPushConstants { color: [0.0, 0.0, 0.0, 1.0] },
		};

		self.render_pass.set_bind_group(0, &self.renderer.uniform_bind_group, &[]);
		self.render_pass.set_push_constants(wgpu::ShaderStages::FRAGMENT, 0, bytemuck::bytes_of(&pushed));
	}

	pub fn render_chunk(&mut self, chunk: &'a super::super::chunk::Chunk) {
		if let Some(mesh) = &chunk.mesh {
			mesh.render(self.render_pass)
		}
	}
}

