use std::collections::HashMap;

use crate::math::{Rect, Vec2u32, vec2};

use super::{Dir, chunk::BlockId};

#[derive(Clone, Copy)]
pub struct TextureId(pub u32);

impl TextureId {
	const SIZE: u32 = 16;

	fn rect(self) -> Rect<u32> {
		let p = self.0 * Self::SIZE;
		Rect {
			x: p,
			y: 0,
			w: Self::SIZE,
			h: Self::SIZE
		}
	}
}

pub struct BlockSides<T: Copy + Clone> {
	pub top: T,
	pub bottom: T,
	pub left: T,
	pub right: T,
	pub front: T,
	pub back: T,
}

impl<T: Copy + Clone> BlockSides<T> {
	pub fn same(t: T) -> Self {
		Self { top: t, bottom: t, left: t, right: t, front: t, back: t }
	}

	pub fn cylinder(top: T, bottom: T, side: T) -> Self {
		Self { top, bottom, left: side, right: side, front: side, back: side }
	}

	pub fn all(&self) -> [T; 6] {
		[self.top, self.bottom, self.left, self.right, self.front, self.back]
	}

	pub fn in_direction(&self, dir: Dir) -> T {
		match dir {
			Dir::PX => self.right,
			Dir::NX => self.left,
			Dir::PY => self.top,
			Dir::NY => self.bottom,
			Dir::PZ => self.front,
			Dir::NZ => self.back,
		}
	}
}

pub type BlockTextures = BlockSides<TextureId>;

pub struct TextureSource {
	pub id: TextureId,
	pub data: Option<image::RgbaImage>
}

pub struct LoadedTextures {
	pub blocks: HashMap<BlockId, BlockTextures>,
	pub textures: Vec<TextureSource>,
	pub size: Vec2u32
}

pub fn load_block_textures(json_path: &str) -> Result<LoadedTextures, std::io::Error> {
	let mut r = LoadedTextures {
		blocks: HashMap::new(),
		textures: Vec::new(),
		size: vec2(0, 0)
	};

	let mut texture_paths = HashMap::<std::path::PathBuf, TextureId>::new();

	let manifest = json::parse(&std::fs::read_to_string(json_path)?)
		.expect("block texture manifest should be correct json");

	if !manifest.is_array() {
		panic!("block texture manifest should be an array");
	}

	// TODO: generate one large image buffer for this.

	let root_dir = std::path::Path::new(json_path).parent().expect("json manifest should have parent dir");
	for entry in manifest.members() {
		if !entry.is_array() {
			panic!("block texture manifest entries should be arrays");
		}

		let name = entry[0].as_str().expect("block texture manifest entry's first element should be the block name");
		let mut texs = BlockTextures::same(TextureId(u32::MAX));
		for (index, texture_path) in entry.members().skip(1).enumerate() {
			let path = texture_path.as_str().expect("block texture manifest entry's elements must be strings");
			let path = root_dir.join(path).canonicalize().expect("texture path should be a valid path");
			
			let id = if !texture_paths.contains_key(&path) {
				let id = TextureId(r.textures.len() as u32);

				let image_data = image::load(
					std::io::BufReader::new(std::fs::File::open(&path).unwrap()),
					image::ImageFormat::from_path(&path).unwrap()
				).unwrap().to_rgba8();

				if r.size == vec2(0, 0) {
					r.size = vec2(image_data.width(), image_data.height());
				}

				r.textures.push(TextureSource {
					id,
					data: Some(image_data)
				});

				texture_paths.insert(path, id);

				id
			} else {
				texture_paths[&path]
			};

			// this is an optimization, so that we don't
			// have to create another vector with TextureIds.
			match entry.len() - 1 {
				1 => texs = BlockTextures::same(id), // the only time we iterate
				3 => match index { // depends on the current index
					0 => texs.top = id,
					1 => texs.bottom = id,
					2 => (texs.left, texs.right, texs.front, texs.back) = (id, id, id, id),
					_ => unreachable!()
				},
				6 => match index {
					0 => texs.back = id,
					1 => texs.bottom = id,
					2 => texs.left = id,
					3 => texs.right = id,
					4 => texs.front = id,
					5 => texs.back = id,
					_ => unreachable!()
				},
				_ => panic!("block texture manifest entry's can only provide 1, 3 or 6 textures.")
			}
		}

		let found_block_id = match name {
			"stone" => Some(BlockId::Stone),
			"grass" => Some(BlockId::Grass),
			"dirt" => Some(BlockId::Dirt),
			"snow" => None,
			"snow_grass" => None,
			_ => None
		};
		if let Some(block_id) = found_block_id {
			r.blocks.insert(block_id, texs);
		} else {
			eprintln!("warning: unknown block id: {}", name);
		}
	}

	Ok(r)
}
