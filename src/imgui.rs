use std::{collections::HashMap, hash::Hasher};


pub struct ScreenRect {
	x: i32,
	y: i32,
	width: i32,
	height: i32
}

pub struct Rect {
	x: f32,
	y: f32,
	width: f32,
	height: f32
}

impl Rect {
	pub fn x1(&self) -> f32 { self.x }
	pub fn y1(&self) -> f32 { self.y }
	pub fn x2(&self) -> f32 { self.x + self.width }
	pub fn y2(&self) -> f32 { self.y + self.height }
}

impl ScreenRect {
	pub fn to_clip(self, screen_width: f32, screen_height: f32) -> Rect {
		Rect {
			x: self.x as f32 / screen_width,
			y: self.y as f32 / screen_height,
			width: self.width as f32 / screen_width,
			height: self.height as f32 / screen_height,
		}
	}
}

pub struct Primitive {
	index_offset: u32,
	vertex_count: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct GuiVertex {
	xy: [f32; 2],
	uv: [f32; 2],
	rgba: [u8; 4]
}

pub struct Font {

}

pub struct FontChar {
	uvs: Rect,
	width: i32,
	height: i32,
	advance_x: i32,
}

impl FontChar {
	pub fn scaled_width(&self, font: &Font) -> f32 { (self.width as f32 * font.scale()).round() }
	pub fn scaled_height(&self, font: &Font) -> f32 { (self.height as f32 * font.scale()).round() }
	pub fn scaled_advance_x(&self, font: &Font) -> f32 { (self.advance_x as f32 * font.scale()).round() }
}

impl Font {
	pub fn scale(&self) -> f32 {
		todo!()
	}

	pub fn char(&self, c: char) -> FontChar {
		todo!()
	}

	pub fn scaled_height(&self) -> i32 {
		todo!()
	}

	pub fn scaled_text_width(&self, text: &str) -> i32 {
		let mut width = 0;
		for ch in text.chars() {
			let ch = self.char(ch);
			width += ch.scaled_advance_x(self) as i32;
		}
		width
	}
}

pub struct Builder {
	screen_width: f32,
	screen_height: f32,
	indices: Vec<u32>,
	vertices: Vec<GuiVertex>,
	primitives: Vec<Primitive>
}

impl Builder {
	pub fn rect(&mut self, rect: ScreenRect, uvs: Rect, rgba: [u8; 4]) {
		let rect = rect.to_clip(self.screen_width, self.screen_height);
		
		let index_offset = self.vertices.len() as u32;

		self.vertices.extend_from_slice(&[
			GuiVertex { xy: [rect.x1(), rect.y1()], uv: [uvs.x1(), uvs.y1()], rgba },
			GuiVertex { xy: [rect.x2(), rect.y1()], uv: [uvs.x2(), uvs.y1()], rgba },
			GuiVertex { xy: [rect.x2(), rect.y2()], uv: [uvs.x2(), uvs.y2()], rgba },
			GuiVertex { xy: [rect.x1(), rect.y2()], uv: [uvs.x1(), uvs.y2()], rgba },
		]);

		self.indices.extend_from_slice(&[
			index_offset + 0,
			index_offset + 1,
			index_offset + 2,
			index_offset + 3,
		]);

		self.primitives.push(Primitive { vertex_count: 4, index_offset })
	}

	pub fn text(&mut self, _rect: ScreenRect, font: &Font, text: &str, rgba: [u8; 4]) {
		let mut x = _rect.x;
		let y = _rect.y;

		for c in text.chars() {
			let ch = font.char(c);
			x += ch.scaled_advance_x(font) as i32;
			self.rect(ScreenRect {
				x, y,
				width: ch.scaled_width(font) as i32,
				height: ch.scaled_height(font) as i32,
			}, ch.uvs, rgba);
			// TODO: kerning
		}
	}
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct WidgetId(u64);

impl From<&str> for WidgetId {
	fn from(value: &str) -> Self {
		let mut hasher = std::collections::hash_map::DefaultHasher::new();
		std::hash::Hash::hash(value, &mut hasher);
		Self(hasher.finish())
	}
}

struct Window {
	rect: ScreenRect,
	title: String
}

impl Window {
	
}

pub struct PanelContext<'builder, 'context> {
	builder: &'builder mut WindowBuilder<'context>,
	x: i32, y: i32,
	last: WidgetId
}

impl<'builder, 'context> PanelContext<'builder, 'context> {
	pub fn label(&mut self, text: &str) {
		let width = self.builder.context.font.scaled_text_width(text) + self.builder.context.style.label_margin.horizontal();
		let height = self.builder.context.font.scaled_height() + self.builder.context.style.label_margin.vertical();

		self.builder.context.builder.text(
			ScreenRect { x: self.x, y: self.y, width, height },
			&self.builder.context.font,
			text,
			[255, 255, 255, 255]
		);
	}
}

pub struct WindowBuilder<'context> {
	id: WidgetId,
	context: &'context mut Context
}

impl<'context> WindowBuilder<'context> {
	pub fn draw<F: FnMut(&Context) -> ()>(&self, mut f: F) {
		f(self.context)
	}
}

enum WidgetState {
	Released,
	JustPressed,
	Pressed,
	JustReleased,
}

struct Widget {
	rect: ScreenRect,
	hovered: bool,
	state: WidgetState
}

pub struct RectSides {
	left: i32,
	right: i32,
	top: i32,
	bottom: i32
}

impl RectSides {
	pub fn horizontal(&self) -> i32 {
		self.left + self.right
	}

	pub fn vertical(&self) -> i32 {
		self.top + self.bottom
	}
}

pub struct Style {
	window_title_margin: RectSides,
	label_margin: RectSides,
}

pub struct Context {
	font: Font,
	builder: Builder,
	windows: HashMap<WidgetId, Window>,
	widget: HashMap<WidgetId, Widget>,
	style: Style,
	screen_width: i32,
	screen_height: i32,
}

impl Context {
	pub fn window<'a>(&'a mut self, title: &str) -> WindowBuilder {
		let id = WidgetId::from(title);

		if !self.windows.contains_key(&id) {
			let width = self.font.scaled_text_width(title) + self.style.window_title_margin.horizontal();
			let height = self.font.scaled_height() + self.style.window_title_margin.vertical();
			self.windows.insert(id, Window {
				rect: ScreenRect {
					x: (self.screen_width - width) / 2,
					y: (self.screen_height - height) / 2,
					width, height
				},
				title: String::from(title)
			});
		}

		WindowBuilder {
			id,
			context: self
		}
	}
}
