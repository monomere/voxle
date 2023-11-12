use noise::NoiseFn;
use crate::math::*;
use super::chunk::{self, CHUNK_SIZE, Block, BlockId};


// Process:
// 1. voronoi noise for mountain ranges
// 2. climate zone (can be "ocean")
// 3. biome (can be "lake")
// 4. river
// 5. terrain height (includes mountain ranges)
// 6. cave
// 7. block


pub struct WorldGen {
	noise: noise::Fbm<noise::Perlin>,
	noise2: noise::RidgedMulti<noise::Perlin>
}

impl WorldGen {
	pub fn new(seed: u32) -> Self {
		Self {
			noise: noise::Fbm::new(seed),
			noise2: noise::RidgedMulti::new(seed),
		}
	}

	fn get_height(&self, world_pos: Vec2i32) -> i32 {
		let h = self.noise.get((world_pos.each_as() * 0.001).0) * 64.0;
		let h2 = self.noise2.get((world_pos.each_as() * 0.0005).0) * 128.0;
		(h + h2) as i32
	}

	fn get_top_layer_block(&self, y: i32, height: i32) -> Block {
		if y > height - 5 {
			if y > 85 {
				Block { id: BlockId::Snow as u16, state: 0 }
			} else if y == height {
				if y > 64 {
					Block { id: BlockId::SnowGrass as u16, state: 0 }
				} else {
					Block { id: BlockId::Grass as u16, state: 0 }
				}
			} else {
				Block { id: BlockId::Dirt as u16, state: 0 }
			}
		} else {
			Block { id: BlockId::Stone as u16, state: 0 }
		}
	}

	pub fn generate_chunk(&self, chunk_pos: Vec3i32) -> Option<chunk::Chunk> {
		let mut chunk = chunk::Chunk::new(chunk_pos, chunk::ChunkData::new());

		for z in 0..CHUNK_SIZE.z as i32 {
			for x in 0..CHUNK_SIZE.x as i32 {
				let local_pos = vec2(x, z);
				let world_pos = chunk.position.xz() * CHUNK_SIZE.xz().each_as() + local_pos;
				let height = self.get_height(world_pos);
				for y in 0..CHUNK_SIZE.y as i32 {
					let local_pos = vec3(x, y, z);
					let world_y = local_pos.y + chunk.position.y * CHUNK_SIZE.y as i32;
					if world_y > height {
						break
					}
					chunk.data.set_block(local_pos, self.get_top_layer_block(world_y, height));
				}
			}
		}

		Some(chunk)
	}
}

