use std::collections::HashMap;

use crate::{gfx, math::*};

use super::{texture::{TextureId, LoadedTextures}, Dir};

pub const CHUNK_SIZE: Vec3<usize> = Vector([32, 32, 32]);

static_assertions::const_assert!(CHUNK_SIZE.0[0].is_power_of_two());
static_assertions::const_assert!(CHUNK_SIZE.0[1].is_power_of_two());
static_assertions::const_assert!(CHUNK_SIZE.0[2].is_power_of_two());

pub const CHUNK_BLOCK_COUNT: usize = CHUNK_SIZE.0[0] * CHUNK_SIZE.0[1] * CHUNK_SIZE.0[2];

/// world -> chunk position (in chunks)
pub fn world_to_chunk(world: Vec3f32) -> Vec3i32 {
	world.zip_map(CHUNK_SIZE, |world, chunk| num::integer::div_floor(world.round() as i32, chunk as i32))
}

/// world -> block position (in blocks)
pub fn world_to_block_local(world: Vec3f32) -> Vec3i32 {
	world.zip_map(CHUNK_SIZE, |world, chunk| num::integer::mod_floor(world.round() as i32, chunk as i32))
}

/// block global -> chunk position (in chunks)
pub fn block_global_to_chunk(global: Vec3i32) -> Vec3i32 {
	global.zip_map(CHUNK_SIZE, |global, chunk| num::integer::div_floor(global, chunk as i32))
}

/// block global -> block position (in blocks)
pub fn block_global_to_block_local(global: Vec3i32) -> Vec3i32 {
	global.zip_map(CHUNK_SIZE, |global, chunk| num::integer::mod_floor(global, chunk as i32))
}

#[derive(Clone, Copy)]
#[repr(packed)]
pub struct Block {
	pub id: u16,
	pub state: u16
}

impl Default for Block {
	fn default() -> Self {
		Self {
			id: 0,
			state: 0
		}		
	}
}

#[allow(dead_code)]
#[non_exhaustive]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BlockId {
	Air = 0,
	Stone = 1,
	Grass = 2,
	Dirt = 3,
	Snow = 4,
	SnowGrass = 5,
	_EndId = 6,
}


impl BlockId {
	fn from_u16(id: u16) -> Option<Self> {
		if id >= Self::_EndId as u16 {
			None
		} else {
			Some(unsafe {
				std::mem::transmute(id as u8)
			})
		}
	}

	fn is_solid(self) -> bool {
		match self {
    	BlockId::Air => false,
			BlockId::Stone => true,
			BlockId::Grass => true,
			BlockId::Dirt => true,
			BlockId::SnowGrass => true,
			BlockId::Snow => true,
			_ => false,
		}
	}

	// fn textures(self) -> Option<BlockTextures> {
	// 	match self {
  //   	BlockId::Air => None,
	// 		BlockId::Stone => Some(BlockTextures::same(TextureId(3))),
	// 		BlockId::Grass => Some(BlockTextures::cylinder(TextureId(1), TextureId(2), TextureId(0))),
	// 		BlockId::Dirt => Some(BlockTextures::same(TextureId(2))),
	// 		_ => None,
	// 	}
	// }
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

pub const CUBE_FACES: [(Dir, [usize; 4]); 6] = [
	(Dir::PX, [5, 4, 0, 1]),
	(Dir::NX, [7, 6, 2, 3]),
	(Dir::PY, [3, 2, 1, 0]),
	(Dir::NY, [4, 5, 6, 7]),
	(Dir::PZ, [4, 7, 3, 0]),
	(Dir::NZ, [6, 5, 1, 2]),
];


impl Block {
	pub fn is_solid(&self) -> bool {
		match BlockId::from_u16(self.id) {
			Some(id) => id.is_solid(),
			None => true
		}
	}
}

// 0 1 3 2 best (top face is weird in corners)
static mut AO_INDEX_MAP_: &'static mut [u32] = &mut [0, 1, 3, 2];

pub fn ao_index_map() -> &'static [u32] {
	unsafe {
		AO_INDEX_MAP_
	}
}

pub fn next_ao_index_map() {
	let nums: &'static mut [u32] = unsafe { &mut AO_INDEX_MAP_ };
	use std::cmp::Ordering;
	// or use feature(array_windows) on nightly
	let last_ascending = match nums.windows(2).rposition(|w| w[0] < w[1]) {
		Some(i) => i,
		None => {
			nums.reverse();
			return;
		}
	};

	let swap_with = nums[last_ascending + 1..]
		.binary_search_by(|n| u32::cmp(&nums[last_ascending], n).then(Ordering::Less))
		.unwrap_err(); // cannot fail because the binary search will never succeed
	nums.swap(last_ascending, last_ascending + swap_with);
	nums[last_ascending + 1..].reverse();
}

pub struct ChunkData {
	pub blocks: Box<[Block; CHUNK_BLOCK_COUNT]>
}

impl ChunkData {
	pub fn new() -> Self {
		Self {
			blocks: unsafe { Box::new_zeroed().assume_init() }
		}
	}

