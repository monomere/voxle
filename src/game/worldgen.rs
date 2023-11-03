use noise::NoiseFn;
use crate::math::*;
use super::chunk;


pub struct WorldGen {
	noise: noise::Perlin // noise::Fbm<noise::Perlin>
}

impl WorldGen {
	pub fn new(seed: u32) -> Self {
		Self {
			noise: noise::Perlin::new(seed)
		}
	}

	fn get_height_at(&self, pos: Vec2i32) -> i32 {
		((self.noise.get((pos.each_as::<f64>() * 0.01).0) * 2.0 - 1.0) * chunk::CHUNK_SIZE.y as f64) as i32
	}

	#[allow(dead_code)]
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
			chunk::BlockId::Grass as u16
		} else if top_y - y < 5 {
			chunk::BlockId::Dirt as u16
		} else {
			chunk::BlockId::Stone as u16
		}
	}

	pub fn generate_chunk(&self, chunk_pos: Vec3i32) -> Option<chunk::Chunk> {
		let mut chunk = chunk::Chunk::new(chunk_pos, chunk::ChunkData::new());
		let chunk_pos_block = chunk_pos * chunk::CHUNK_SIZE.each_as::<i32>();

		if chunk_pos.y < -1 {
			for y in 0..chunk::CHUNK_SIZE.y as i32 {
				for z in 0..chunk::CHUNK_SIZE.z as i32 {
					for x in 0..chunk::CHUNK_SIZE.x as i32 {
						chunk.data.set_block(vec3(x, y, z), chunk::Block {
							id: if chunk_pos.y == -2 {
								Self::grass_dirt_stone(y, chunk::CHUNK_SIZE.y as i32 - 1)
							} else {
								chunk::BlockId::Stone as u16
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

