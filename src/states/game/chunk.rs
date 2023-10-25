use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::{gfx, math::*};

pub const CHUNK_SIZE: Vec3<usize> = Vector([32, 32, 32]);
pub const CHUNK_BLOCK_COUNT: usize = CHUNK_SIZE.0[0] * CHUNK_SIZE.0[1] * CHUNK_SIZE.0[2];

#[derive(Clone, Copy)]
#[repr(packed)]
pub struct Block {
	pub id: u16,
	pub state: u16
}

#[derive(FromPrimitive)]
enum BlockId {
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
}

impl Block {
	fn is_solid(&self) -> bool {
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

	fn get(&self) -> &ChunkData {
		unsafe { &*self._ptr }
	}
}


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

const CUBE_VERTICES: [[f32; 3]; 8] = [
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

const CUBE_FACES: [(FaceDirection, [usize; 4]); 6] = [
	(FaceDirection::PX, [0, 4, 5, 1]),
	(FaceDirection::NX, [2, 6, 7, 3]),
	(FaceDirection::PY, [0, 1, 2, 3]),
	(FaceDirection::NY, [4, 5, 6, 7]),
	(FaceDirection::PZ, [0, 3, 7, 4]),
	(FaceDirection::NZ, [1, 2, 6, 5]),
];

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
		chunk_neighbors: &[Option<UnsafeChunkDataRef>]
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
						chunk_position.map(|c| c as f32) *
						CHUNK_SIZE.map(|c| c as f32) +
						Vector([x, y, z]).map(|c| c as f32);

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
										// BUG: this always happens?? wrong offset (?)
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
	
						for index in face_vertices {
							let [vx, vy, vz] = CUBE_VERTICES[index];
							vertices.push(super::renderer::chunk::BlockVertex {
								position: [
									vx + block_position.x,
									vy + block_position.y,
									vz + block_position.z,
								],
								_pad: 0,
								data: block.id as u32 | (direction as u32) << 16,
							});
						}

						for index in [0, 1, 2,  2, 3, 0] {
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

	pub fn update_mesh(&mut self, gfx: &gfx::Gfx, chunk_neighbors: &[Option<UnsafeChunkDataRef>]) {
		let (vertices, indices) = self.data.generate_mesh(self.position, chunk_neighbors);
		if let Some(ref mut mesh) = &mut self.mesh {
			mesh.update(gfx, &vertices, &indices);
		} else {
			self.mesh = Some(gfx::Mesh::new(gfx, &vertices, &indices));
		}
	}
}
