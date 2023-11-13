use crate::gfx::{self, graph};
use self::{chunk::ChunkRenderContext, ui::{UiRenderContext, UiBuilder}};
use lazy_static::lazy_static;

pub mod chunk;
pub mod ui;

pub fn load_shader_module<F: Fn(&str) -> &str>(name: &str, get_const: Option<&F>) -> Result<String, std::io::Error> {
	use std::io::Read;

	let base_path = std::path::PathBuf::from("data/shaders");
	let module_path = base_path.join(name).with_extension("wgsl");
	if !module_path.is_file() {
		panic!("Shader not found: {:?}", module_path);
	}

	let mut module_source = String::new();
	std::io::BufReader::new(std::fs::File::open(&module_path)?).read_to_string(&mut module_source)?;
	let mut module_string = String::new();

	let first_line = module_source.lines().next().unwrap();
	if first_line.starts_with("//!use") {
		for include in first_line.split_whitespace().skip(1) {
			module_string.push_str(&*load_shader_module(include, get_const).unwrap());
		}
	}

	module_string.push_str(&module_source);
	
	if let Some(get_const) = get_const.as_ref() {
	lazy_static! {
			static ref RE: regex::Regex = regex::Regex::new(
				r"/\*!const\(([\w_]+)\)\*/"
			).unwrap();
		}

		let mut offset: usize = 0;
		loop {
			if let Some(caps) = RE.captures_at(&module_string, offset) {
				offset += caps.get(0).unwrap().len();
				let range = caps.get(0).unwrap().range();
				let c = get_const(caps.get(1).unwrap().as_str());
				module_string.replace_range(range, &c.to_owned());
			} else {
				break;
			}
		}
	}

	Ok(module_string)
}

pub fn load_shader(name: &str) -> Result<wgpu::ShaderModuleDescriptor, std::io::Error>  {
	let shader_code = load_shader_module::<fn(&str) -> &str>(name, None)?;

	Ok(wgpu::ShaderModuleDescriptor {
		label: Some(name),
		source: wgpu::ShaderSource::Wgsl(shader_code.into()),
	})
}

pub fn load_shader_consts<F: Fn(&str) -> &str>(name: &str, get_const: F) -> Result<wgpu::ShaderModuleDescriptor, std::io::Error>  {
	let shader_code = load_shader_module(name, Some(&get_const))?;

	Ok(wgpu::ShaderModuleDescriptor {
		label: Some(name),
		source: wgpu::ShaderSource::Wgsl(shader_code.into()),
	})
}

pub struct GameRenderer {
	pub chunk_renderer: chunk::ChunkRenderer,
	pub ui_renderer: ui::UiRenderer,
	// uniform_buffer: wgpu::Buffer,
	graph: graph::Graph<super::GameState>,
}

impl GameRenderer {
	pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;
	pub const SAMPLES: u32 = 4;
	pub const CLEAR_COLOR: wgpu::Color = wgpu::Color { r: 0.2, g: 0.3, b: 0.5, a: 1.0 };

	pub fn new(gfx: &gfx::Gfx, block_textures: &super::texture::LoadedTextures) -> Self {
		let graph_spec = graph::GraphSpec::<super::GameState> {
			attachments: &[
				Some(("output", graph::AttachmentSpec::Output(graph::OutputAttachmentSpec {
					ops: |_: &gfx::Gfx| wgpu::Operations {
						load: wgpu::LoadOp::Clear(if Self::SAMPLES == 1 { Self::CLEAR_COLOR } else { wgpu::Color::BLACK }),
						store: true
					}
				}))),
				if Self::SAMPLES != 1 {
					Some(("msaa-output", graph::AttachmentSpec::Color(graph::ColorAttachmentSpec {
						format: gfx.config.format,
						resolve: Some("output"),
						samples: Self::SAMPLES,
						size: graph::AttachmentSizeSpec::Output(1.0),
						ops: |_: &gfx::Gfx| wgpu::Operations {
							load: wgpu::LoadOp::Clear(Self::CLEAR_COLOR),
							store: false // we can discard, since the unresolved output isn't needed.
						},
					})))
				} else { None },
				Some(("depth", graph::AttachmentSpec::DepthStencil(graph::DepthStencilAttachmentSpec {
					format: Self::DEPTH_FORMAT,
					depth_ops: Some(|_: &gfx::Gfx| Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: true })),
					stencil_ops: None,
					samples: Self::SAMPLES // TODO: allow user to sample count
				}))),
			],
			nodes: &[
				graph::NodeSpec {
					id: "main",
					color_attachments: &[if Self::SAMPLES == 1 { "output" } else { "msaa-output" }],
					depth_stencil_attachment: Some("depth"),
					render: |gfx, render_pass, game| {
						game.renderer.render_main(gfx, render_pass, game);
					}
				}
			],
		};
	
		Self {
			chunk_renderer: chunk::ChunkRenderer::new(gfx, block_textures),
			ui_renderer: ui::UiRenderer::new(gfx),
			graph: graph_spec.build(gfx),
		}
	}

	pub fn render(&self, ctx: &mut gfx::RenderContext, game: &super::GameState) {
		ctx.render_graph(&self.graph, game);
	}

	fn render_main<'ctx>(&'ctx self, gfx: &gfx::Gfx, render_pass: &mut wgpu::RenderPass<'ctx>, game: &'ctx super::GameState) {
		game.on_render(gfx, &mut GameRenderContext { renderer: self, render_pass });
	}

	pub fn update(&mut self, gfx: &gfx::Gfx, ui_builder: UiBuilder) {
		self.chunk_renderer.update();
		self.ui_renderer.update(gfx, ui_builder);
	}
}

pub struct GameRenderContext<'a, 'b> {
	renderer: &'a GameRenderer,
	render_pass: &'b mut wgpu::RenderPass<'a>,
}

impl<'a, 'b> GameRenderContext<'a, 'b> {
	pub fn begin_chunk_context<'ctx>(&'ctx mut self, gfx: &gfx::Gfx) -> ChunkRenderContext<'a, 'ctx> {
		ChunkRenderContext::begin(gfx, self.renderer, self.render_pass)
	}

	pub fn begin_ui_context<'ctx>(&'ctx mut self, gfx: &gfx::Gfx) -> UiRenderContext<'a, 'ctx> {
		UiRenderContext::begin(gfx, self.renderer, self.render_pass)
	}
}
