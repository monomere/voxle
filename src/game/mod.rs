use std::collections::{HashMap, HashSet};
use noise::NoiseFn;
use winit::keyboard::KeyCode;
use crate::{state::State, gfx, UpdateContext, math::*};

use self::{renderer::GameRenderContext, chunk::Block};

mod chunk;
mod renderer;
mod camera;


#[derive(Debug, Clone, Copy)]
pub enum FaceDirection {
	PX = 0, NX = 1,
	PY = 2, NY = 3,
	PZ = 4, NZ = 5,
}

impl FaceDirection {
	pub const fn all() -> &'static [FaceDirection; FaceDirection::count()] {
		&[Self::PX, Self::NX, Self::PY, Self::NY, Self::PZ, Self::NZ]
	}

	pub fn normal<T: Scalar + From<i32>>(&self) -> Vec3<T> {
		match self {
			Self::PX => vec3(( 1).into(), ( 0).into(), ( 0).into()),
			Self::NX => vec3((-1).into(), ( 0).into(), ( 0).into()),
			Self::PY => vec3(( 0).into(), ( 1).into(), ( 0).into()),
			Self::NY => vec3(( 0).into(), (-1).into(), ( 0).into()),
			Self::PZ => vec3(( 0).into(), ( 0).into(), ( 1).into()),
			Self::NZ => vec3(( 0).into(), ( 0).into(), (-1).into()),
		}
	}

	/// clamps the axis of the normal, leaves other axes.
	pub fn zero_axis<T: Scalar + From<i32>>(
		&self,
		Vector([x, y, z]): Vec3<T>,
		Vector([nx, ny, nz]): Vec3<T>,
		Vector([px, py, pz]): Vec3<T>,
	) -> Vec3<T> {
		match self {
			Self::PX => vec3(px, y, z),
			Self::NX => vec3(nx, y, z),
			Self::PY => vec3(x, py, z),
			Self::NY => vec3(x, ny, z),
			Self::PZ => vec3(x, y, pz),
			Self::NZ => vec3(x, y, nz),
		}
	}

	pub const fn count() -> usize { 6 }
}

pub const CUBE_VERTICES: [[f32; 3]; 8] = [
	// X  /  Y  /  Z //
	[ 0.5,  0.5,  0.5], // 0
	[ 0.5,  0.5, -0.5], // 1
	[-0.5,  0.5, -0.5], // 2
	[-0.5,  0.5,  0.5], // 3
	[ 0.5, -0.5,  0.5], // 4
	[ 0.5, -0.5, -0.5], // 5
	[-0.5, -0.5, -0.5], // 6
	[-0.5, -0.5,  0.5], // 7
];

