use std::{collections::HashMap, sync::LazyLock};

/*
 * In this file we literally are decompressing assets from read-only memory extracted from Earthbound.
 * Credit goes to Mr. Accident of forum.starmen.net for the reverse engineering of the ROM data.
 * This implementation is based on his work. It's also probably in a legal grey area. Don't tell Nintendo :)
 */

/// background ROM data extracted directly from Earthbound/Mother 2
const BACKGROUNDS_DAT: &[u8] = include_bytes!("../../../assets/saturn_bg.dat");

#[derive(Clone,Debug)]
pub struct SaturnBgData {
	pub tiles: Vec<Option<TileSet>>,
	pub arrangements: Vec<Arrangement>,
	pub palettes: Vec<Option<Palette>>,
	pub backgrounds: Vec<BackgroundEntry>,
	pub effects: Vec<DistortionEffect>,
}

impl Default for SaturnBgData {
	fn default() -> Self {
		Self::from_rom(BACKGROUNDS_DAT).unwrap()
	}
}

impl SaturnBgData {
	pub const BLACKLISTED: [usize;20] = [
		// these are either pure black screens, or solid unchanging colors
		0,
		33,
		191,
		192,
		193,
		194,
		235,
		236,
		237,
		238,
		239,
		241,
		242,
		244,
		245,
		247,
		253,
		254,
		265,
		279,
	];
	pub const GIYGAS: [usize;17] = [
		// these are all the backgrounds used in the Giygas fight.
		// Will make your pc look haunted, so they are disabled by default, but can be enabled in the config file.
		220,
		221,
		222,
		223,
		224,
		225,
		226,
		227,
		248,
		249,
		250,
		251,
		252,
		295,
		296,
		298,
		301,
	];
	pub fn new() -> Self {
		Self::default()
	}

	pub fn get_effect(&self, bg_index: usize, effect_index: usize) -> &DistortionEffect {
		let bg = &self.backgrounds[bg_index];

		let mut effect = &self.effects[bg.effects[effect_index] as usize];
		let mut rot_idx = effect_index;
		while effect.is_empty() {
			rot_idx = (rot_idx + 1) % 4;
			if rot_idx == effect_index {
				// we've looped through all 4 effects and they're all default, so just return the first one
				break;
			}
			effect = &self.effects[bg.effects[effect_index] as usize];
		}
		effect
	}

	pub fn valid_indices(&self, no_giygas: bool) -> Vec<usize> {
		let mut indices = vec![];
		for i in 0..327 {
			if Self::BLACKLISTED.contains(&i)
			|| no_giygas && Self::GIYGAS.contains(&i) {
				// it's a black screen for some reason.
				continue;
			}
			if self.index_is_valid(i) {
				indices.push(i);
			}
		}

		indices
	}

	pub fn index_is_valid(&self, bg_index: usize) -> bool {
		let bg = &self.backgrounds[bg_index];
		self.tiles[bg.graphics_index as usize].is_some() && self.palettes[bg.palette_index as usize].is_some()
	}

	pub fn get_framebuffer(&self, bg_index: usize) -> Vec<u8> {
		let bg = &self.backgrounds[bg_index];
		let tileset = self.tiles[bg.graphics_index as usize].as_ref().unwrap();
		let arrangement = &self.arrangements[bg.graphics_index as usize];

		let mut f_buf = vec![0u8; 256 * 256];

		for py in 0..256 {
			for px in 0..256 {
				let entry = &arrangement.grid[py / 8][px / 8];
				let tile = &tileset.tiles[entry.tile_index as usize];

				let tx = if entry.h_flip { 7 - (px % 8) } else { px % 8 };
				let ty = if entry.v_flip { 7 - (py % 8) } else { py % 8 };

				f_buf[py * 256 + px] = tile[ty][tx];
			}
		}

		f_buf
	}

	pub fn get_palette(&self, bg_index: usize) -> Vec<[u8; 3]> {
		self.palettes[self.backgrounds[bg_index].palette_index as usize]
			.as_ref().unwrap().colors.clone()
	}
}

#[derive(Clone,Debug)]
pub struct TileSet {
	pub tiles: Vec<[[u8;8];8]> // 8x8 tiles, each pixel is a palette index
}

#[derive(Clone,Debug)]
pub struct Arrangement {
	pub grid: [[ArrangementEntry; 32]; 32], // 32x32 grid of tile references
}

#[derive(Debug, Clone, Default, Copy)]
pub struct ArrangementEntry {
	pub tile_index: u16, // bits 0-9: which tile
	pub sub_palette: u8, // bits 10-12: which sub-palette
	pub h_flip: bool,		 // bit 14
	pub v_flip: bool		 // bit 15
}

#[derive(Clone,Debug)]
pub struct Palette {
	pub colors: Vec<[u8;3]>, // RGB colors (4 or 16 depending on BPP)
}

