use wgpu::util::DeviceExt;

use crate::{gfx, math::{Vec2u32, vec2, vec3, Vec3f32, Vector, Vec4f32}};

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
		let direction = vec3(
			self.yaw.cos() * self.pitch.cos(),
			self.pitch.sin(),
			self.yaw.sin() * self.pitch.cos(),
		);

		let view = glm::look_at_rh(
			&glm::vec3(self.position.x, self.position.y, self.position.z),
			&glm::TVec::from_column_slice(&(self.position + direction).0),
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

	fn camera_uniform<'a>(&'a self) -> &'a CameraUniform {
		bytemuck::from_bytes(&self.data[self.camera_uniform_range()])
	}

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
	render_pipeline: wgpu::RenderPipeline,
	wf_render_pipeline: wgpu::RenderPipeline,
	uniform_bind_group: wgpu::BindGroup,
	world_uniforms_buffer: wgpu::Buffer,
	texture_bind_group: wgpu::BindGroup,
	texture: gfx::Texture,
	world_uniforms: WorldUniforms,
	pub camera: Camera
}

impl ChunkRenderer {
	fn create_world_uniforms(gfx: &gfx::Gfx) -> WorldUniforms {
		WorldUniforms::new(gfx.device.limits().min_uniform_buffer_offset_alignment as usize)
	}

	pub fn new(gfx: &gfx::Gfx) -> Self {
		let camera = Camera {
			position: Vector([0.0, 0.5, -2.0]),
			yaw: 3.0 * glm::quarter_pi::<f32>(),
			pitch: 0.0,
			aspect: gfx.config.width as f32 / gfx.config.height as f32,
			fovy: 60.0,
			znear: 0.01,
			zfar: 1000.0
		};

		let shader = gfx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: None,
			source: wgpu::ShaderSource::Wgsl(std::fs::read_to_string("src/shader_3d.wgsl").unwrap().into())
		});

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
						view_dimension: wgpu::TextureViewDimension::D2,
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

		let layout = gfx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: None,
			bind_group_layouts: &[&world_bind_group_layout, &texture_bind_group_layout],
			push_constant_ranges: &[wgpu::PushConstantRange {
				range: 0..std::mem::size_of::<ChunkPushConstants>() as u32,
				stages: wgpu::ShaderStages::FRAGMENT
			}]
		});

		let render_pipeline = create_pipeline(gfx, &layout, &shader, wgpu::PolygonMode::Fill, super::GameRenderer::DEPTH_FORMAT);
		let wf_render_pipeline = create_pipeline(gfx, &layout, &shader, wgpu::PolygonMode::Line, super::GameRenderer::DEPTH_FORMAT);

		let texture = {
			let bytes = image::load(std::io::BufReader::new(std::fs::File::open("data/spritesheet.png").unwrap()), image::ImageFormat::Png).unwrap();
			let rgba8 = bytes.to_rgba8();
			let texture = gfx::Texture::create_binding_texture(
				gfx,
				wgpu::TextureFormat::Rgba8UnormSrgb,
				wgpu::Extent3d {
					width: rgba8.width(),
					height: rgba8.height(),
					depth_or_array_layers: 1,
				}
			);
			gfx.queue.write_texture(
				texture.texture.as_image_copy(),
				&rgba8,
				wgpu::ImageDataLayout {
					offset: 0,
					bytes_per_row: Some(4 * texture.size().width),
					rows_per_image: Some(texture.size().height),
				}, texture.size()
			);
			texture
		};

		let texture_bind_group = gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: None,
			layout: &texture_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&texture.view),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(texture.sampler.as_ref().unwrap()),
				}
			]
		});

		let world_uniforms = Self::create_world_uniforms(gfx);
		let uniform_buffer = Self::create_uniform_buffer(gfx, &world_uniforms.data);

		let uniform_bind_group = gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: None,
			layout: &world_bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
						buffer: &uniform_buffer,
						offset: world_uniforms.camera_uniform_offset() as u64,
						size: Some(std::num::NonZeroU64::new(world_uniforms.camera_uniform_size() as u64).unwrap())
					})
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
						buffer: &uniform_buffer,
						offset: world_uniforms.lighting_uniform_offset() as u64,
						size: Some(std::num::NonZeroU64::new(world_uniforms.lighting_uniform_size() as u64).unwrap())
					})
				}
			]
		});

		Self {
			render_pipeline,
			wf_render_pipeline,
			texture,
			texture_bind_group,
			world_uniforms_buffer: uniform_buffer,
			uniform_bind_group,
			world_uniforms,
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

	pub fn texture_size(&self) -> Vec2u32 {
		let size = self.texture.size();
		vec2(size.width, size.height)
	}

	/// NB: run before rendering.
	pub fn set_sun_direction(&mut self, dir: Vec4f32) {
		self.world_uniforms.lighting_uniform_mut().sun_dir = dir.0;
	}

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
		mode: ChunkRenderMode,
		renderer: &'a super::GameRenderer,
		render_pass: &'b mut wgpu::RenderPass<'a>
	) -> ChunkRenderContext<'a, 'b> {
		let mut ctx = ChunkRenderContext {
			renderer,
			render_pass
		};
		ctx.set_mode(mode);
		gfx.queue.write_buffer(
			&renderer.chunk_renderer.world_uniforms_buffer,
			0,
			&renderer.chunk_renderer.world_uniforms.data
		);
		ctx
	}
	
	// TODO: states/game/renderer -> renderer?

	fn set_mode(&mut self, mode: ChunkRenderMode) {
		self.render_pass.set_pipeline(match mode {
			ChunkRenderMode::Normal => &self.renderer.chunk_renderer.render_pipeline,
			ChunkRenderMode::Wireframe => &self.renderer.chunk_renderer.wf_render_pipeline,
		});

		let pushed = match mode {
			ChunkRenderMode::Normal => ChunkPushConstants { color: [1.0, 1.0, 1.0, 1.0] },
			ChunkRenderMode::Wireframe => ChunkPushConstants { color: [0.0, 0.0, 0.0, 1.0] },
		};

		self.render_pass.set_bind_group(0, &self.renderer.chunk_renderer.uniform_bind_group, &[]);
		self.render_pass.set_bind_group(1, &self.renderer.chunk_renderer.texture_bind_group, &[]); // TODO: make this a GameRenderer thing
		self.render_pass.set_push_constants(wgpu::ShaderStages::FRAGMENT, 0, bytemuck::bytes_of(&pushed));
	}

	pub fn render_chunk(&mut self, chunk: &'a super::super::chunk::Chunk) {
		if let Some(mesh) = &chunk.mesh {
			mesh.render(self.render_pass)
		}
	}
}