pub const CUBE_FACES: [(FaceDirection, [usize; 4]); 6] = [
	(FaceDirection::PX, [5, 4, 0, 1]),
	(FaceDirection::NX, [7, 6, 2, 3]),
	(FaceDirection::PY, [3, 2, 1, 0]),
	(FaceDirection::NY, [4, 5, 6, 7]),
	(FaceDirection::PZ, [4, 7, 3, 0]),
	(FaceDirection::NZ, [6, 5, 1, 2]),
];
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

	fn grass_dirt_stone(y: i32, top_y: i32) -> u16 {
		if y == top_y {
			chunk::BlockId::GRASS as u16
		} else if top_y - y < 5 {
			chunk::BlockId::DIRT as u16
		} else {
			chunk::BlockId::STONE as u16
		}
	}

	fn generate_chunk(&self, chunk_pos: Vec3i32) -> Option<chunk::Chunk> {
		let mut chunk = chunk::Chunk::new(chunk_pos, chunk::ChunkData::new());
		let chunk_pos_block = chunk_pos * chunk::CHUNK_SIZE.each_as::<i32>();

		if chunk_pos.y < -1 && chunk_pos.y > -4 {
			for y in 0..chunk::CHUNK_SIZE.y as i32 {
				for z in 0..chunk::CHUNK_SIZE.z as i32 {
					for x in 0..chunk::CHUNK_SIZE.x as i32 {
						chunk.data.set_block(vec3(x, y, z), chunk::Block {
							id: if chunk_pos.y == -2 {
								Self::grass_dirt_stone(y, chunk::CHUNK_SIZE.y as i32)
							} else {
								chunk::BlockId::STONE as u16
							},
							state: 0
						});
					}
				}
			}
			return Some(chunk);
		}

		for z in 0..chunk::CHUNK_SIZE.z as i32 {
			for x in 0..chunk::CHUNK_SIZE.x as i32 {
				let abs_y = self.get_height_at(chunk_pos_block.xz() + vec2(x, z));

				let (chunk_y, loc_y) = num::integer::div_mod_floor(abs_y, chunk::CHUNK_SIZE.y as i32);

				// check if we're in our chunk.
				if chunk_y >= chunk_pos.y {
					let (loc_y, top_y) = if chunk_y > chunk_pos.y {
						(chunk::CHUNK_SIZE.y as i32 - 1, -1)
					} else {
						(loc_y.abs(), loc_y.abs())
					};

					for y in 0..=loc_y {
						chunk.data.set_block(vec3(x, y, z), chunk::Block {
							id: Self::grass_dirt_stone(y, top_y),
							state: 0
						});
					}
				}
			}	
		}

		Some(chunk)
	}
}

/// world -> chunk position (in chunks)
pub fn world_to_map(world: Vec3f32) -> Vec3i32 {
	world.zip_map(chunk::CHUNK_SIZE, |world, chunk| num::integer::div_floor(world.round() as i32, chunk as i32))
}

/// world -> block position (in blocks)
pub fn world_to_chunk_local(world: Vec3f32) -> Vec3i32 {
	world.zip_map(chunk::CHUNK_SIZE, |world, chunk| num::integer::mod_floor(world.round() as i32, chunk as i32))
}

#[derive(Debug, Clone, Copy)]
struct BlockTarget {
	/// chunk position (in chunks)
	chunk: Vec3i32,
	// block position local to chunk
	block: Vec3i32,
	face: Vec3f32
}

impl BlockTarget {
	pub fn to_global(self) -> Vec3i32 {
		self.chunk * chunk::CHUNK_SIZE.each_as() + self.block
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
	worldgen: WorldGen,
	target_block_position: Option<BlockTarget>,
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
			worldgen: WorldGen::new(69),
			target_block_position: None,
		}
	}

	fn update_chunk_quick(&mut self, gfx: &gfx::Gfx, pos: Vec3i32) {
		let neighbors = FaceDirection::all().clone().map(|dir| {
			let normal = dir.normal::<i32>();
			self.chunks.get(&(pos + normal)).and_then(|c| Some(chunk::UnsafeChunkDataRef::new(&*c.data)))
		});

		self.chunks.get_mut(&pos).unwrap().update_mesh(
			gfx,
			&neighbors,
			self.renderer.chunk_renderer.texture_size()
		);
	}

	fn update_chunk(&mut self, gfx: &gfx::Gfx, pos: Vec3i32) {
		self.update_chunk_quick(gfx, pos);

		let to_update: Vec<Vec3i32> = FaceDirection::all().iter().filter_map(|dir| {
			let normal = dir.normal::<i32>();
			self.chunks.contains_key(&(pos + normal)).then_some(pos + normal)
		}).collect();

		for pos in to_update {
			self.update_chunk_quick(gfx, pos);
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
								for dir in FaceDirection::all() {
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
				self.update_chunk_quick(gfx, pos);
			}
		}
	}

	fn raycast_target(&mut self) {
		let dir = self.renderer.chunk_renderer.camera.direction().normalized();
		let step = 0.1;
		let mut len = 0.0;
		let mut ray = self.renderer.chunk_renderer.camera.position.clone();
		let mut found = false;
		let one_over_dir = dir.map(|c| 1.0 / c);
		while len < 16.0 {
			let chunk_position = world_to_map(ray);
			let block_position = world_to_chunk_local(ray);
			if let Some(chunk) = &self.chunks.get(&chunk_position) {
				if chunk.data.get_block(block_position).is_some_and(|b| b.is_solid()) {
					// https://www.shadertoy.com/view/ld23DV (Inigo Quilez)
					let ro = (ray - dir * step) - chunk.position.each_as() - block_position.each_as();
					let n = one_over_dir * ro;
					let k = one_over_dir.abs() * 0.5;
					let t1 = -n - k;
					let t_n = t1.x.max(t1.y).max(t1.z);
					let normal = t1.step(t_n);
					self.target_block_position = Some(BlockTarget {
						chunk: chunk_position,
						block: block_position,
						face: normal
					});
					found = true;
					break;
				}
			}
			len += step;
			ray += dir * step;
		}
		if !found {
			self.target_block_position = None;
		}
	}
}