#[derive(Clone,Debug)]
pub struct BackgroundEntry {
	pub graphics_index: u8,
	pub palette_index: u8,
	pub bpp: u8, // bits per pixel (2 or 4)
	pub cycle_type: u8, // palette cycling mode (0-3)
	pub cycle1_start: u8,
	pub cycle1_end: u8,
	pub cycle2_start: u8,
	pub cycle2_end: u8,
	pub cycle_speed: u8,
	pub movement: [u8; 4],
	pub effects: [u8; 4] // 4 distortion effect indices (bytes 13-16)
}

#[derive(Clone,PartialEq,Default,Debug)]
pub struct DistortionEffect {
	pub duration: i16,
	pub distortion_type: u8,
	pub frequency: i16,
	pub amplitude: i16,
	pub compression: i16,
	pub freq_accel: i16,
	pub amp_accel: i16,
	pub speed: u8,
	pub comp_accel: i16
}

impl DistortionEffect {
	pub const HORIZONTAL: u8 = 1;
	pub const INTERLACE: u8 = 2;
	pub const VERTICAL: u8 = 3;

	pub fn is_empty(&self) -> bool {
		self.frequency == 0 &&
		self.amplitude == 0 &&
		self.compression == 0 &&
		self.freq_accel == 0 &&
		self.amp_accel == 0 &&
		self.speed == 0 &&
		self.comp_accel == 0
	}
}

impl SaturnBgData {
	const BG_ENTRIES: usize = 327;
	const TILESETS: usize = 103;
	const ARRANGEMENTS: usize = 103;
	const PALETTES: usize = 114;
	const DISTORTIONS: usize = 135;

	const BG_ENTRY_OFFSET: usize = 0xADEA1;
	const TILESET_PTR_OFFSET: usize = 0xAD9A1;
	const ARRANGEMENT_PTR_OFFSET: usize = 0xADB3D;
	const PALETTE_PTR_OFFSET: usize = 0xADCD9;
	const DISTORTION_OFFSET: usize = 0xAF908;

	pub fn from_rom(rom: &[u8]) -> anyhow::Result<Self> {
		let backgrounds = Self::get_background_entries(rom);
		let bpp_palette_map: HashMap<u8,u8> = backgrounds.iter()
			.map(|e| (e.palette_index,e.bpp))
			.collect();
		let bpp_graphics_map: HashMap<u8,u8> = backgrounds.iter()
			.map(|e| (e.graphics_index,e.bpp))
			.collect();

		let palettes = Self::get_palettes(rom, &bpp_palette_map);
		let tiles = Self::get_tilesets(rom, &bpp_graphics_map);
		let arrangements = Self::get_arrangements(rom);
		let effects = Self::get_effects(rom);

		Ok(Self {
			tiles,
			arrangements,
			palettes,
			backgrounds,
			effects,
		})
	}

	fn get_background_entries(rom: &[u8]) -> Vec<BackgroundEntry> {
		let mut entries = vec![];

		for i in 0..Self::BG_ENTRIES {
			let start = Self::BG_ENTRY_OFFSET + i * 17;
			let end = start + 17;
			let mut data = &rom[start..end];
			let entry = BackgroundEntry {
				graphics_index: Self::read_one(&mut data).unwrap(),
				palette_index: Self::read_one(&mut data).unwrap(),
				bpp: Self::read_one(&mut data).unwrap(),
				cycle_type: Self::read_one(&mut data).unwrap(),
				cycle1_start: Self::read_one(&mut data).unwrap(),
				cycle1_end: Self::read_one(&mut data).unwrap(),
				cycle2_start: Self::read_one(&mut data).unwrap(),
				cycle2_end: Self::read_one(&mut data).unwrap(),
				cycle_speed: Self::read_one(&mut data).unwrap(),
				movement: Self::read_four(&mut data).unwrap(),
				effects: Self::read_four(&mut data).unwrap()
			};

			entries.push(entry);
		}

		entries
	}

	fn get_palettes(rom: &[u8], bpp_map: &HashMap<u8, u8>) -> Vec<Option<Palette>> {
		let mut palettes = vec![];

		for i in 0..Self::PALETTES {
			let Some(bpp) = bpp_map.get(&(i as u8)) else {
				palettes.push(None);
				continue
			};

			let ptr_offset = Self::PALETTE_PTR_OFFSET + i * 4;
			let mut ptr_data = &rom[ptr_offset..ptr_offset+4];
			let addr = Self::read_u32_le(&mut ptr_data).unwrap();
			if addr == 0 {
				palettes.push(None);
				continue
			}
			let offset = Self::snes_to_offset(addr);
			let num_colors = 1 << bpp; // 4 or 16
			let num_reads = num_colors * 2; // each color is 2 bytes

			let mut pal_data = &rom[offset..offset + num_reads];
			let mut colors = vec![];
			for _ in 0..num_colors {
				let raw = Self::read_u16_le(&mut pal_data).unwrap();
				let r = ((raw & 0x1F) * 8) as u8;
				let g = (((raw >> 5) & 0x1F) * 8) as u8;
				let b = (((raw >> 10) & 0x1F) * 8) as u8;
				colors.push([r, g, b]);
			}
			palettes.push(Some(Palette { colors }));
		}

		palettes
	}

