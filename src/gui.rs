
struct ScreenRect {
	x: i32,
	y: i32,
	width: i32,
	height: i32
}

struct Rect {
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

struct Primitive {
	clip_rect: Rect,
	index_offset: u32
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct GuiVertex {
	x: f32, y: f32,
	u: f32, v: f32
}

struct Font {

}

struct FontChar {
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
}

struct GuiRenderer {
	screen_width: f32,
	screen_height: f32,
	indices: Vec<u32>,
	vertices: Vec<GuiVertex>,
	primitives: Vec<Primitive>
}

impl GuiRenderer {
	pub fn rect(&mut self, rect: ScreenRect, uvs: Rect) {
		let rect = rect.to_clip(self.screen_width, self.screen_height);
		
		let index_offset = self.vertices.len() as u32;

		self.vertices.extend_from_slice(&[
			GuiVertex { x: rect.x1(), y: rect.y1(), u: uvs.x1(), v: uvs.y1() },
			GuiVertex { x: rect.x2(), y: rect.y1(), u: uvs.x2(), v: uvs.y1() },
			GuiVertex { x: rect.x2(), y: rect.y2(), u: uvs.x2(), v: uvs.y2() },
			GuiVertex { x: rect.x1(), y: rect.y2(), u: uvs.x1(), v: uvs.y2() },
		]);

		self.indices.extend_from_slice(&[
			index_offset + 0,
			index_offset + 1,
			index_offset + 2,
			index_offset + 3,
		]);

		self.primitives.push(Primitive { clip_rect: rect, index_offset })
	}

	pub fn text(&mut self, _rect: ScreenRect, font: &Font, text: &str) {
		let mut x = _rect.x;
		let y = _rect.y;

		for c in text.chars() {
			let ch = font.char(c);
			x += ch.scaled_advance_x(font) as i32;
			self.rect(ScreenRect {
				x, y,
				width: ch.scaled_width(font) as i32,
				height: ch.scaled_height(font) as i32,
			}, ch.uvs);
			// TODO: kerning
		}
	}
}

struct GuiContext {
	
}
