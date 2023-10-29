#![feature(trait_alias)]
#![feature(const_trait_impl)]

use state::State;
use winit::{
	event::*,
	event_loop::EventLoop,
	window::{WindowBuilder, CursorGrabMode}, keyboard::{KeyCode, PhysicalKey}, dpi::LogicalPosition
};

mod gfx;
mod state;
mod game;
mod polyfill;
mod imgui;
mod math;

extern crate nalgebra_glm as glm;

#[derive(Debug, Clone, Copy, PartialEq)]
enum InputElementState {
	None,
	JustPressed,
	Held,
	JustReleased,
}

impl InputElementState {
	fn from_old_new(old: InputElementState, new: ElementState) -> Self {
		if old.held() {
			match new {
				ElementState::Pressed => Self::Held,
				ElementState::Released => Self::JustReleased
			}
		} else {
			match new {
				ElementState::Pressed => Self::JustPressed,
				ElementState::Released => Self::None
			}
		}
	}

	pub fn held(&self) -> bool {
		*self == Self::JustPressed || *self == Self::Held
	}

	#[allow(dead_code)]
	pub fn not_held(&self) -> bool {
		*self == Self::JustReleased || *self == Self::JustPressed
	}

	pub fn just_pressed(&self) -> bool {
		*self == Self::JustPressed
	}

	pub fn just_released(&self) -> bool {
		*self == Self::JustReleased
	}
}

#[derive(Clone, Copy)]
pub struct Input {
	keys: [InputElementState; 256],
	buttons: [InputElementState; 256],
	mouse_delta: (f32, f32),
	close_requested: bool,
}

impl Input {
	fn new() -> Self {
		Self {
			keys: [InputElementState::None; 256],
			buttons: [InputElementState::None; 256],
			mouse_delta: (0.0, 0.0),
			close_requested: false,
		}
	}

	fn reset_deltas(&mut self) {
		self.mouse_delta = (0.0, 0.0);
		for key in &mut self.keys {
			*key = match *key {
				InputElementState::JustPressed => InputElementState::Held,
				InputElementState::JustReleased => InputElementState::None,
				_ => *key
			}
		}
		for button in &mut self.buttons {
			*button = match *button {
				InputElementState::JustPressed => InputElementState::Held,
				InputElementState::JustReleased => InputElementState::None,
				_ => *button
			}
		}
	}

	fn process_event(&mut self, event: &winit::event::Event<()>) -> bool {
		match event {
			Event::DeviceEvent { event, .. } => {
				match *event {
					DeviceEvent::MouseMotion { delta } => {
						self.mouse_delta = (delta.0 as f32, delta.1 as f32);
						true
					},
					_ => false
				}
			}
			Event::WindowEvent { event, .. } => match event {
				WindowEvent::CloseRequested => {
					self.close_requested = true;
					true
				},
				// WindowEvent::CursorMoved { position, .. } => {
				// 	let cur_mouse_pos = (position.x as f32, position.y as f32);
				// 	self.mouse_delta = (cur_mouse_pos.0 - self.last_mouse_pos.0, cur_mouse_pos.1 - self.last_mouse_pos.1);
				// 	self.last_mouse_pos = cur_mouse_pos;
				// 	true
				// },
				WindowEvent::MouseInput { state, button, .. } => {
					let index = match button {
						MouseButton::Left => 0,
						MouseButton::Right => 1,
						MouseButton::Middle => 2,
						MouseButton::Back => 3,
						MouseButton::Forward => 4,
						MouseButton::Other(index) => *index,
					} as usize;

					self.buttons[index] = InputElementState::from_old_new(self.buttons[index], *state);
					true
				}
				WindowEvent::KeyboardInput {
					event: KeyEvent { physical_key: PhysicalKey::Code(key), state, repeat: false, .. },
					..
				} => {
					self.keys[*key as usize] = InputElementState::from_old_new(self.keys[*key as usize], *state);
					true
				}
				_ => false
			}
			_ => false
		}
	}

	fn key(&self, key: KeyCode) -> InputElementState {
		self.keys[key as usize]
	}

	fn button(&self, button: u32) -> InputElementState {
		self.buttons[button as usize]
	}