	fn get_tilesets(rom: &[u8], bpp_map: &HashMap<u8, u8>) -> Vec<Option<TileSet>> {
		let mut tilesets = vec![];

		for i in 0..Self::TILESETS {
			let Some(bpp) = bpp_map.get(&(i as u8)) else {
				tilesets.push(None);
				continue
			};

			let ptr_offset = Self::TILESET_PTR_OFFSET + i * 4;
			let mut ptr_data = &rom[ptr_offset..ptr_offset+4];
			let addr = Self::read_u32_le(&mut ptr_data).unwrap();
			if addr == 0 {
				tilesets.push(None);
				continue
			}
			let offset = Self::snes_to_offset(addr);

			let mut target_data = &rom[offset..];
			let decompressed_data = Self::decompress(&mut target_data);

			let num_tiles = decompressed_data.len() / (8 * *bpp as usize); // each tile is 8 rows of pixels, each row is bpp bits per pixel
			let mut tiles = vec![];
			let mut tile = [[0u8;8]; 8];
			for i in 0..num_tiles {
				let base = i * 8 * *bpp as usize;
				for y in 0..8 {
					for x in 0..8 {
						let mut color = 0u8;
						for bp in 0..*bpp {
							let half_bp = bp / 2;
							let byte = decompressed_data[base + y * 2 + half_bp as usize * 16 + (bp as usize & 1)];
							color |= ((byte >> (7 - x)) & 1) << bp;
						}
						tile[y][x] = color;
					}
				}

				tiles.push(std::mem::take(&mut tile));
			}
			tilesets.push(Some(TileSet { tiles }));
		}

		tilesets
	}

	fn get_arrangements(rom: &[u8]) -> Vec<Arrangement> {
		let mut arrangements = vec![];

		for i in 0..Self::ARRANGEMENTS {
			let ptr_offset = Self::ARRANGEMENT_PTR_OFFSET + i * 4;
			let mut ptr_data = &rom[ptr_offset..ptr_offset+4];
			let addr = Self::read_u32_le(&mut ptr_data).unwrap();
			if addr == 0 {
				arrangements.push(Arrangement { grid: [[ArrangementEntry::default(); 32]; 32] });
				continue
			}
			let offset = Self::snes_to_offset(addr);

			let mut target_data = &rom[offset..];
			let decompressed_data = Self::decompress(&mut target_data);

			let mut grid = [[ArrangementEntry::default(); 32]; 32];
			for y in 0..32 {
				for x in 0..32 {
					let base = (y * 32 + x) * 2;
					let raw = u16::from_le_bytes([decompressed_data[base], decompressed_data[base + 1]]);
					let entry = ArrangementEntry {
						tile_index: raw & 0x3FF,
						sub_palette: ((raw >> 10) & 0x7) as u8,
						h_flip: (raw & 0x4000) != 0,
						v_flip: (raw & 0x8000) != 0
					};
					grid[y][x] = entry;
				}
			}
			arrangements.push(Arrangement { grid });
		}

		arrangements
	}

	#[allow(clippy::field_reassign_with_default)]
	fn get_effects(rom: &[u8]) -> Vec<DistortionEffect> {
		let mut effects = vec![];

		for i in 0..Self::DISTORTIONS {
			let start = Self::DISTORTION_OFFSET + i * 17;
			let end = start + 17;
			let mut data = &rom[start..end];

			let mut entry = DistortionEffect::default();

			entry.duration = Self::read_i16_le(&mut data).unwrap();
			entry.distortion_type = Self::read_one(&mut data).unwrap();
			entry.frequency = Self::read_i16_le(&mut data).unwrap();
			entry.amplitude = Self::read_i16_le(&mut data).unwrap();
			Self::read_one(&mut data); // byte 7 is unused
			entry.compression = Self::read_i16_le(&mut data).unwrap();
			entry.freq_accel = Self::read_i16_le(&mut data).unwrap();
			entry.amp_accel = Self::read_i16_le(&mut data).unwrap();
			entry.speed = Self::read_one(&mut data).unwrap();
			entry.comp_accel = Self::read_i16_le(&mut data).unwrap();

			effects.push(entry);
		}

		effects
	}

	fn read_one(data: &mut &[u8]) -> Option<u8> {
		let byte = data.first()?;
		*data = &data[1..];
		Some(*byte)
	}

