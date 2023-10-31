use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum AttachmentSizeSpec {
	Fixed(wgpu::Extent3d),
	/// With scale.
	Output(f32)
}

#[derive(Clone, Copy, Debug)]
pub struct ColorAttachmentSpec<Id> {
	pub format: wgpu::TextureFormat,
	pub resolve: Option<Id>,
	pub size: AttachmentSizeSpec,
	pub ops: fn(&super::Gfx) -> wgpu::Operations<wgpu::Color>,
	pub samples: u32
}

#[derive(Clone, Copy, Debug)]
pub struct DepthStencilAttachmentSpec {
	pub format: wgpu::TextureFormat,
	pub depth_ops: Option<fn(&super::Gfx) -> Option<wgpu::Operations<f32>>>,
	pub stencil_ops: Option<fn(&super::Gfx) -> Option<wgpu::Operations<u32>>>,
	pub samples: u32
}

#[derive(Clone, Copy, Debug)]
pub struct OutputAttachmentSpec {
	pub ops: fn(&super::Gfx) -> wgpu::Operations<wgpu::Color>
}


#[derive(Clone, Copy, Debug)]
pub enum AttachmentSpec<Id> {
	#[allow(dead_code)]
	Color(ColorAttachmentSpec<Id>),
	DepthStencil(DepthStencilAttachmentSpec),
	Output(OutputAttachmentSpec)
}

#[derive(Clone, Copy, Debug)]
pub struct NodeSpec<R: ?Sized, Id, Ids> {
	pub id: Id,
	pub color_attachments: Ids,
	pub depth_stencil_attachment: Option<Id>,
	pub render: for<'a> fn(gfx: &super::Gfx, render_pass: &mut wgpu::RenderPass<'a>, renderer: &'a R),
}

pub struct GraphSpec<'a, R> {
	pub attachments: &'a [Option<(&'a str, AttachmentSpec<&'a str>)>],
	pub nodes: &'a [NodeSpec<R, &'a str, &'a [&'a str]>]
}

#[derive(Debug)]
struct Attachment {
	spec: AttachmentSpec<u32>,
	texture: Option<super::Texture>
}

impl Attachment {
	fn create_from_spec<Id: Copy, F: FnMut(Id) -> u32>(gfx: &super::Gfx, spec: AttachmentSpec<Id>, mut get_id: F) -> Self {
		Self {
			spec: match spec {
				AttachmentSpec::Color(ColorAttachmentSpec {
					format,
					size,
					ops,
					samples,
					resolve,
				}) => AttachmentSpec::Color(ColorAttachmentSpec {
					samples,
					format,
					size, ops,
					resolve: resolve.and_then(|id| Some(get_id(id)))
				}),
				AttachmentSpec::DepthStencil(info) => AttachmentSpec::DepthStencil(info),
				AttachmentSpec::Output(info) => AttachmentSpec::Output(info),
			},
			texture: match spec {
				AttachmentSpec::Color(ColorAttachmentSpec { format, size, samples, .. }) =>
					Some(super::Texture::create_attachment_texture(gfx, format, match size {
						AttachmentSizeSpec::Fixed(extent) => extent,
						AttachmentSizeSpec::Output(_scale) => wgpu::Extent3d {
							width: gfx.config.width,
							height: gfx.config.height,
							depth_or_array_layers: 1
						}
					}, samples)),
				AttachmentSpec::DepthStencil(DepthStencilAttachmentSpec { format, samples, .. }) =>
					Some(super::Texture::create_depth_texture(gfx, format, samples)),
				AttachmentSpec::Output(_) => None
			}
		}
	}
}

pub struct Graph<R: ?Sized> {
	attachments: HashMap<u32, Attachment>,
	passes: Vec<NodeSpec<R, u32, Vec<u32>>>,
}

impl<'a, R> GraphSpec<'a, R> {
	pub fn build(self, gfx: &super::Gfx) -> Graph<R> {
		let mut ids: HashMap<&'a str, u32> = HashMap::new();

		let mut get_id = |id: &'a str| -> u32 {
			if ids.contains_key(id) {
				ids[id]
			} else {
				ids.insert(id, ids.len() as u32);
				ids[id]
			}
		};

		Graph {
			attachments: self.attachments.into_iter().filter_map(|x| *x).map(|(name, val)| 
				(get_id(name), Attachment::create_from_spec(gfx, val, &mut get_id))
			).collect(),
			passes: self.nodes.into_iter().map(|spec| NodeSpec {
				id: get_id(spec.id),
				color_attachments: spec.color_attachments.into_iter().map(|id| get_id(*id)).collect(),
				depth_stencil_attachment: spec.depth_stencil_attachment.and_then(|id| Some(get_id(id))),
				render: spec.render
			}).collect()
		}
	}
}

pub struct GraphRenderContext<'a> {
	pub gfx: &'a super::Gfx,
	pub output: Option<&'a wgpu::TextureView>,
	pub encoder: &'a mut wgpu::CommandEncoder
}

impl<R> Graph<R> {
	// pub fn resize(&mut self, gfx: &super::Gfx) {

	// }

	pub fn render(&self, ctx: &mut GraphRenderContext, renderer: &R) {
		for pass in &self.passes {
			let color_attachments = Vec::from_iter(pass.color_attachments.iter().map(
				|a| Some(wgpu::RenderPassColorAttachment {
					view: self.attachments[a].texture.as_ref().and_then(|t| Some(&t.view)).or_else(|| ctx.output).unwrap(),
					ops: match self.attachments[a].spec {
						AttachmentSpec::Color(ColorAttachmentSpec { ops, .. }) => ops(ctx.gfx),
						AttachmentSpec::Output(OutputAttachmentSpec { ops, .. }) => ops(ctx.gfx),
						_ => panic!("depth-stencil attachment provided in color attachments.") // TODO: move this to the build phase.
					},
					resolve_target: match self.attachments[a].spec {
						AttachmentSpec::Color(ColorAttachmentSpec { resolve, .. }) =>
							resolve.and_then(|id| self.attachments[&id].texture.as_ref().and_then(|t| Some(&t.view)).or_else(|| ctx.output)),
						_ => None
					},
				})
			));

			let depth_stencil_attachment = pass.depth_stencil_attachment.and_then(|ref a| {
				let info = match &self.attachments[a].spec {
					AttachmentSpec::DepthStencil(info) => info,
					_ => panic!("color attachment provided in depth-stencil attachment.") // TODO: move this to the build phase.
				};
				Some(wgpu::RenderPassDepthStencilAttachment {
					view: &self.attachments[a].texture.as_ref().unwrap().view,
					depth_ops: info.depth_ops.and_then(|f| f(ctx.gfx)),
					stencil_ops: info.stencil_ops.and_then(|f| f(ctx.gfx))
				})
			});

			(pass.render)(ctx.gfx, &mut ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: Some(&pass.id.to_string()),
				color_attachments: &color_attachments,
				depth_stencil_attachment
			}), renderer);
		}
	}
}