	pub fn close_requested(&self) -> bool {
		self.close_requested
	}

	pub fn mouse_delta(&self) -> (f32, f32) {
		self.mouse_delta
	}
}

pub struct Window {
	input: Box<Input>,
	window: winit::window::Window,
	capture_cursor: bool
}

impl Window {
	pub fn input(&self) -> &Input {
		&self.input
	}

	pub fn window(&self) -> &winit::window::Window {
		&self.window
	}

	pub fn capture_cursor(&mut self, capture: bool) {
		self.window.set_cursor_visible(!capture);

		if capture {
			self.window.set_cursor_grab(CursorGrabMode::Confined)
				.or_else(|_e| self.window.set_cursor_grab(CursorGrabMode::Locked))
				.unwrap();
		} else {
			self.window.set_cursor_grab(CursorGrabMode::None).unwrap();
		}

		self.capture_cursor = capture;
	}
}

pub struct UpdateContext<'a> {
	gfx: &'a mut gfx::Gfx,
	pub dt: f32,
}

impl UpdateContext<'_> {
	fn input(&self) -> &Input { self.window().input() }
	fn window(&self) -> &Window { self.gfx.window() }
	fn window_mut(&mut self) -> &mut Window { self.gfx.window_mut() }
}

pub struct LoadContext<'a> {
	gfx: &'a mut gfx::Gfx,
}

impl LoadContext<'_> {
	#[allow(dead_code)]
	fn window(&self) -> &Window { self.gfx.window() }
	fn window_mut(&mut self) -> &mut Window { self.gfx.window_mut() }
}

async fn run() {
	env_logger::init();
	
	let event_loop = EventLoop::new().unwrap();
	
	let mut gfx = Box::new(gfx::Gfx::new(Window {
		input: Box::new(Input::new()),
		window: WindowBuilder::new().build(&event_loop).unwrap(),
		capture_cursor: false
	}).await);

	
	let mut state = game::GameState::new(&gfx);

	{
		let mut context = LoadContext { gfx: &mut gfx };
		state.load(&mut context);
	}

	event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
	
	let event_loop_start = std::time::Instant::now();
	let mut last_render_time = std::time::Instant::now();
	event_loop.run(move |event, elwt| {
		if !gfx.window.capture_cursor {
			gfx.egui_platform.handle_event(&gfx.window.window, &event);
			if gfx.egui_platform.captures_event(&event) {
				return;
			}
		}

    if gfx.window_mut().input.process_event(&event) {
			return;
		}

		if gfx.window().input().close_requested() {
			elwt.exit();
		}
		
		match event {
			Event::WindowEvent {
				window_id,
				ref event
			} if window_id == gfx.window().window.id() => {
				match event {
					WindowEvent::Resized(physical_size) => {
						gfx.resize(*physical_size);
					}
					WindowEvent::ScaleFactorChanged { .. } => {
						gfx.resize(gfx.window.window.inner_size());
					}
					WindowEvent::CursorMoved { .. } if gfx.window().capture_cursor => {
						// TODO: do we need this?
						let outer_size = gfx.window().window.outer_size();
						let pos = LogicalPosition::new(outer_size.width / 2, outer_size.height / 2);
						let _ = gfx.window().window.set_cursor_position(pos);
					}
					_ => {}
				}
			}
			Event::AboutToWait => {
				elwt.set_control_flow(winit::event_loop::ControlFlow::Poll);
				let now = std::time::Instant::now();
				let dt = now - last_render_time;
				last_render_time = now;
				
				{
					let mut context = UpdateContext {
						gfx: &mut gfx,
						dt: dt.as_secs_f32()
					};
					
					state.update(&mut context);
				}

				gfx.egui_platform.update_time(event_loop_start.elapsed().as_secs_f64());
				
				match gfx.render(&state) {
					Ok(_) => {}
					Err(wgpu::SurfaceError::Lost) => gfx.resize(gfx.size()),
					Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
					Err(e) => eprintln!("{:?}", e),
				}

				gfx.window_mut().input.reset_deltas();
				gfx.window().window.request_redraw();
			}
			_ => {}
		}
	}).unwrap();
}

fn main() {
	pollster::block_on(run());
}