	fn coords_to_offset(&self, Vector([x, y, z]): Vec3<i32>) -> Option<usize> {
		if x < 0 || y < 0 || z < 0 {
			None
		} else {
			let (x, y, z) = (x as usize, y as usize, z as usize);
			if x >= CHUNK_SIZE.x || y >= CHUNK_SIZE.y || z >= CHUNK_SIZE.z {
				None
			} else {
				let offset = y * CHUNK_SIZE.z * CHUNK_SIZE.x + CHUNK_SIZE.x * z + x;
				if offset >= self.blocks.len() { None } else { Some(offset) }
			}
		}
	}

	pub fn get_block(&self, position: Vec3<i32>) -> Option<&Block> {
		if let Some(offset) = self.coords_to_offset(position) {
			Some(&self.blocks[offset])
		} else {
			None
		}
	}

	pub fn set_block(&mut self, position: Vec3<i32>, block: Block) {
		if let Some(offset) = self.coords_to_offset(position) {
			self.blocks[offset] = block;
		}
	}

	pub fn generate_mesh(
		&self,
		chunk_position: Vec3i32,
		chunk: &HashMap<Vec3i32, Chunk>,
		block_textures: &LoadedTextures
	) -> (Vec<super::renderer::chunk::BlockVertex>, Vec<u32>) {
		let mut vertices = Vec::<super::renderer::chunk::BlockVertex>::new();
		let mut indices = Vec::<u32>::new();

		let is_block_solid_at = |local: Vec3i32, normal: Vec3i32| -> bool {
			if let Some(offset) = self.coords_to_offset(local + normal) {
				self.blocks[offset].is_solid()
			} else if let Some(neighbor_chunk) = &chunk.get(&(chunk_position + normal)) {
				let pos = normal.map_indexed(|c, i| {
					if c == 0 { local.0[i] }
					else if c > 0 { 0 }
					else { CHUNK_SIZE.0[i] as i32 - 1 }
				});

				if let Some(offset) = neighbor_chunk.data.coords_to_offset(pos) {
					neighbor_chunk.data.blocks[offset].is_solid()
				} else {
					panic!("😭")
				}
			} else {
				true
			}
		};

		'outer: for y in 0..CHUNK_SIZE.y as i32 {
			for z in 0..CHUNK_SIZE.z as i32 {
				for x in 0..CHUNK_SIZE.x as i32 {
					let pos = vec3(x, y, z);
					let offset = self.coords_to_offset(pos);
					if offset.is_none() { break 'outer }
					let offset = offset.unwrap();

					let block = self.blocks[offset];

					let block_pos_local =
						Vector([x, y, z]).each_as();

					if block.id == 0 {
						continue; // don't render air.
					}

					for (direction, face_vertices) in CUBE_FACES {
						let normal = direction.normal::<i32>();
						if is_block_solid_at(pos, normal) {
							continue;
						}

						let start_index = vertices.len() as u32;
					
						let texture_id = (BlockId::from_u16(block.id))
							.and_then(|id| block_textures.blocks.get(&id).map(|tex| tex.in_direction(direction)))
							.unwrap_or(TextureId(0)).0;

						let mut ao = [0u8; 4];
						// let ao_index_map = [0, 1, 2, 3];
						for (index, vertex_index) in face_vertices.into_iter().enumerate() {
							let vertex = Vector(CUBE_VERTICES[vertex_index]);
							ao[ao_index_map()[index] as usize] = {
								let vertex = vertex * 2.0; // times 2 because vertices are -0.5..=0.5
								let vertex_cross = direction.exclude_axis(vertex.each_as());
								let corner = is_block_solid_at(pos, vertex.each_as());
								let edge1 = is_block_solid_at(pos, direction.with_others(vec2(vertex_cross.x, 0)));
								let edge2 = is_block_solid_at(pos, direction.with_others(vec2(0, vertex_cross.y)));
								
								if edge1 && edge2 {
									0
								} else {
									3 - (edge1 as u8 + edge2 as u8 + corner as u8)
								}
							};
						}

						for (index_index, index) in face_vertices.into_iter().enumerate() {
							let vertex = Vector(CUBE_VERTICES[index]);

							vertices.push(super::renderer::chunk::BlockVertex::new(
								vertex + block_pos_local,
								index_index as u8,
								&ao,
								texture_id
							));
						}

						for index in [0, 1, 2, 2, 3, 0] {
							indices.push(start_index + index);
						}
					}
				}
			}
		}

		(vertices, indices)
	}
}

pub struct Chunk {
	pub data: ChunkData,
	pub mesh: Option<gfx::Mesh<super::renderer::chunk::BlockVertex>>,
	pub position: Vec3i32
}

impl Chunk {
	pub fn new(position: Vec3i32, data: ChunkData) -> Self {
		Self {
			data,
			position,
			mesh: None,
		}
	}

	pub fn update_mesh(&mut self, gfx: &gfx::Gfx, chunks: &HashMap<Vec3i32, Chunk>, block_textures: &LoadedTextures) {
		let (vertices, indices) = self.data.generate_mesh(self.position, chunks, block_textures);
		if let Some(ref mut mesh) = &mut self.mesh {
			mesh.update(gfx, &vertices, &indices);
		} else {
			self.mesh = Some(gfx::Mesh::new(gfx, &vertices, &indices, Some(format!("Chunk {:?}", self.position.0).as_str())));
		}
	}
}
