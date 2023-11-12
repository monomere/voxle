use wgpu::util::DeviceExt;

use crate::{gfx, math::*};


#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiVertex {
	pub position: [f32; 2],
	pub texcoord: [f32; 2],
}

fn create_pipeline(
	gfx: &gfx::Gfx,
	layout: &wgpu::PipelineLayout,
	shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
	gfx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		label: None,
		layout: Some(&layout),
		vertex: wgpu::VertexState {
			module: &shader,
			entry_point: "vs_main",
			buffers: &[wgpu::VertexBufferLayout {
				array_stride: std::mem::size_of::<UiVertex>() as wgpu::BufferAddress,
				step_mode: wgpu::VertexStepMode::Vertex,
				attributes: &[
					// position
					wgpu::VertexAttribute {
						format: wgpu::VertexFormat::Float32x2,
						offset: 0,
						shader_location: 0
					},
					// texcoord
					wgpu::VertexAttribute {
						format: wgpu::VertexFormat::Float32x2,
						offset: 4 * 2,
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
					blend: Some(wgpu::BlendState {
						color: wgpu::BlendComponent {
							src_factor: wgpu::BlendFactor::SrcAlpha,
							dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
							operation: wgpu::BlendOperation::Add,
						},
						alpha: wgpu::BlendComponent::OVER
					}),
					write_mask: wgpu::ColorWrites::ALL
				})
			]
		}),
		primitive: wgpu::PrimitiveState {
			topology: wgpu::PrimitiveTopology::TriangleList,
			strip_index_format: None,
			front_face: wgpu::FrontFace::Cw,
			cull_mode: None,
			unclipped_depth: false,
			polygon_mode: wgpu::PolygonMode::Fill,
			conservative: false
		},
		depth_stencil: Some(wgpu::DepthStencilState {
			format: super::GameRenderer::DEPTH_FORMAT,
			depth_write_enabled: false,
			depth_compare: wgpu::CompareFunction::Always,
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

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ViewUniform {
	proj: [[f32; 4]; 4]
}

// pub struct UiRect {
// 	x: 
// }

pub struct UiPrimitive {
	offset: u32,
	count: u32,
}

pub struct UiBuilder {
	vertices: Vec<UiVertex>,
	indices: Vec<u32>,
	primitives: Vec<UiPrimitive>,
	screen_size: Vec2u32,
	texture_size: Vec2u32,
}

impl UiBuilder {
	pub fn new(screen_size: Vec2u32, texture_size: Vec2u32) -> Self {
		Self {
			vertices: vec![],
			indices: vec![],
			primitives: vec![],
			screen_size,
			texture_size
		}
	}

	pub fn add_rect(
		&mut self,
		rect: Rect<i32>,
		uvs: Rect<i32>,
	) {
		let vertex_offset = self.vertices.len() as u32;
		let rect = {
			let mut r = rect;
			r.x -= self.screen_size.x as i32 / 2;
			r.y -= self.screen_size.y as i32 / 2;
			r
		}.each_as::<f32>() / self.screen_size.each_as();
		let uvs = uvs.each_as::<f32>() / self.texture_size.each_as();

		// note: uvs flipped on y
		self.vertices.extend_from_slice(&[
			UiVertex { position: [rect.x1(), rect.y1()], texcoord: [uvs.x1(), uvs.y2()] },
			UiVertex { position: [rect.x2(), rect.y1()], texcoord: [uvs.x2(), uvs.y2()] },
			UiVertex { position: [rect.x2(), rect.y2()], texcoord: [uvs.x2(), uvs.y1()] },
			UiVertex { position: [rect.x1(), rect.y2()], texcoord: [uvs.x1(), uvs.y1()] },
		]);

		/*
		    #0                #1
			(x1,y1) --------- (x2,y1)
			   | `-._            |
				 |      `-._       |
				 |           `-._  |
			(x1,y2) --------- (x2,y2)
		    #3                #2
		*/

		let index_offset = self.indices.len() as u32;
		self.indices.extend_from_slice(&[
			vertex_offset + 0,
			vertex_offset + 1,
			vertex_offset + 2,
			vertex_offset + 0,
			vertex_offset + 2,
			vertex_offset + 3,
		]);

		self.primitives.push(UiPrimitive { offset: index_offset, count: 6 })
	}
}

pub struct UiRenderer {
	quad_render_pipeline: wgpu::RenderPipeline,
	uniform_bind_group: wgpu::BindGroup,
	view_uniform_buffer: wgpu::Buffer,
	texture: gfx::Texture,
	mesh: gfx::Mesh<UiVertex>,
	primitives: Vec<UiPrimitive>
}

impl UiRenderer {
	pub fn new(gfx: &gfx::Gfx) -> Self {
		let bind_group_layout = gfx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Texture {
						multisampled: false,
						view_dimension: wgpu::TextureViewDimension::D2,
						sample_type: wgpu::TextureSampleType::Float { filterable: true }
					},
					count: None,
				},
				wgpu::BindGroupLayoutEntry {
        	binding: 2,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
					count: None
				}
			]
		});

		let quad_pipeline_layout = gfx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: None,
			bind_group_layouts: &[&bind_group_layout],
			push_constant_ranges: &[]
		});
		
		let quad_shader = gfx.device.create_shader_module(super::load_shader("ui/quad").unwrap());
		let quad_render_pipeline = create_pipeline(gfx, &quad_pipeline_layout, &quad_shader);

		let texture = {
			let bytes = image::load(std::io::BufReader::new(std::fs::File::open("data/textures/ui_spritesheet.png").unwrap()), image::ImageFormat::Png).unwrap();
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

		let view_uniform_buffer = gfx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: None,
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			contents: bytemuck::bytes_of(&ViewUniform { proj: [[0.0; 4]; 4] })
		});

		let uniform_bind_group = gfx.device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: None,
			layout: &bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: view_uniform_buffer.as_entire_binding()
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::TextureView(&texture.view),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::Sampler(texture.sampler.as_ref().unwrap()),
				}
			]
		});

		Self {
			quad_render_pipeline,
			texture,
			uniform_bind_group,
			view_uniform_buffer,
			mesh: gfx::Mesh::new(gfx, &[], &[], Some("UI Mesh")),
			primitives: vec![]
		}
	}

	pub fn texture_size(&self) -> Vec2u32 {
		let size = self.texture.size();
		vec2(size.width, size.height)
	}

	pub fn update(&mut self, gfx: &gfx::Gfx, builder: UiBuilder) {
		self.mesh.update(gfx, &builder.vertices, &builder.indices);
		self.primitives = builder.primitives;
		gfx.queue.write_buffer(
			&self.view_uniform_buffer,
			0,
			bytemuck::bytes_of(&ViewUniform {
				proj: ortho_matrix(0.0, gfx.config.width as f32, gfx.config.height as f32, 0.0),
			})
		);
	}
}

pub struct UiRenderContext<'a, 'b> {
	pub(super) renderer: &'a super::GameRenderer,
	pub(super) render_pass: &'b mut wgpu::RenderPass<'a>
}

impl<'a, 'b> UiRenderContext<'a, 'b> {
	pub(super) fn begin(
		_gfx: &gfx::Gfx,
		renderer: &'a super::GameRenderer,
		render_pass: &'b mut wgpu::RenderPass<'a>
	) -> UiRenderContext<'a, 'b> {
		Self {
			renderer,
			render_pass
		}
	}
	
	pub fn render(&mut self) {
		self.render_pass.set_pipeline(&self.renderer.ui_renderer.quad_render_pipeline);
		self.render_pass.set_bind_group(0, &self.renderer.ui_renderer.uniform_bind_group, &[]);
		self.render_pass.set_vertex_buffer(0, self.renderer.ui_renderer.mesh.buffers.vertex_buffer.slice(..));
		self.render_pass.set_index_buffer(self.renderer.ui_renderer.mesh.buffers.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
		for primitive in &self.renderer.ui_renderer.primitives {
			self.render_pass.draw_indexed(primitive.offset..primitive.count, 0, 0..1);
		}
	}
}
