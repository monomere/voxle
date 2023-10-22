use std::marker::PhantomData;

use wgpu::util::DeviceExt;
use winit::raw_window_handle::{HasWindowHandle, HasDisplayHandle};

use crate::{scene::State, Window};

use self::graph::GraphRenderContext;


pub trait Renderer<P> {
	fn on_use<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, param: P);
}

pub struct Gfx {
	pub surface: wgpu::Surface,
	pub device: wgpu::Device,
	pub queue: wgpu::Queue,
	pub config: wgpu::SurfaceConfiguration,
	pub size: winit::dpi::PhysicalSize<u32>,
	// The window must be declared after the surface so
	// it gets dropped after it as the surface contains
	// unsafe references to the window's resources.
	pub window: Window,
}

impl Gfx {
	pub async fn new(window: Window) -> Self {
		let size = window.window.inner_size();
		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
			backends: wgpu::Backends::all(),
			dx12_shader_compiler: Default::default(),
		});
		
		// The surface needs to live as long as the window that created it.
		// State owns the window so this should be safe.
		let surface = unsafe { instance.create_surface(&window.window) }.unwrap();
		
		let adapter = instance.request_adapter(
			&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::default(),
				compatible_surface: Some(&surface),
				force_fallback_adapter: false,
			},
		).await.unwrap();
		
		let (device, queue) = adapter.request_device(
			&wgpu::DeviceDescriptor {
				features: wgpu::Features::POLYGON_MODE_LINE | wgpu::Features::PUSH_CONSTANTS,
				// WebGL doesn't support all of wgpu's features, so if
				// we're building for the web we'll have to disable some.
				limits: if cfg!(target_arch = "wasm32") {
					wgpu::Limits::downlevel_webgl2_defaults()
				} else {
					wgpu::Limits {
						max_push_constant_size: 128,
						..Default::default()
					}
				},
				label: None,
			},
			None, // Trace path
		).await.unwrap();
		
		let surface_caps = surface.get_capabilities(&adapter);
		
		let surface_format = surface_caps.formats.iter()
			.copied()
			.find(|f| f.is_srgb())            
			.unwrap_or(surface_caps.formats[0]);
		
		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: surface_format,
			width: size.width,
			height: size.height,
			present_mode: surface_caps.present_modes[0],
			alpha_mode: surface_caps.alpha_modes[0],
			view_formats: vec![],
		};
		
		surface.configure(&device, &config);

		Self {
			window,
			surface,
			device,
			queue,
			config,
			size,
		}
	}
	
	pub fn window(&self) -> &Window {
		&self.window
	}
	
	pub fn window_mut(&mut self) -> &mut Window {
		&mut self.window
	}

	pub fn size(&self) -> winit::dpi::PhysicalSize<u32> { self.size }
	
	pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
		if new_size.width > 0 && new_size.height > 0 {
			self.size = new_size;
			self.config.width = new_size.width;
			self.config.height = new_size.height;
			self.surface.configure(&self.device, &self.config);
		}
	}
	
	pub fn render(&mut self, state: &dyn State) -> Result<(), wgpu::SurfaceError> {
		let output = self.surface.get_current_texture()?;
		let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
		let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
		
		{
			let mut context = RenderContext {
				gfx: &self,
				output: Some(&view),
				encoder: &mut encoder,
			};

			state.render(&mut context);
		}
		
		self.queue.submit(std::iter::once(encoder.finish()));
		output.present();

		Ok(())
	}
}

pub mod graph;

#[derive(Debug)]
pub struct Texture {
	texture: wgpu::Texture,
	view: wgpu::TextureView,
	sampler: Option<wgpu::Sampler>
}

