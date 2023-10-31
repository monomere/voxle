use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::{gfx, math::*};

use super::{Dir, CUBE_FACES, CUBE_VERTICES, texture::{BlockTextures, TextureId}};

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

#[derive(FromPrimitive)]
pub enum BlockId {
	AIR = 0,
	STONE = 1,
	GRASS = 2,
	DIRT = 3,
}


impl BlockId {
	fn is_solid(self) -> bool {
		match self {
    	BlockId::AIR => false,
			BlockId::STONE => true,
			BlockId::GRASS => true,
			BlockId::DIRT => true,
		}
	}

	fn textures(self) -> Option<BlockTextures> {
		match self {
    	BlockId::AIR => None,
			BlockId::STONE => Some(BlockTextures::same(TextureId(3))),
			BlockId::GRASS => Some(BlockTextures::cylinder(TextureId(1), TextureId(2), TextureId(0))),
			BlockId::DIRT => Some(BlockTextures::same(TextureId(2))),
		}
	}
}

impl Block {
	pub fn is_solid(&self) -> bool {
		match BlockId::from_u16(self.id) {
			Some(id) => id.is_solid(),
			None => true
		}
	}
}

pub struct ChunkData {
	pub blocks: [Block; CHUNK_BLOCK_COUNT]
}

pub struct UnsafeChunkDataRef {
	_ptr: *const ChunkData
}

impl UnsafeChunkDataRef {
	pub fn new(ptr: *const ChunkData) -> Self {
		Self { _ptr: ptr }
	}

	pub fn get(&self) -> &ChunkData {
		unsafe { &*self._ptr }
	}
}


impl ChunkData {
	pub fn new() -> Self {
		Self {
			blocks: [Block { id: 0, state: 0 }; CHUNK_BLOCK_COUNT]
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
		chunk_neighbors: &[Option<UnsafeChunkDataRef>],
		texture_size: Vec2u32
	) -> (Vec<super::renderer::chunk::BlockVertex>, Vec<u32>) {
		let mut vertices = Vec::<super::renderer::chunk::BlockVertex>::new();
		let mut indices = Vec::<u32>::new();

		'outer: for y in 0..CHUNK_SIZE.y as i32 {
			for z in 0..CHUNK_SIZE.z as i32 {
				for x in 0..CHUNK_SIZE.x as i32 {
					let pos = vec3(x, y, z);
					let offset = self.coords_to_offset(pos);
					if let None = offset { break 'outer }
					let offset = offset.unwrap();

					let block = self.blocks[offset];

					let block_position =
						chunk_position.each_as::<f32>() *
						CHUNK_SIZE.each_as() +
						Vector([x, y, z]).each_as();

					if block.id == 0 {
						continue; // don't render air.
					}

					for (direction, face_vertices) in CUBE_FACES {
						let normal = direction.normal::<i32>();
						if let Some(offset) = self.coords_to_offset(normal + pos) {
							if self.blocks[offset].is_solid() {
								continue; // don't render faces facing solid blocks.
							}
						} else { // neighbor block in different chunk:
							if let Some(neighbor_chunk) = &chunk_neighbors[direction as usize] {
								let pos = direction.zero_axis(
									pos,
									CHUNK_SIZE.each_as() - 1,
									Vector::zero()
								);

								if let Some(offset) = neighbor_chunk.get().coords_to_offset(pos) {
									if neighbor_chunk.get().blocks[offset].is_solid() {
										continue; // don't render faces facing solid blocks (in other chunks).
									}
								} else {
									panic!("😭");
								}
							} else {
								continue; // don't render faces for chunks that aren't generated yet.
							}
						}

						let start_index = vertices.len() as u32;

						let make_uvs = |i: usize, uvs: Rect<f32>| {
							[
								vec2(0.0, 1.0),
								vec2(1.0, 1.0),
								vec2(1.0, 0.0),
								vec2(0.0, 0.0),
								// vec2(uvs.x1(), uvs.h - uvs.y1()),
								// vec2(uvs.x2(), uvs.h - uvs.y1()),
								// vec2(uvs.x2(), uvs.h - uvs.y2()),
								// vec2(uvs.x1(), uvs.h - uvs.y2()),
							][i]
						};
	
						for (index_index, index) in face_vertices.into_iter().enumerate() {
							let vertex = Vector(CUBE_VERTICES[index]);
							vertices.push(super::renderer::chunk::BlockVertex {
								position: (vertex + block_position).0,
								texcoord: make_uvs(
									index_index,
									Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }
									// BlockId::from_u16(block.id)
									// 	.and_then(|id| id.textures().map(|tex| tex.in_direction(direction).uvs(texture_size)))
									// 	.unwrap_or()
								).0,
								data: block.id as u32 | (direction as u32) << 16, // TODO: check if in cave (for shading)
							});
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
	pub data: Box<ChunkData>,
	pub mesh: Option<gfx::Mesh<super::renderer::chunk::BlockVertex>>,
	pub position: Vec3i32
}

impl Chunk {
	pub fn new(position: Vec3i32, data: ChunkData) -> Self {
		Self {
			data: Box::new(data),
			position,
			mesh: None,
		}
	}

	pub fn update_mesh(&mut self, gfx: &gfx::Gfx, chunk_neighbors: &[Option<UnsafeChunkDataRef>], texture_size: Vec2u32) {
		let (vertices, indices) = self.data.generate_mesh(self.position, chunk_neighbors, texture_size);
		if let Some(ref mut mesh) = &mut self.mesh {
			mesh.update(gfx, &vertices, &indices);
		} else {
			self.mesh = Some(gfx::Mesh::new(gfx, &vertices, &indices));
		}
	}
}
