use wgpu::{util::DeviceExt, ShaderStages};

use crate::gfx::{self, graph::{self, Graph}};

use super::{GameState, chunk::Block};

pub struct Camera {
	pub position: glm::Vec3,
	pub yaw: f32,
	pub pitch: f32,
	pub aspect: f32,
	pub fovy: f32,
	pub znear: f32,
	pub zfar: f32,
}

impl Camera {
	fn build_view_proj_matrix(&self) -> glm::Mat4 {
		let direction = glm::vec3(
			self.yaw.cos() * self.pitch.cos(),
			self.pitch.sin(),
			self.yaw.sin() * self.pitch.cos(),
		);

		let view = glm::look_at_rh(
			&self.position,
			&(self.position + &direction),
			&glm::vec3(0.0, 1.0, 0.0)
		);
		
		let proj = glm::perspective_rh_zo(self.aspect, glm::radians(&glm::vec1(self.fovy)).x, self.znear, self.zfar);

		proj * view
	}

	fn to_uniform(&self) -> CameraUniform {
		let view_proj = self.build_view_proj_matrix().data.0;
		CameraUniform { view_proj }
	}
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlockVertex {
	pub position: [f32; 3],
	pub _pad: u32,
	pub data: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
	view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct PushConstants {
	color: [f32; 4]
}

pub struct GameRenderer {
	render_pipeline: wgpu::RenderPipeline,
	wf_render_pipeline: wgpu::RenderPipeline,
	uniform_buffer: wgpu::Buffer,
	uniform_bind_group: wgpu::BindGroup,
	_uniform_bind_group_layout: wgpu::BindGroupLayout,
	graph: graph::Graph<super::GameState>,
	pub camera: Camera,
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
						offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
						shader_location: 1
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

impl GameRenderer {
	pub fn new(gfx: &gfx::Gfx) -> Self {
		let uniform_bind_group_layout = gfx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: None,
			entries: &[wgpu::BindGroupLayoutEntry {
				binding: 0,
				count: None,
				visibility: wgpu::ShaderStages::VERTEX,
				ty: wgpu::BindingType::Buffer {
					ty: wgpu::BufferBindingType::Uniform,
					has_dynamic_offset: false,
					min_binding_size: None
				},
			}]
		});
		
		let camera = Camera {
			position: glm::vec3(0.0, 0.5, -2.0),
			yaw: 3.0 * glm::quarter_pi::<f32>(),
			pitch: 0.0,
			aspect: gfx.config.width as f32 / gfx.config.height as f32,
			fovy: 60.0,
			znear: 0.01,
			zfar: 100.0
		};

		let uniform_buffer = Self::create_uniform_buffer(gfx, bytemuck::bytes_of(&camera.to_uniform()));

		let uniform_bind_group = gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: None,
			layout: &uniform_bind_group_layout,
			entries: &[wgpu::BindGroupEntry {
				binding: 0,
				resource: uniform_buffer.as_entire_binding()
			}]
		});

		let shader = gfx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: None,
			source: wgpu::ShaderSource::Wgsl(std::fs::read_to_string("src/shader_3d.wgsl").unwrap().into())
		});

		let layout = gfx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: None,
			bind_group_layouts: &[&uniform_bind_group_layout],
			push_constant_ranges: &[wgpu::PushConstantRange {
				range: 0..std::mem::size_of::<PushConstants>() as u32,
				stages: ShaderStages::FRAGMENT
			}]
		});

		const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;
		let render_pipeline = create_pipeline(gfx, &layout, &shader, wgpu::PolygonMode::Fill, DEPTH_FORMAT);
		let wf_render_pipeline = create_pipeline(gfx, &layout, &shader, wgpu::PolygonMode::Line, DEPTH_FORMAT);

		let graph_spec = graph::GraphSpec::<super::GameState> {
			attachments: &[
				("output", graph::AttachmentSpec::Output(graph::OutputAttachmentSpec {
					ops: |_: &gfx::Gfx| wgpu::Operations {
						load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
						store: true
					}
				})),
				("msaa-output", graph::AttachmentSpec::Color(graph::ColorAttachmentSpec {
					format: gfx.config.format,
					resolve: Some("output"),
					samples: 4,
					size: graph::AttachmentSizeSpec::Output(1.0),
					ops: |_: &gfx::Gfx| wgpu::Operations {
						load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.2, g: 0.3, b: 0.5, a: 1.0 }),
						store: false // we can discard, since the unresolved output isn't needed.
					},
				})),
				("depth", graph::AttachmentSpec::DepthStencil(graph::DepthStencilAttachmentSpec {
					format: DEPTH_FORMAT,
					depth_ops: Some(|_: &gfx::Gfx| Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: true })),
					stencil_ops: None,
					samples: 4 // TODO: allow user to sample count
				})),
			],
			nodes: &[
				graph::NodeSpec {
					id: "main",
					color_attachments: &["msaa-output"],
					depth_stencil_attachment: Some("depth"),
					render: |gfx, render_pass, game| {
						game.renderer.render_main(gfx, render_pass, game);
					}
				}
			],
		};
	
		Self {
			render_pipeline,
			wf_render_pipeline,
			uniform_buffer,
			uniform_bind_group,
			_uniform_bind_group_layout: uniform_bind_group_layout,
			graph: graph_spec.build(gfx),
			camera
		}
	}

	fn create_uniform_buffer(gfx: &gfx::Gfx, contents: &[u8]) -> wgpu::Buffer {
		gfx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: None,
			usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
			contents
		})
	}

	pub fn render(&self, ctx: &mut gfx::RenderContext, game: &super::GameState) {
		ctx.gfx.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&self.camera.to_uniform()));
		ctx.render_graph(&self.graph, game);
	}

	fn use_pipeline<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, mode: GameRendererMode) {
		render_pass.set_pipeline(match mode {
			GameRendererMode::Normal => &self.render_pipeline,
			GameRendererMode::Wireframe => &self.wf_render_pipeline,
		});

		let pushed = match mode {
			GameRendererMode::Normal => PushConstants { color: [1.0, 1.0, 1.0, 1.0] },
			GameRendererMode::Wireframe => PushConstants { color: [0.0, 0.0, 0.0, 1.0] },
		};

		render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
		render_pass.set_push_constants(ShaderStages::FRAGMENT, 0, bytemuck::bytes_of(&pushed));
	}

	fn render_main<'a>(&'a self, _gfx: &gfx::Gfx, render_pass: &mut wgpu::RenderPass<'a>, game: &'a super::GameState) {
		self.use_pipeline(render_pass, GameRendererMode::Normal);
		for (_, chunk) in &game.chunks {
			if let Some(mesh) = &chunk.mesh {
				mesh.render(render_pass);
			}
		}

		self.use_pipeline(render_pass, GameRendererMode::Wireframe);
		for (_, chunk) in &game.chunks {
			if let Some(mesh) = &chunk.mesh {
				mesh.render(render_pass);
			}
		}
	}
}

pub enum GameRendererMode {
	Normal,
	Wireframe
}

// impl gfx::Renderer<GameRendererMode> for GameRenderer {
// 	fn on_use<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, mode: GameRendererMode) {
	// }
// }
