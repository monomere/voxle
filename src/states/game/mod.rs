use std::collections::{HashMap, HashSet};
use noise::NoiseFn;
use winit::keyboard::KeyCode;
use crate::{state::State, gfx, UpdateContext, math::*};

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

	fn get_height_at(&self, pos: Vec2i32) -> i32 {
		((self.noise.get((pos.each_as::<f64>() * 0.01).0) * 2.0 - 1.0) * chunk::CHUNK_SIZE.y as f64) as i32
	}

	fn fill_chunk(chunk: &mut chunk::Chunk, block: chunk::Block) {
		for y in 0..chunk::CHUNK_SIZE.y as i32 {
			for z in 0..chunk::CHUNK_SIZE.z as i32 {
				for x in 0..chunk::CHUNK_SIZE.x as i32 {
					chunk.data.set_block(vec3(x, y, z), block);
				}
			}
		}
	}

	fn generate_chunk(&self, chunk_pos: Vec3i32) -> Option<chunk::Chunk> {
		let mut chunk = chunk::Chunk::new(chunk_pos, chunk::ChunkData::new());
		let chunk_pos_block = chunk_pos * chunk::CHUNK_SIZE.each_as::<i32>();

		if chunk_pos.y < -1 && chunk_pos.y > -4 {
			Self::fill_chunk(&mut chunk, chunk::Block {
				id: 1,
				state: 0
			});
			return Some(chunk);
		}

		for z in 0..chunk::CHUNK_SIZE.z as i32 {
			for x in 0..chunk::CHUNK_SIZE.x as i32 {
				let abs_y = self.get_height_at(chunk_pos_block.xz() + vec2(x, z));

				let (chunk_y, loc_y) = num::integer::div_mod_floor(abs_y, chunk::CHUNK_SIZE.y as i32);

				// check if we're in our chunk.
				if chunk_y == chunk_pos.y {
					let loc_y = loc_y.abs();
					for y in 0..=loc_y {
						chunk.data.set_block(vec3(x, y, z), chunk::Block {
							id: (rand::random::<f32>() * 100.0) as u16 + 1,
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
	chunks: HashMap<Vec3i32, chunk::Chunk>,
	renderer: renderer::GameRenderer,
	camera_controller: camera::CameraController,
	render_distance: i32,
	current_chunk_position: Vec3i32,
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
			current_chunk_position: (0, 0, 0).vector(),
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
					let pos = vec3(x, y, z);
					let abs_pos = self.current_chunk_position + pos;
					if x*x + y*y + z*z < squared_rd {
						saved_chunks.insert(abs_pos);
						if !self.chunks.contains_key(&abs_pos) {
							if let Some(chunk) = self.worldgen.generate_chunk(abs_pos) {
								self.chunks.insert(abs_pos, chunk);
								to_be_updated.insert(abs_pos);

								// update neighbor meshes
								for dir in chunk::FaceDirection::all() {
									let normal = dir.normal::<i32>();
									let abs_pos = abs_pos + normal;
									if self.chunks.contains_key(&abs_pos) {
										to_be_updated.insert(abs_pos);
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

		for pos in to_be_updated {
			if self.chunks.contains_key(&pos) {
				let neighbors = chunk::FaceDirection::all().clone().map(|dir| {
					let normal = dir.normal::<i32>();
					self.chunks.get(&(pos + normal)).and_then(|c| Some(chunk::UnsafeChunkDataRef::new(&*c.data)))
				});

				self.chunks.get_mut(&pos).unwrap().update_mesh(
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

		self.current_chunk_position = self.renderer.camera.position.each_as::<i32>()
			.zip_map(chunk::CHUNK_SIZE.each_as::<i32>(), |a, b| num::integer::div_floor(a, b));

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
			ui.label(format!("chunk: {}", self.current_chunk_position));
			ui.label(format!("eye: {}", self.renderer.camera.position));
			
			let loc_block_pos = self.renderer.camera.position.each_as::<i32>()
				.zip_map(chunk::CHUNK_SIZE.each_as::<i32>(), |a, b| num::integer::mod_floor(a, b));

			let block = self.chunks[&self.current_chunk_position].data.get_block(loc_block_pos);

			ui.label(format!("block: {:?}", block.map(|b| b.id)));
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
