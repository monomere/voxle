use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::gfx;

use super::renderer::BlockVertex;

pub const CHUNK_SIZE: (usize, usize, usize) = (16, 16, 16);
pub const CHUNK_BLOCK_COUNT: usize = CHUNK_SIZE.0 * CHUNK_SIZE.1 * CHUNK_SIZE.2;

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

pub struct ChunkDataRef {
	_ptr: *const ChunkData
}

impl ChunkDataRef {
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

	pub fn normal<T: From<i32>>(&self) -> (T, T, T) {
		match self {
			Self::PX => (( 1).into(), ( 0).into(), ( 0).into()),
			Self::NX => ((-1).into(), ( 0).into(), ( 0).into()),
			Self::PY => (( 0).into(), ( 1).into(), ( 0).into()),
			Self::NY => (( 0).into(), (-1).into(), ( 0).into()),
			Self::PZ => (( 0).into(), ( 0).into(), ( 1).into()),
			Self::NZ => (( 0).into(), ( 0).into(), (-1).into()),
		}
	}

	/// zeroes the axis of the normal, leaves other axes.
	pub fn zero_axis<T: From<i32>>(&self, (x, y, z): (T, T, T)) -> (T, T, T) {
		match self {
			Self::PX => ((0).into(), y, z),
			Self::NX => ((0).into(), y, z),
			Self::PY => (x, (0).into(), z),
			Self::NY => (x, (0).into(), z),
			Self::PZ => (x, y, (0).into()),
			Self::NZ => (x, y, (0).into()),
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

	fn coords_to_offset(&self, x: i32, y: i32, z: i32) -> Option<usize> {
		if x < 0 || y < 0 || z < 0 {
			None
		} else {
			let (x, y, z) = (x as usize, y as usize, z as usize);
			if x >= CHUNK_SIZE.0 || y >= CHUNK_SIZE.1 || z >= CHUNK_SIZE.2 {
				None
			} else {
				let offset = y * CHUNK_SIZE.2 * CHUNK_SIZE.0 + CHUNK_SIZE.0 * z + x;
				if offset >= self.blocks.len() { None } else { Some(offset) }
			}
		}
	}

	pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: Block) {
		if let Some(offset) = self.coords_to_offset(x, y, z) {
			self.blocks[offset] = block;
		}
	}

	pub fn generate_mesh(
		&self,
		chunk_position: [i32; 3],
		chunk_neighbors: &[Option<ChunkDataRef>]
	) -> (Vec<super::renderer::BlockVertex>, Vec<u32>) {
		let mut vertices = Vec::<super::renderer::BlockVertex>::new();
		let mut indices = Vec::<u32>::new();

		'outer: for y in 0..CHUNK_SIZE.1 as i32 {
			for z in 0..CHUNK_SIZE.2 as i32 {
				for x in 0..CHUNK_SIZE.0 as i32 {
					let offset = self.coords_to_offset(x, y, z);
					if let None = offset { break 'outer }
					let offset = offset.unwrap();

					let block = self.blocks[offset];

					let block_position = [
						x as f32 + chunk_position[0] as f32 * CHUNK_SIZE.0 as f32,
						y as f32 + chunk_position[1] as f32 * CHUNK_SIZE.1 as f32,
						z as f32 + chunk_position[2] as f32 * CHUNK_SIZE.2 as f32,
					];

					if block.id == 0 {
						continue; // don't render air.
					}

					for (direction, face_vertices) in CUBE_FACES {
						let (nx, ny, nz) = direction.normal::<i32>();
						if let Some(offset) = self.coords_to_offset(nx + x, ny + y, nz + z) {
							if self.blocks[offset].is_solid() {
								continue; // don't render faces facing solid blocks.
							}
						} else { // neighbor block in different chunk:
							if let Some(neighbor_chunk) = &chunk_neighbors[direction as usize] {
								let (x, y, z) = direction.zero_axis((x, y, z));

								if let Some(offset) = neighbor_chunk.get().coords_to_offset(x, y, z) {
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
							vertices.push(super::renderer::BlockVertex {
								position: [
									vx + block_position[0],
									vy + block_position[1],
									vz + block_position[2],
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
	pub mesh: Option<gfx::Mesh<BlockVertex>>,
	pub position: [i32; 3]
}

impl Chunk {
	pub fn new(position: [i32; 3], data: ChunkData) -> Self {
		Self {
			data: Box::new(data),
			position,
			mesh: None,
		}
	}

	pub fn update_mesh(&mut self, gfx: &gfx::Gfx, chunk_neighbors: &[Option<ChunkDataRef>]) {
		let (vertices, indices) = self.data.generate_mesh(self.position, chunk_neighbors);
		if let Some(ref mut mesh) = &mut self.mesh {
			mesh.update(gfx, &vertices, &indices);
		} else {
			self.mesh = Some(gfx::Mesh::new(gfx, &vertices, &indices));
		}
	}
}