impl Texture {
	pub fn create_depth_texture(
		gfx: &Gfx,
		format: wgpu::TextureFormat,
		sample_count: u32
	) -> Self {
		let size = wgpu::Extent3d {
			width: gfx.config.width,
			height: gfx.config.height,
			depth_or_array_layers: 1
		};

		let texture = gfx.device.create_texture(&wgpu::TextureDescriptor {
			label: None,
			size,
			mip_level_count: 1,
			sample_count,
			dimension: wgpu::TextureDimension::D2,
			format,
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
			view_formats: &[],
		});

		let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

		let sampler = gfx.device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			address_mode_w: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Linear,
			mipmap_filter: wgpu::FilterMode::Nearest,
			compare: Some(wgpu::CompareFunction::LessEqual),
			lod_min_clamp: 0.0,
			lod_max_clamp: 100.0,
			..Default::default()
		});

		Self { texture, view, sampler: Some(sampler) }
	}

	pub fn create_attachment_texture(
		gfx: &Gfx,
		format: wgpu::TextureFormat,
		size: wgpu::Extent3d,
		sample_count: u32,
	) -> Self {
		let texture = gfx.device.create_texture(&wgpu::TextureDescriptor {
			label: None,
			size,
			mip_level_count: 1,
			sample_count,
			dimension: wgpu::TextureDimension::D2,
			format,
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
			view_formats: &[],
		});

		let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

		let sampler = gfx.device.create_sampler(&wgpu::SamplerDescriptor {
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			address_mode_w: wgpu::AddressMode::ClampToEdge,
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Linear,
			mipmap_filter: wgpu::FilterMode::Nearest,
			compare: None,
			lod_min_clamp: 0.0,
			lod_max_clamp: 100.0,
			..Default::default()
		});

		Self { texture, view, sampler: Some(sampler) }
	}
}

pub struct RenderContext<'a> {
	pub gfx: &'a Gfx,
	output: Option<&'a wgpu::TextureView>,
	encoder: &'a mut wgpu::CommandEncoder
}

impl<'a> RenderContext<'a> {
	pub fn render_graph<R>(&mut self, graph: &graph::Graph<R>, renderer: &R) {
		graph.render(&mut GraphRenderContext {
			gfx: self.gfx,
			output: self.output,
			encoder: self.encoder
		}, renderer);
	}
}

pub trait Vertex = Clone + Copy + bytemuck::Zeroable + bytemuck::Pod;

pub struct Mesh<V: Vertex> {
	buffers: MeshBuffers<V>,
}

struct MeshBuffers<V: Vertex> {
	index_count: usize,
	vertex_count: usize,
	vertex_buffer: wgpu::Buffer,
	index_buffer: wgpu::Buffer,
	_pd: PhantomData<V>
}

impl<V: Vertex> MeshBuffers<V> {
	fn new(gfx: &Gfx, vertices: &[V], indices: &[u32]) -> Self {
		Self {
			vertex_count: vertices.len(),
			index_count: indices.len(),
			vertex_buffer: Self::create_vertex_buffer(gfx, vertices),
			index_buffer: Self::create_index_buffer(gfx, indices),
			_pd: PhantomData
		}
	}

	fn update_vertex_buffer(&mut self, gfx: &Gfx, vertices: &[V]) {
		let bytes = bytemuck::cast_slice::<V, u8>(vertices);

		if bytes.len() as u64 == self.vertex_buffer.size() {
			gfx.queue.write_buffer(&self.vertex_buffer, 0, bytes);
		} else {
			self.vertex_count = vertices.len();
			self.vertex_buffer = Self::create_vertex_buffer(gfx, vertices);
		}
	}

	fn update_index_buffer(&mut self, gfx: &Gfx, indices: &[u32]) {
		let bytes = bytemuck::cast_slice::<u32, u8>(indices);

		if bytes.len() as u64 == self.index_buffer.size() {
			gfx.queue.write_buffer(&self.index_buffer, 0, bytes);
		} else {
			self.index_count = indices.len();
			self.index_buffer = Self::create_index_buffer(gfx, indices);
		}
	}

	fn create_vertex_buffer(gfx: &Gfx, vertices: &[V]) -> wgpu::Buffer {
		gfx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: None,
			usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX,
			contents: bytemuck::cast_slice(vertices),
		})
	}

	fn create_index_buffer(gfx: &Gfx, indices: &[u32]) -> wgpu::Buffer {
		gfx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: None,
			usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::INDEX,
			contents: bytemuck::cast_slice(indices),
		})
	}
}

impl<V: Vertex> Mesh<V> {
	pub fn new(gfx: &Gfx, vertices: &[V], indices: &[u32]) -> Self {
		Self { buffers: MeshBuffers::new(gfx, vertices, indices) }
	}

	pub fn update(&mut self, gfx: &Gfx, vertices: &[V], indices: &[u32]) {
		self.buffers.update_vertex_buffer(gfx, vertices);
		self.buffers.update_index_buffer(gfx, indices);
	}

	pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
		render_pass.set_vertex_buffer(0, self.buffers.vertex_buffer.slice(..));
		render_pass.set_index_buffer(self.buffers.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
		render_pass.draw_indexed(0..self.buffers.index_count as u32, 0, 0..1);
	}
}
