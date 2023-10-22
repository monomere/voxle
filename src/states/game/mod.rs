use std::collections::HashMap;
use noise::NoiseFn;
use crate::{scene::State, gfx, UpdateContext};

mod chunk;
mod renderer;
mod camera;


struct WorldGen {
	noise: noise::Fbm<noise::Perlin>
}

impl WorldGen {
	fn new(seed: u32) -> Self {
		Self {
			noise: noise::Fbm::new(seed)
		}
	}

	fn generate_chunk(&self, position: [i32; 3]) -> Option<chunk::Chunk> {
		let mut chunk = chunk::Chunk::new(position, chunk::ChunkData::new());

		for z in 0..chunk::CHUNK_SIZE.2 as i32 {
			for x in 0..chunk::CHUNK_SIZE.0 as i32 {

				let raw_y = (self.noise.get([
					(position[0] * chunk::CHUNK_SIZE.0 as i32 + x) as f64 * 0.01,
					(position[2] * chunk::CHUNK_SIZE.2 as i32 + z) as f64 * 0.01
				]) * chunk::CHUNK_SIZE.1 as f64) as i32;

				// check if we're in our chunk.
				if raw_y / chunk::CHUNK_SIZE.1 as i32 == position[1] {

					// chunk-local position.
					let top_y = raw_y - chunk::CHUNK_SIZE.1 as i32 * position[1];

					for y in 0..top_y {
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
	worldgen: WorldGen
}

impl GameState {
	pub fn new(gfx: &gfx::Gfx) -> Self {
		let _world = shipyard::World::new();
		
		Self {
			_world,
			chunks: HashMap::new(),
			renderer: renderer::GameRenderer::new(gfx),
			camera_controller: camera::CameraController::new(10.0, 2.0),
			render_distance: 8,
			current_chunk_position: (0, 0, 0),
			worldgen: WorldGen::new(69)
		}
	}

	fn generate_chunks(&mut self, gfx: &gfx::Gfx) {
		let mut to_be_updated = Vec::new();

		let half_rd = self.render_distance / 2;
		let squared_rd = self.render_distance * self.render_distance;

		for x in -half_rd..half_rd {
			for y in -half_rd..half_rd {
				for z in -half_rd..half_rd {
					let (cx, cy, cz) = (
						self.current_chunk_position.0 + x,
						self.current_chunk_position.1 + y,
						self.current_chunk_position.2 + z
					);
					if x*x + y*y + z*z < squared_rd {
						if !self.chunks.contains_key(&(cx, cy, cz)) {
							if let Some(chunk) = self.worldgen.generate_chunk([cx, cy, cz]) {
								self.chunks.insert((cx, cy, cz), chunk);
								to_be_updated.push((cx, cy, cz));

								// update neighbor meshes
								for dir in chunk::FaceDirection::all() {
									let (dx, dy, dz) = dir.normal::<i32>();
									let (x, y, z) = (x + dx, y + dy, z + dz);
									if self.chunks.contains_key(&(x, y, z)) {
										to_be_updated.push((x, y, z))
									}
								}
							}
						}
					} else {
						if self.chunks.contains_key(&(cx, cy, cz)) {
							self.chunks.remove(&(cx, cy, cz));
						}
					}
				}
			}	
		}

		for (x, y, z) in to_be_updated {
			let chunks = &mut self.chunks;

			let neighbors = chunk::FaceDirection::all().clone().map(|dir| {
				let (dx, dy, dz) = dir.normal::<i32>();
				chunks.get(&(x + dx, y + dy, z + dz)).and_then(|c| Some(chunk::ChunkDataRef::new(&*c.data)))
			});

			chunks.get_mut(&(x, y, z)).unwrap().update_mesh(
				gfx,
				&neighbors
			);
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
				p.x as i32 / chunk::CHUNK_SIZE.0 as i32,
				p.y as i32 / chunk::CHUNK_SIZE.1 as i32,
				p.z as i32 / chunk::CHUNK_SIZE.2 as i32,
			)
		};
		if last_chunk_position != self.current_chunk_position {
			self.generate_chunks(context.gfx);
		}
	}
	
	fn render<'a>(&'a self, context: &mut gfx::RenderContext<'a>) {
		self.renderer.render(context, self);
	}
}
