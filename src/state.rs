use crate::{gfx, UpdateContext, LoadContext};

pub trait State {
	fn load(&mut self, _context: &mut LoadContext) {}
	fn update(&mut self, _context: &mut UpdateContext) {}
	fn render<'a>(&'a self, _context: &mut gfx::RenderContext<'a>) { }
	// fn ui(&self, _context: &mut gfx::UiContext) { }
	fn ui(&self, _context: &egui::Context) { }
}

pub struct StateStack {
	stack: Vec<Box<dyn State>>
}

#[allow(dead_code)] // XXX: use
impl StateStack {
	pub fn new() -> Self {
		Self { stack: Vec::new() }
	}

	pub fn push(&mut self, state: Box<dyn State>, _context: &LoadContext) {
		self.stack.push(state)
		// TODO: self.stack.top.load()?
	}

	pub fn pop(&mut self) -> Option<Box<dyn State>> {
		self.stack.pop()
	}
}

impl State for StateStack {
	fn load(&mut self, context: &mut LoadContext) {
		if let Some(top) = self.stack.last_mut() {
			top.load(context)
		}
	}

	fn update<'a>(&mut self, context: &mut UpdateContext<'a>) {
		if let Some(top) = self.stack.last_mut() {
			top.update(context)
		}
	}

	fn render<'a>(&'a self, context: &mut gfx::RenderContext<'a>) {
		if let Some(top) = self.stack.last() {
			top.render(context)
		}
	}

	fn ui(&self, context: &egui::Context) {
		if let Some(top) = self.stack.last() {
			top.ui(context)
		}
	}
}