impl State for GameState {
	fn load(&mut self, context: &mut crate::LoadContext) {
		self.camera_controller.load(context);
		self.generate_chunks(&context.gfx);
		self.renderer.chunk_renderer.set_sun_direction(vec4::<f32>(4.0, -5.0, 5.0, 1.0).normalized());
	}

	fn update(&mut self, context: &mut UpdateContext) {
		self.camera_controller.update_camera(context, &mut self.renderer.chunk_renderer.camera, context.dt);
		
		let last_chunk_position = self.current_chunk_position;

		self.current_chunk_position = world_to_map(self.renderer.chunk_renderer.camera.position);

		if last_chunk_position != self.current_chunk_position {
			self.generate_chunks(context.gfx);
		}

		if context.input().key(KeyCode::KeyG).just_pressed() {
			self.render_wireframe = !self.render_wireframe;
		}

		for (id, keycode) in [KeyCode::Digit0, KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3].into_iter().enumerate() {
			if context.input().key(keycode).just_pressed() {
				let loc_block_pos = world_to_chunk_local(self.renderer.chunk_renderer.camera.position);

				{
					let chunk = self.chunks.get_mut(&self.current_chunk_position).unwrap();
					chunk.data.set_block(loc_block_pos, Block {
						id: id as u16,
						state: 0
					});
				}

				self.update_chunk(&context.gfx, self.current_chunk_position);
			}
		}

		self.raycast_target();
		self.renderer.update();
	}
	
	fn render<'a>(&'a self, context: &mut gfx::RenderContext<'a>) {
		self.renderer.render(context, self);
	}

	fn ui<'a>(&'a self, ctx: &egui::Context) {
		egui::Window::new("debug").default_open(false).show(ctx, |ui| {
			ui.label(format!("chunk: {}", self.current_chunk_position));
			ui.label(format!("eye: {}", self.renderer.chunk_renderer.camera.position));
			
			let loc_block_pos = world_to_chunk_local(self.renderer.chunk_renderer.camera.position);

			let block = self.chunks[&self.current_chunk_position].data.get_block(loc_block_pos);

			ui.label(format!("block: {:?}", block.map(|b| b.id)));
			ui.label(format!("   at: {:?}", loc_block_pos));
			ui.label(format!("target: {:?}", self.target_block_position));
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

	fn on_render<'a>(&'a self, gfx: &gfx::Gfx, ctx: &mut GameRenderContext<'a, '_>) {
		{
			let mut chunk_ctx = ctx.begin_chunk_context(gfx);
			
			chunk_ctx.set_mode(renderer::chunk::ChunkRenderMode::Normal);
			self.render_chunks(&mut chunk_ctx);

			if self.render_wireframe {
				chunk_ctx.set_mode(renderer::chunk::ChunkRenderMode::Wireframe);
				self.render_chunks(&mut chunk_ctx);
			}

			if let Some(position) = self.target_block_position {
				chunk_ctx.render_outline(position.to_global().each_as())
			}
		}
	}
}
