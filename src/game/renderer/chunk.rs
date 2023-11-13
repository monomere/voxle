use wgpu::util::DeviceExt;

use crate::{gfx, math::*, game::{texture, chunk::CHUNK_SIZE}};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct OutlineVertex {
	pub position: [f32; 3],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BlockVertex {
	pub data0: u32, // z:10 y:10 x:10 uv:2
	pub data1: u32, // tid:24 ao:8
}

fn i32_to_i10(i: i32) -> u32 {
	let i = i as u32;
	((i >> 31) << 9) | (i & 0x1ff)
}

impl BlockVertex {
	pub fn new(pos: Vec3f32, uv: u8, ao: &[u8; 4], tex: u32) -> Self {
		Self {
			data0: ((uv as u32 & 0b11) << 30)
				| (i32_to_i10((pos.x * 2.0) as i32) << 00)
				| (i32_to_i10((pos.y * 2.0) as i32) << 10)
				| (i32_to_i10((pos.z * 2.0) as i32) << 20),
			data1: ((tex as u32) << 8)
				| ((ao[0] as u32 & 0b11) << 0)
				| ((ao[1] as u32 & 0b11) << 2)
				| ((ao[2] as u32 & 0b11) << 4)
				| ((ao[3] as u32 & 0b11) << 6)
		}
	}
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BlockPushConsts {
	chunk_pos: [i32; 3]
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct OutlinePushConstants {
	position: [f32; 3]
}

fn create_block_pipeline(
	gfx: &gfx::Gfx,
	layout: &wgpu::PipelineLayout,
	shader: &wgpu::ShaderModule,
	polymode: wgpu::PolygonMode,
	depth_format: wgpu::TextureFormat
) -> wgpu::RenderPipeline {
	gfx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some("Block Pipeline"),
		layout: Some(&layout),
		vertex: wgpu::VertexState {
			module: &shader,
			entry_point: "vs_main",
			buffers: &[wgpu::VertexBufferLayout {
				array_stride: std::mem::size_of::<BlockVertex>() as wgpu::BufferAddress,
				step_mode: wgpu::VertexStepMode::Vertex,
				attributes: &[
					// data0 (position, ao)
					wgpu::VertexAttribute {
						format: wgpu::VertexFormat::Uint32,
						offset: 0,
						shader_location: 0
					},
					// data1 (uvs, texture)
					wgpu::VertexAttribute {
						format: wgpu::VertexFormat::Uint32,
						offset: 4,
						shader_location: 1
					},
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
			cull_mode: Some(wgpu::Face::Back),
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
				_ => wgpu::CompareFunction::LessEqual,
			},
			stencil: wgpu::StencilState::default(),
			bias: wgpu::DepthBiasState::default(),
		}),
		multisample: wgpu::MultisampleState {
			count: super::GameRenderer::SAMPLES,
			mask: !0,
			alpha_to_coverage_enabled: false
		},
		multiview: None
	})
}

fn create_outline_pipeline(
	gfx: &gfx::Gfx,
	layout: &wgpu::PipelineLayout,
	shader: &wgpu::ShaderModule,
	depth_format: wgpu::TextureFormat
) -> wgpu::RenderPipeline {
	gfx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: Some("Outline Pipeline"),
		layout: Some(&layout),
		vertex: wgpu::VertexState {
			module: &shader,
			entry_point: "vs_main",
			buffers: &[wgpu::VertexBufferLayout {
				array_stride: std::mem::size_of::<OutlineVertex>() as wgpu::BufferAddress,
				step_mode: wgpu::VertexStepMode::Vertex,
				attributes: &[
					// position
					wgpu::VertexAttribute {
						format: wgpu::VertexFormat::Float32x3,
						offset: 0,
						shader_location: 0
					},
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
			topology: wgpu::PrimitiveTopology::LineList,
			strip_index_format: None,
			front_face: wgpu::FrontFace::Cw,
			cull_mode: Some(wgpu::Face::Back),
			unclipped_depth: false,
			polygon_mode: wgpu::PolygonMode::Line,
			conservative: false
		},
		depth_stencil: Some(wgpu::DepthStencilState {
			format: depth_format,
			depth_write_enabled: false,
			depth_compare: wgpu::CompareFunction::Less,
			stencil: wgpu::StencilState::default(),
			bias: wgpu::DepthBiasState::default(),
		}),
		multisample: wgpu::MultisampleState {
			count: super::GameRenderer::SAMPLES,
			mask: !0,
			alpha_to_coverage_enabled: false
		},
		multiview: None
	})
}

pub struct Camera {
	pub position: Vec3f32,
	pub yaw: f32,
	pub pitch: f32,
	pub aspect: f32,
	pub fovy: f32,
	pub znear: f32,
	pub zfar: f32,
}

impl Camera {
	fn build_view_proj_matrix(&self) -> glm::Mat4 {
		let direction = self.direction();

		let view = glm::look_at_rh(
			&glm::vec3(self.position.x, self.position.y, self.position.z),
			&glm::TVec::from_column_slice(&(self.position + direction).0),
			&glm::vec3(0.0, 1.0, 0.0)
		);
		
		let proj = glm::perspective_rh_zo(self.aspect, glm::radians(&glm::vec1(self.fovy)).x, self.znear, self.zfar);

		proj * view
	}

	pub fn direction(&self) -> Vec3f32 {
		vec3(
			self.yaw.cos() * self.pitch.cos(),
			self.pitch.sin(),
			self.yaw.sin() * self.pitch.cos(),
		)
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

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct LightingUniform {
	sun_dir: [f32; 4]
}

struct WorldUniforms {
	data: Vec<u8>,
	parts: [usize; 2]
}

fn align_up(val: usize, align: usize) -> usize {
	let mask = align - 1;
	if val & mask == 0 { val } else { (val | mask) + 1 }
}

impl WorldUniforms {
	fn new(align: usize) -> Self {
		let parts = [
			align_up(std::mem::size_of::<CameraUniform>(), align),
			std::mem::size_of::<LightingUniform>()
		];

		let size = parts.iter().sum();
		
		Self {
			data: bytemuck::zeroed_vec(size),
			parts
		}
	}

	#[allow(dead_code)]
	fn camera_uniform<'a>(&'a self) -> &'a CameraUniform {
		bytemuck::from_bytes(&self.data[self.camera_uniform_range()])
	}

	#[allow(dead_code)]
	fn lighting_uniform<'a>(&'a self) -> &'a LightingUniform {
		bytemuck::from_bytes(&self.data[self.lighting_uniform_range()])
	}

	fn camera_uniform_mut<'a>(&'a mut self) -> &'a mut CameraUniform {
		let range = self.camera_uniform_range();
		bytemuck::from_bytes_mut(&mut self.data[range])
	}

	fn lighting_uniform_mut<'a>(&'a mut self) -> &'a mut LightingUniform {
		let range = self.lighting_uniform_range();
		bytemuck::from_bytes_mut(&mut self.data[range])
	}

	const fn camera_uniform_offset(&self) -> usize { 0 }
	const fn camera_uniform_size(&self) -> usize { std::mem::size_of::<CameraUniform>() }
	const fn lighting_uniform_offset(&self) -> usize { self.parts[0] }
	const fn lighting_uniform_size(&self) -> usize { std::mem::size_of::<LightingUniform>() }
	const fn camera_uniform_range(&self) -> std::ops::Range<usize> {
		self.camera_uniform_offset() .. self.camera_uniform_offset() + self.camera_uniform_size()
	}
	const fn lighting_uniform_range(&self) -> std::ops::Range<usize> {
		self.lighting_uniform_offset() .. self.lighting_uniform_offset() + self.lighting_uniform_size()
	}
}

