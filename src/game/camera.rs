use winit::keyboard::KeyCode;
use crate::{UpdateContext, LoadContext, math::*};

use super::renderer;

#[derive(Debug)]
pub struct CameraController {
	max_speed: f32,
	sensitivity: f32,
	capturing: bool,
	velocity: Vec3f32,
	smooth: bool,
	time_since_last_forward_press: f32,
	sprinting_double_press: bool,
	is_sprinting: bool,
	acceleration: f32
}

impl CameraController {
	pub fn new(speed: f32, sensitivity: f32) -> Self {
		Self {
			max_speed: speed,
			sensitivity,
			capturing: false,
			velocity: Vector([0.0, 0.0, 0.0]),
			smooth: true,
			time_since_last_forward_press: f32::INFINITY,
			sprinting_double_press: false,
			is_sprinting: false,
			acceleration: 50.0
		}
	}

	pub fn load(&mut self, ctx: &mut LoadContext) {
		ctx.window_mut().capture_cursor(true);
		self.capturing = true;
	}

	pub fn update_camera(&mut self, ctx: &mut UpdateContext, camera: &mut renderer::chunk::Camera, dt: f32) -> bool {
		let delta = {
			let mut res = glm::vec3(0.0, 0.0, 0.0);
			if ctx.input().key(KeyCode::KeyD).held() { res.x += 1.0; }
			if ctx.input().key(KeyCode::KeyA).held() { res.x -= 1.0; }
			if ctx.input().key(KeyCode::KeyW).held() { res.z += 1.0; }
			if ctx.input().key(KeyCode::KeyS).held() { res.z -= 1.0; }
			if ctx.input().key(KeyCode::Space).held() { res.y += 1.0; }
			if ctx.input().key(KeyCode::ShiftLeft).held() { res.y -= 1.0; }
			res
		};

		if ctx.input().key(KeyCode::KeyW).just_pressed() {
			if self.time_since_last_forward_press <= 0.2 {
				self.is_sprinting = true;
				self.sprinting_double_press = true;
				self.time_since_last_forward_press = 0.0;
			}
		}

		if ctx.input().key(KeyCode::KeyW).just_released() {
			self.time_since_last_forward_press = 0.0;
			self.sprinting_double_press = false;
			self.is_sprinting = false;
		}

		self.time_since_last_forward_press += dt;

		if ctx.input().key(KeyCode::ControlLeft).held() {
			self.is_sprinting = true;
		}

		if ctx.input().key(KeyCode::ControlLeft).just_released() {
			self.is_sprinting = false || self.sprinting_double_press;
		}

		if ctx.input().key(KeyCode::KeyN).just_pressed() {
			self.smooth = !self.smooth;
		}

		let last_capturing = self.capturing;

		if self.capturing && ctx.window().input().key(KeyCode::Escape).just_pressed() {
			ctx.window_mut().capture_cursor(false);
			self.capturing = false;
		}

		if !self.capturing && ctx.window().input().button(0).just_pressed() {
			ctx.window_mut().capture_cursor(true);
			self.capturing = true;
		}
		
		{
			// let (yaw_sin, yaw_cos) = camera.yaw.sin_cos();
			// let forward = glm::vec3(-yaw_sin, 0.0, yaw_cos).normalize();
			// let right = glm::vec3(-yaw_cos, 0.0, -yaw_sin).normalize();

			let front = Vector([
				camera.yaw.cos(), // * camera.pitch.cos(),
				0.0, // camera.pitch.sin(),
				camera.yaw.sin(), // * camera.pitch.cos(),
			]);

			let right = front.cross(Vector([0.0, 1.0, 0.0]));

			let movement = front * delta.z + right * delta.x + Vector([0.0, delta.y, 0.0]);

			if self.smooth {
				let mul = if self.is_sprinting { 5.0 } else { 1.0 };

				self.velocity += movement * mul * self.acceleration * dt;

				if self.velocity.mag_squared() > mul * self.max_speed * mul * self.max_speed {
					self.velocity = self.velocity.normalized() * self.max_speed * mul;
				}

				self.velocity = self.velocity.lerp((0.0, 0.0, 0.0).vector(), (self.max_speed * 0.5 * dt).min(1.0));
				self.velocity = self.velocity.lerp(Vector([0.0, 0.0, 0.0]), (self.max_speed * 0.5 * dt).min(1.0));
				
				if self.velocity.mag_squared() <= 0.001 {
					self.velocity = Vector([0.0, 0.0, 0.0]);
				}
			} else {
				self.velocity = movement * self.max_speed;
			}

			camera.position += self.velocity * dt;
		}
		
		// camera.fovy += ctx.window().input.scroll_diff() * self.speed * self.sensitivity * dt;
		
		if self.capturing || ctx.input().button(0).held() {
			let (
				rotate_horizontal,
				rotate_vertical
			) = ctx.window().input.mouse_delta().into();

			camera.yaw += rotate_horizontal * self.sensitivity * dt;
			camera.pitch += -rotate_vertical * self.sensitivity * dt;

			// Keep the camera's angle from going too high/low.
			let safe_angle: f32 = 3.141592 * 0.5 - 0.01; // glm::epsilon::<f32>()
	
			camera.pitch = camera.pitch.clamp(-safe_angle, safe_angle);
		}

		last_capturing
	}
}
