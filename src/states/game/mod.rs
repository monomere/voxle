use std::collections::{HashMap, HashSet};
use noise::NoiseFn;
use winit::keyboard::KeyCode;
use crate::{state::State, gfx, UpdateContext, math::{Vec3i32, Vector}};

use self::renderer::GameRenderContext;

mod chunk;
mod renderer;
mod camera;


struct WorldGen {
	noise: noise::Perlin // noise::Fbm<noise::Perlin>
}

impl WorldGen {
	fn new(seed: u32) -> Self {
		Self {
			noise: noise::Perlin::new(seed)
		}
	}

	fn generate_chunk(&self, position: Vec3i32) -> Option<chunk::Chunk> {
		let mut chunk = chunk::Chunk::new(position, chunk::ChunkData::new());

		if position == Vector([0, 0, 0]) {
			for y in 0..chunk::CHUNK_SIZE.y as i32 {
				for z in 0..chunk::CHUNK_SIZE.z as i32 {
					for x in 0..chunk::CHUNK_SIZE.x as i32 {
						chunk.data.set_block(x, y, z, chunk::Block {
							id: 1,
							state: 0
						});
					}
				}
			}
			return Some(chunk);
		}

		for z in 0..chunk::CHUNK_SIZE.z as i32 {
			for x in 0..chunk::CHUNK_SIZE.x as i32 {

				let raw_y = (self.noise.get([
					(position.x * chunk::CHUNK_SIZE.x as i32 + x) as f64 * 0.01,
					(position.z * chunk::CHUNK_SIZE.z as i32 + z) as f64 * 0.01
				]) * chunk::CHUNK_SIZE.y as f64) as i32;

				// check if we're in our chunk.
				if raw_y / chunk::CHUNK_SIZE.y as i32 == position.y {

					// chunk-local position.
					let top_y = raw_y - chunk::CHUNK_SIZE.y as i32 * position.y;

					for y in 0..=top_y {
						chunk.data.set_block(x, y, z, chunk::Block {
							id: (rand::random::<f32>() * 10.0) as u16 + 1,
							state: 0
						});
					}

				}
			}	
		}

		Some(chunk)
	}
}

pub struct GameState {
	_world: shipyard::World,
	chunks: HashMap<(i32, i32, i32), chunk::Chunk>,
	renderer: renderer::GameRenderer,
	camera_controller: camera::CameraController,
	render_distance: i32,
	current_chunk_position: (i32, i32, i32),
	render_wireframe: bool,
	worldgen: WorldGen
}

impl GameState {
	pub fn new(gfx: &gfx::Gfx) -> Self {
		let _world = shipyard::World::new();
		
		Self {
			_world,
			chunks: HashMap::new(),
			renderer: renderer::GameRenderer::new(gfx),
			camera_controller: camera::CameraController::new(10.0, 1.0),
			render_distance: 8,
			current_chunk_position: (0, 0, 0),
			render_wireframe: false,
			worldgen: WorldGen::new(69)
		}
	}

	fn generate_chunks(&mut self, gfx: &gfx::Gfx) {
		let mut to_be_updated = HashSet::new();
		let mut saved_chunks = HashSet::new();

		let half_rd = self.render_distance / 2;
		let squared_rd = self.render_distance * self.render_distance;

		for x in -half_rd - 1 .. half_rd + 1 {
			for y in -half_rd - 1 .. half_rd + 1 {
				for z in -half_rd - 1 .. half_rd + 1 {
					let (cx, cy, cz) = (
						self.current_chunk_position.0 + x,
						self.current_chunk_position.1 + y,
						self.current_chunk_position.2 + z
					);
					if x*x + y*y + z*z < squared_rd {
						saved_chunks.insert((cx, cy, cz));
						if !self.chunks.contains_key(&(cx, cy, cz)) {
							if let Some(chunk) = self.worldgen.generate_chunk(Vector([cx, cy, cz])) {
								self.chunks.insert((cx, cy, cz), chunk);
								to_be_updated.insert((cx, cy, cz));

								// update neighbor meshes
								for dir in chunk::FaceDirection::all() {
									let (dx, dy, dz) = dir.normal::<i32>();
									let (x, y, z) = (x + dx, y + dy, z + dz);
									if self.chunks.contains_key(&(x, y, z)) {
										to_be_updated.insert((x, y, z));
									}
								}
							}
						}
					}
				}
			}	
		}

		{
			let mut to_be_removed = Vec::with_capacity(self.chunks.len() - saved_chunks.len());
			for (position, _) in &self.chunks {
				if !saved_chunks.contains(position) {
					to_be_removed.push(*position);
				}
			}

			for position in to_be_removed {
				self.chunks.remove(&position);
			}
		}

		for (x, y, z) in to_be_updated {
			if self.chunks.contains_key(&(x, y, z)) {
				let neighbors = chunk::FaceDirection::all().clone().map(|dir| {
					let (dx, dy, dz) = dir.normal::<i32>();
					self.chunks.get(&(x + dx, y + dy, z + dz)).and_then(|c| Some(chunk::UnsafeChunkDataRef::new(&*c.data)))
				});

				self.chunks.get_mut(&(x, y, z)).unwrap().update_mesh(
					gfx,
					&neighbors
				);
			}
		}
	}
}

impl State for GameState {
	fn load(&mut self, context: &mut crate::LoadContext) {
		self.camera_controller.load(context);
		self.generate_chunks(&context.gfx);
	}

	fn update(&mut self, context: &mut UpdateContext) {
		self.camera_controller.update_camera(context, &mut self.renderer.camera, context.dt);
		
		let last_chunk_position = self.current_chunk_position;

		self.current_chunk_position = {
			let p = self.renderer.camera.position;
			(
				p.x as i32 / chunk::CHUNK_SIZE.x as i32,
				p.y as i32 / chunk::CHUNK_SIZE.y as i32,
				p.z as i32 / chunk::CHUNK_SIZE.z as i32,
			)
		};

		if last_chunk_position != self.current_chunk_position {
			self.generate_chunks(context.gfx);
		}

		if context.input().key(KeyCode::KeyG).just_pressed() {
			self.render_wireframe = !self.render_wireframe;
		}
	}
	
	fn render<'a>(&'a self, context: &mut gfx::RenderContext<'a>) {
		self.renderer.render(context, self);
	}

	fn ui<'a>(&'a self, ctx: &egui::Context) {
		egui::Window::new("debug").show(ctx, |ui| {
			ui.label(format!("chunk: {:?}", self.current_chunk_position));
		});
	}
}

impl GameState {
	// i have to have the lifetimes like this, otherwise ctx.render_chunk(chunk) doesn't work.
	// (and there's only one way to have lifetimes in ctx.render_chunk)
	fn render_chunks<'a, 'b>(&'a self, ctx: &mut renderer::chunk::ChunkRenderContext<'a, 'b>) {
		for (_, chunk) in &self.chunks {
			ctx.render_chunk(chunk);
		}
	}

	fn on_render<'a>(&'a self, ctx: &mut GameRenderContext<'a, '_>) {
		self.render_chunks(&mut ctx.chunk_context(renderer::chunk::ChunkRenderMode::Normal));

		if self.render_wireframe {
			self.render_chunks(&mut ctx.chunk_context(renderer::chunk::ChunkRenderMode::Wireframe));
		}
	}
}