pub struct ChunkRenderer {
	block_render_pipeline: wgpu::RenderPipeline,
	block_wf_render_pipeline: wgpu::RenderPipeline,
	outline_render_pipeline: wgpu::RenderPipeline,
	uniform_bind_group: wgpu::BindGroup,
	world_uniforms_buffer: wgpu::Buffer,
	texture_bind_group: wgpu::BindGroup,
	_texture: gfx::Texture,
	world_uniforms: WorldUniforms,
	outline_mesh: gfx::Mesh<OutlineVertex>,
	pub camera: Camera
}

impl ChunkRenderer {
	fn create_world_uniforms(gfx: &gfx::Gfx) -> WorldUniforms {
		WorldUniforms::new(gfx.device.limits().min_uniform_buffer_offset_alignment as usize)
	}

	pub fn new(gfx: &gfx::Gfx, block_textures: &texture::LoadedTextures) -> Self {
		let camera = Camera {
			position: Vector([0.0, 128.5, -2.0]),
			yaw: 3.0 * glm::quarter_pi::<f32>(),
			pitch: 0.0,
			aspect: gfx.config.width as f32 / gfx.config.height as f32,
			fovy: 60.0,
			znear: 0.01,
			zfar: 1000.0
		};

		let world_bind_group_layout = gfx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: None,
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					count: None,
					visibility: wgpu::ShaderStages::VERTEX,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Uniform,
						has_dynamic_offset: false,
						min_binding_size: None
					},
				},
				wgpu::BindGroupLayoutEntry {
					binding: 1,
					count: None,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Uniform,
						has_dynamic_offset: false,
						min_binding_size: None
					},
				}
			]
		});

		let texture_bind_group_layout = gfx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("Texture Bind Group Layout"),
			entries: &[
				wgpu::BindGroupLayoutEntry {
        	binding: 0,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Texture {
						multisampled: false,
						view_dimension: wgpu::TextureViewDimension::D2Array,
						sample_type: wgpu::TextureSampleType::Float { filterable: true }
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
        	binding: 1,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
					count: None
				}
			],
		});

		let block_pipeline_layout = gfx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: None,
			bind_group_layouts: &[&world_bind_group_layout, &texture_bind_group_layout],
			push_constant_ranges: &[
				wgpu::PushConstantRange {
					range: 0..std::mem::size_of::<BlockPushConsts>() as u32,
					stages: wgpu::ShaderStages::VERTEX
				}
			]
		});

		let outline_pipeline_layout = gfx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: None,
			bind_group_layouts: &[&world_bind_group_layout],
			push_constant_ranges: &[wgpu::PushConstantRange {
				range: 0..std::mem::size_of::<OutlinePushConstants>() as u32,
				stages: wgpu::ShaderStages::VERTEX
			}]
		});

		fn shader_get_const(is_black: bool) -> impl for<'a> Fn(&'a str) -> &'a str {
			move |name: &str| -> &str {
				match name {
					"is_black" => if is_black { "true" } else { "false" },
					s => s
				}
			}
		}
		let block_shader = gfx.device.create_shader_module(super::load_shader_consts("game/block", shader_get_const(false)).unwrap());
		let wf_block_shader = gfx.device.create_shader_module(super::load_shader_consts("game/block", shader_get_const(true)).unwrap());
		let outline_shader = gfx.device.create_shader_module(super::load_shader("game/outline").unwrap());

		let block_render_pipeline = create_block_pipeline(gfx, &block_pipeline_layout, &block_shader, wgpu::PolygonMode::Fill, super::GameRenderer::DEPTH_FORMAT);
		let block_wf_render_pipeline = create_block_pipeline(gfx, &block_pipeline_layout, &wf_block_shader, wgpu::PolygonMode::Line, super::GameRenderer::DEPTH_FORMAT);
		let outline_render_pipeline = create_outline_pipeline(gfx, &outline_pipeline_layout, &outline_shader, super::GameRenderer::DEPTH_FORMAT);

		let block_texture = gfx.device.create_texture(&wgpu::TextureDescriptor {
			label: Some("Block Array Texture"),
			dimension: wgpu::TextureDimension::D2,
			format: wgpu::TextureFormat::Rgba8UnormSrgb,
			mip_level_count: 1,
			sample_count: 1,
			size: wgpu::Extent3d {
				width: block_textures.size.x,
				height: block_textures.size.y,
				depth_or_array_layers: block_textures.textures.len() as u32
			},
			usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
			view_formats: &[]
		});

		for texture_source in &block_textures.textures {
			if let Some(ref texture_data) = &texture_source.data {
				gfx.queue.write_texture(
					wgpu::ImageCopyTexture {
						aspect: wgpu::TextureAspect::All,
						mip_level: 0,
						origin: wgpu::Origin3d {
							x: 0,
							y: 0,
							z: texture_source.id.0
						},
						texture: &block_texture
					},
					&texture_data,
					wgpu::ImageDataLayout {
						offset: 0,
						bytes_per_row: Some(block_texture.width() * 4),
						rows_per_image: Some(block_texture.height())
					},
					wgpu::Extent3d {
						width: block_texture.width(),
						height: block_texture.height(),
						depth_or_array_layers: 1
					}
				);
			}
		}

		let block_texture_view = block_texture.create_view(&wgpu::TextureViewDescriptor {
			label: Some("Block Array Texture View"),
			aspect: wgpu::TextureAspect::All,
			array_layer_count: Some(block_texture.depth_or_array_layers()),
			format: Some(block_texture.format()),
			dimension: Some(wgpu::TextureViewDimension::D2Array),
			base_mip_level: 0,
			mip_level_count: None,
			base_array_layer: 0
		});

		let block_texture = gfx::Texture {
			texture: block_texture,
			view: block_texture_view,
			sampler: Some(gfx.device.create_sampler(&wgpu::SamplerDescriptor {
				address_mode_u: wgpu::AddressMode::ClampToEdge,
				address_mode_v: wgpu::AddressMode::ClampToEdge,
				address_mode_w: wgpu::AddressMode::ClampToEdge,
				mag_filter: wgpu::FilterMode::Nearest,
				min_filter: wgpu::FilterMode::Nearest,
				mipmap_filter: wgpu::FilterMode::Nearest,
				compare: None,
				lod_min_clamp: 0.0,
				lod_max_clamp: 0.0,
				..Default::default()
			})),
		};

		let texture_bind_group = gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: None,
			layout: &texture_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&block_texture.view),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(block_texture.sampler.as_ref().unwrap()),
				}
			]
		});

		let world_uniforms = Self::create_world_uniforms(gfx);
		let world_uniforms_buffer = Self::create_uniform_buffer(gfx, &world_uniforms.data);

		let uniform_bind_group = gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: None,
			layout: &world_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
						buffer: &world_uniforms_buffer,
						offset: world_uniforms.camera_uniform_offset() as u64,
						size: Some(std::num::NonZeroU64::new(world_uniforms.camera_uniform_size() as u64).unwrap())
					})
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
						buffer: &world_uniforms_buffer,
						offset: world_uniforms.lighting_uniform_offset() as u64,
						size: Some(std::num::NonZeroU64::new(world_uniforms.lighting_uniform_size() as u64).unwrap())
					})
				}
			]
		});

		let outline_mesh = {
			let vertices = super::super::chunk::CUBE_VERTICES.map(|v| OutlineVertex { position: v });
			let indices: [u32; 24] = [
				0, 1,  1, 2,  2, 3,  3, 0,
				4, 5,  5, 6,  6, 7,  7, 4,
				0, 4,  1, 5,  2, 6,  3, 7,
			];
			gfx::Mesh::new(gfx, &vertices, &indices, Some("Block Outline Mesh"))
		};

		Self {
			block_render_pipeline,
			block_wf_render_pipeline,
			outline_render_pipeline,
			_texture: block_texture,
			texture_bind_group,
			world_uniforms_buffer,
			uniform_bind_group,
			world_uniforms,
			outline_mesh,
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

	/// NB: run before rendering.
	pub fn set_sun_direction(&mut self, dir: Vec4f32) {
		self.world_uniforms.lighting_uniform_mut().sun_dir = dir.0;
	}

	#[allow(dead_code)]
	pub fn sun_direction(&self) -> Vec4f32 {
		Vector(self.world_uniforms.lighting_uniform().sun_dir)
	}

	pub fn update(&mut self) {
		*self.world_uniforms.camera_uniform_mut() = self.camera.to_uniform()
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
		gfx: &gfx::Gfx,
		renderer: &'a super::GameRenderer,
		render_pass: &'b mut wgpu::RenderPass<'a>
	) -> ChunkRenderContext<'a, 'b> {
		let ctx = ChunkRenderContext {
			renderer,
			render_pass
		};
		gfx.queue.write_buffer(
			&renderer.chunk_renderer.world_uniforms_buffer,
			0,
			&renderer.chunk_renderer.world_uniforms.data
		);
		ctx
	}
	
	// TODO: states/game/renderer -> renderer?

	pub fn set_mode(&mut self, mode: ChunkRenderMode) {
		self.render_pass.set_pipeline(match mode {
			ChunkRenderMode::Normal => &self.renderer.chunk_renderer.block_render_pipeline,
			ChunkRenderMode::Wireframe => &self.renderer.chunk_renderer.block_wf_render_pipeline,
		});

		self.render_pass.set_bind_group(0, &self.renderer.chunk_renderer.uniform_bind_group, &[]);
		self.render_pass.set_bind_group(1, &self.renderer.chunk_renderer.texture_bind_group, &[]); // TODO: make this a GameRenderer thing
	}

	pub fn render_chunk(&mut self, chunk: &'a super::super::chunk::Chunk) {
		if let Some(mesh) = &chunk.mesh {
			self.render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 0, bytemuck::bytes_of(&BlockPushConsts {
				chunk_pos: (chunk.position * CHUNK_SIZE.each_as()).0
			}));
			mesh.render(self.render_pass)
		}
	}

	pub fn render_outline(&mut self, position: Vec3f32) {
		self.render_pass.set_pipeline(&self.renderer.chunk_renderer.outline_render_pipeline);
		self.render_pass.set_bind_group(0, &self.renderer.chunk_renderer.uniform_bind_group, &[]);
		self.render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 0, bytemuck::bytes_of(&OutlinePushConstants {
			position: position.0
		}));

		self.renderer.chunk_renderer.outline_mesh.render(self.render_pass);
	}
}