	fn read_two(data: &mut &[u8]) -> Option<[u8;2]> {
		let byte1 = data.get(0)?;
		let byte2 = data.get(1)?;
		*data = &data[2..];
		Some([*byte1, *byte2])
	}

	fn read_four(data: &mut &[u8]) -> Option<[u8;4]> {
		let byte1 = data.get(0)?;
		let byte2 = data.get(1)?;
		let byte3 = data.get(2)?;
		let byte4 = data.get(3)?;
		*data = &data[4..];
		Some([*byte1, *byte2, *byte3, *byte4])
	}

	fn read_i16_le(data: &mut &[u8]) -> Option<i16> {
		let bytes = Self::read_two(data)?;
		Some(i16::from_le_bytes(bytes))
	}

	fn read_u16_be(data: &mut &[u8]) -> Option<u16> {
		let bytes = Self::read_two(data)?;
		Some(u16::from_be_bytes(bytes))
	}
	fn read_u16_le(data: &mut &[u8]) -> Option<u16> {
		let bytes = Self::read_two(data)?;
		Some(u16::from_le_bytes(bytes))
	}

	fn read_u32_le(data: &mut &[u8]) -> Option<u32> {
		let bytes = Self::read_four(data)?;
		Some(u32::from_le_bytes(bytes))
	}

	fn snes_to_offset(addr: u32) -> usize {
		let addr = if (0xC00000u32..0x1000000u32).contains(&addr) {
			addr - 0xC00000
		} else if (0x400000..0x600000).contains(&addr) {
			addr
		} else {
			panic!("snes address out of range: {addr:#X}")
		};
		(addr as usize) + 0x200
	}
	fn decompress(data: &mut &[u8]) -> Vec<u8> {
		// Copy N bytes verbatim
		const COPY_N: u8 = 0;
		// Repeat one byte N times
		const REPEAT_ONE_N: u8 = 1;
		// Repeat two bytes N times, alternating
		const REPEAT_TWO_N: u8 = 2;
		// Write increment sequence (byte, byte+1, byte+2)
		const INC_SEQ: u8 = 3;
		// Copy N bytes from earlier in the output buffer
		const COPY_N_FROM: u8 = 4;
		// Same as COPY_N_FROM except bit-reverse each byte
		const COPY_N_BIT_REV_FROM: u8 = 5;
		// Same as COPY_N_FROM but read backwards
		const COPY_N_FROM_BKWD: u8 = 6;
		// Signal end of compressed data (must be last command)
		const STOP: u8 = 0xFF;

		let mut out_buf = vec![];

		while let Some(byte) = Self::read_one(data) {
			if byte == STOP {
				break;
			}

			let (cmd_type, length) = if byte >> 5 == 7 {
				// all upper bits are 1, signaling the extended length format
				let cmd_type = (byte >> 2) & 0x7; // bits 2-4
				let next = Self::read_one(data).unwrap();
				let length = (((byte & 0x3) as u16) << 8) + (next as u16) + 1;
				(cmd_type, length)
			} else {
				// normal length format
				(byte >> 5, (byte & 0x1F) as u16 + 1)
			};
			match cmd_type {
				COPY_N => {
					for _ in 0..length {
						out_buf.push(Self::read_one(data).unwrap());
					}
				}
				REPEAT_ONE_N => {
					let b = Self::read_one(data).unwrap();
					for _ in 0..length {
						out_buf.push(b);
					}
				}
				REPEAT_TWO_N => {
					let b1 = Self::read_one(data).unwrap();
					let b2 = Self::read_one(data).unwrap();
					for _ in 0..length {
						out_buf.push(b1);
						out_buf.push(b2);
					}
				}
				INC_SEQ => {
					let mut b = Self::read_one(data).unwrap();
					for _ in 0..length {
						out_buf.push(b);
						b = b.wrapping_add(1);
					}
				}
				COPY_N_FROM => {
					let pos = Self::read_u16_be(data).unwrap() as usize;
					for i in 0..length {
						out_buf.push(out_buf[pos + i as usize]);
					}
				}
				COPY_N_BIT_REV_FROM => {
					let pos = Self::read_u16_be(data).unwrap() as usize;
					for i in 0..length {
						out_buf.push(out_buf[pos + i as usize].reverse_bits());
					}
				}
				COPY_N_FROM_BKWD => {
					let pos = Self::read_u16_be(data).unwrap() as usize;
					for i in 0..length {
						out_buf.push(out_buf[pos - i as usize]);
					}
				}
				_ => panic!("invalid command type: {cmd_type}")
			}
		}

		out_buf
	}
}

pub static BACKGROUNDS: LazyLock<SaturnBgData> = LazyLock::new(|| {
	SaturnBgData::from_rom(BACKGROUNDS_DAT).unwrap()
});
