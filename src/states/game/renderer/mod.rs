use wgpu::util::DeviceExt;
use crate::{gfx::{self, graph}, math::{self, Vector, vec3}};

use self::chunk::ChunkRenderContext;

pub mod chunk;


pub struct GameRenderer {
	pub chunk_renderer: chunk::ChunkRenderer,
	// uniform_buffer: wgpu::Buffer,
	graph: graph::Graph<super::GameState>,
}

impl GameRenderer {
	pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;

	pub fn new(gfx: &gfx::Gfx) -> Self {
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
			chunk_renderer: chunk::ChunkRenderer::new(gfx),
			graph: graph_spec.build(gfx),
		}
	}

	pub fn render(&self, ctx: &mut gfx::RenderContext, game: &super::GameState) {
		ctx.render_graph(&self.graph, game);
	}

	fn render_main<'ctx>(&'ctx self, gfx: &gfx::Gfx, render_pass: &mut wgpu::RenderPass<'ctx>, game: &'ctx super::GameState) {
		game.on_render(gfx, &mut GameRenderContext { renderer: self, render_pass });
	}

	pub fn update(&mut self) {
		self.chunk_renderer.update()
	}
}

pub struct GameRenderContext<'a, 'b> {
	renderer: &'a GameRenderer,
	render_pass: &'b mut wgpu::RenderPass<'a>,
}

impl<'a, 'b> GameRenderContext<'a, 'b> {
	pub fn chunk_context<'ctx>(&'ctx mut self, gfx: &gfx::Gfx, mode: chunk::ChunkRenderMode) -> ChunkRenderContext<'a, 'ctx> {
		ChunkRenderContext::begin(gfx, mode, self.renderer, self.render_pass)
	}
}
