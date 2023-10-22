use wgpu::util::DeviceExt;
use crate::gfx::{self, graph};

use self::chunk::ChunkRenderContext;

pub mod chunk;

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
struct CameraUniform {
	view_proj: [[f32; 4]; 4],
}

pub struct GameRenderer {
	chunk_renderer: chunk::ChunkRenderer,
	uniform_buffer: wgpu::Buffer,
	uniform_bind_group: wgpu::BindGroup,
	_uniform_bind_group_layout: wgpu::BindGroupLayout,
	graph: graph::Graph<super::GameState>,
	pub camera: Camera,
}

impl GameRenderer {
	pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;

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
			zfar: 1000.0
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
					format: Self::DEPTH_FORMAT,
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
			chunk_renderer: chunk::ChunkRenderer::new(gfx, &uniform_bind_group_layout),
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

	fn render_main<'ctx>(&'ctx self, _gfx: &gfx::Gfx, render_pass: &mut wgpu::RenderPass<'ctx>, game: &'ctx super::GameState) {
		game.on_render(&mut GameRenderContext { renderer: self, render_pass });
	}
}

pub struct GameRenderContext<'a, 'b> {
	renderer: &'a GameRenderer,
	render_pass: &'b mut wgpu::RenderPass<'a>,
}

impl<'a, 'b> GameRenderContext<'a, 'b> {
	pub fn chunk_context<'ctx>(&'ctx mut self, mode: chunk::ChunkRenderMode) -> ChunkRenderContext<'a, 'ctx> {
		ChunkRenderContext::begin(mode, self.renderer, self.render_pass)
	}
}


// impl gfx::Renderer<GameRendererMode> for GameRenderer {
// 	fn on_use<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, mode: GameRendererMode) {
	// }
// }
