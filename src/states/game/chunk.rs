use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::{gfx, math::*};

pub const CHUNK_SIZE: Vec3<usize> = Vector([32, 32, 32]);

static_assertions::const_assert!(CHUNK_SIZE.0[0].is_power_of_two());
static_assertions::const_assert!(CHUNK_SIZE.0[1].is_power_of_two());
static_assertions::const_assert!(CHUNK_SIZE.0[2].is_power_of_two());

pub const CHUNK_BLOCK_COUNT: usize = CHUNK_SIZE.0[0] * CHUNK_SIZE.0[1] * CHUNK_SIZE.0[2];

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

#[derive(Clone, Copy)]
struct BlockTex(u32);

impl BlockTex {
	const SIZE: u32 = 16;

	fn rect(self) -> Vec4u32 {
		let p = self.0 * Self::SIZE;
		vec4(
			p, 16,
			p + 16, 0
		)
	}

	fn uvs(self, texture_size: Vec2u32) -> Vec4f32 {
		self.rect().each_as::<f32>() / vec4(
			texture_size.x as f32,
			texture_size.y as f32,
			texture_size.x as f32,
			texture_size.y as f32,
		)
	}
}

struct BlockTextures {
	top: BlockTex,
	bottom: BlockTex,
	left: BlockTex,
	right: BlockTex,
	front: BlockTex,
	back: BlockTex,
}

impl BlockTextures {
	fn same(t: BlockTex) -> Self {
		Self { top: t, bottom: t, left: t, right: t, front: t, back: t }
	}

	fn cylinder(top: BlockTex, bottom: BlockTex, side: BlockTex) -> Self {
		Self { top, bottom, left: side, right: side, front: side, back: side }
	}

	fn in_direction(&self, dir: FaceDirection) -> BlockTex {
		match dir {
			FaceDirection::PX => self.right,
			FaceDirection::NX => self.left,
			FaceDirection::PY => self.top,
			FaceDirection::NY => self.bottom,
			FaceDirection::PZ => self.front,
			FaceDirection::NZ => self.back,
		}
	}
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
			BlockId::STONE => Some(BlockTextures::same(BlockTex(4))),
			BlockId::GRASS => Some(BlockTextures::cylinder(BlockTex(1), BlockTex(2), BlockTex(0))),
			BlockId::DIRT => Some(BlockTextures::same(BlockTex(2))),
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

	pub fn get(&self) -> &ChunkData {
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
	(FaceDirection::PX, [5, 4, 0, 1]),
	(FaceDirection::NX, [7, 6, 2, 3]),
	(FaceDirection::PY, [3, 2, 1, 0]),
	(FaceDirection::NY, [4, 5, 6, 7]),
	(FaceDirection::PZ, [4, 7, 3, 0]),
	(FaceDirection::NZ, [6, 5, 1, 2]),
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

						let face_uvs = [
							vec2(1.0, 1.0),
							vec2(0.0, 1.0),
							vec2(0.0, 0.0),
							vec2(1.0, 0.0),
						];

						let make_uvs = |i: usize, uvs: Vec4f32| {
							face_uvs[i] * uvs.xy() + (vec2(1.0, 1.0) - face_uvs[i]) * uvs.zw()
						};
	
						for (index_index, index) in face_vertices.into_iter().enumerate() {
							let vertex = Vector(CUBE_VERTICES[index]);
							vertices.push(super::renderer::chunk::BlockVertex {
								position: (vertex + block_position).0,
								texcoord: make_uvs(
									index_index,
									BlockId::from_u16(block.id)
										.and_then(|id| id.textures().map(|tex| tex.in_direction(direction).uvs(texture_size)))
										.unwrap_or(vec4(0.0, 0.0, 0.0, 0.0))
								).0,
								data: block.id as u32 | (direction as u32) << 16,
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
