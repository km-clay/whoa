use std::f64::consts::PI;

use crossterm::style::Color;

use crate::anim::{Animation, Cell, Frame, saturn::romparse::{DistortionEffect, SaturnBgData}};

pub mod romparse;

const C1: f64 = 1.0 / 512.0;
const C2: f64 = 8.0 * PI / (1024.0 * 256.0);
const C3: f64 = PI / 60.0;

fn modulo(n: i32, m: i32) -> usize {
	(((n % m) + m) % m) as usize
}

#[derive(Debug,Default,Clone)]
pub struct Saturn {
	framebuf: Vec<u8>, // palette indices
	palette: Vec<[u8; 3]>, // current palette (mutated by cycling)
	bg_index: usize,
	effect_index: usize,
	data: SaturnBgData,
	orig: Frame,
	interm: Frame,
	no_giygas: bool,
	rows: usize,
	cols: usize,
	speed_x: u8,
	speed_y: u8,
	scroll_x: u32,
	scroll_y: u32,
	anim_lifetime: f64, // seconds before re-rolling animation
	anim_count: usize,
	tick: usize
}

impl Saturn {
	const SPEED_MOD: u32 = 128;
	pub fn new() -> Self {
		let data = SaturnBgData::default();
		Self::from_data(data)
	}
	pub fn from_data(data: SaturnBgData) -> Self {
		let valid_indices = data.valid_indices();
		let bg_index = valid_indices[rand::random_range::<usize, std::ops::Range<usize>>(0..valid_indices.len())];
		//let bg_index = 302;
		let effect_index = rand::random_range::<usize, std::ops::Range<usize>>(0..4);
		let framebuf = data.get_framebuffer(bg_index);
		let palette = data.get_palette(bg_index);
		Self {
			framebuf,
			palette,
			effect_index,
			bg_index,
			speed_x: data.backgrounds[bg_index].movement[0],
			speed_y: data.backgrounds[bg_index].movement[1],
			data,
			anim_lifetime: 20.0,
			no_giygas: true,
			..Default::default()
		}
	}
}

impl Animation for Saturn {
	fn configure(&mut self, config: &toml::Value) {
		let Some(config) = config.get("saturn") else { return };
		self.anim_lifetime = config.get("lifetime")
			.unwrap_or(&toml::Value::Float(20.0)).as_float().unwrap_or(20.0);
		self.bg_index = config.get("bg_index")
			.unwrap_or(&toml::Value::Integer(self.bg_index as i64)).as_integer().unwrap_or(self.bg_index as i64) as usize;
		self.no_giygas = config.get("no_giygas")
			.unwrap_or(&toml::Value::Boolean(true)).as_bool().unwrap_or(true);
	}
	fn init(&mut self, initial: Frame) {
		let valid_indices = self.data.valid_indices();
		self.bg_index = valid_indices[rand::random_range::<usize, std::ops::Range<usize>>(0..valid_indices.len())];
		self.effect_index = rand::random_range::<usize, std::ops::Range<usize>>(0..4);
		self.framebuf = self.data.get_framebuffer(self.bg_index);
		self.palette = self.data.get_palette(self.bg_index);
		self.speed_x = self.data.backgrounds[self.bg_index].movement[0];
		self.speed_y = self.data.backgrounds[self.bg_index].movement[1];

		let (rows,cols) = initial.dims().unwrap_or((0,0));
		self.orig = initial.clone();
		self.interm = Frame::with_capacity(cols, rows);
		self.rows = rows;
		self.cols = cols;
	}

	fn update(&mut self, dt: std::time::Duration) -> Frame {
		log::debug!("current animation has been running for {} seconds", dt.as_secs_f64());
		log::debug!("current animation lifetime is {} seconds", self.anim_lifetime);
		log::debug!("current animation count is {}", self.anim_count);
		log::debug!("dt.as_secs_f64() % self.anim_lifetime = {}", dt.as_secs_f64() % self.anim_lifetime);
		//if (dt.as_secs_f64() % self.anim_lifetime).floor() as usize > self.anim_count {
			//log::debug!("re-rolling animation");
			//// re-roll animation
			//let frame = self.initial_frame();
			//self.init(frame);
			//self.anim_count += 1;
		//}

		let Frame(ref cells) = self.orig;

		let mut effect = self.data.get_effect(self.bg_index, self.effect_index).clone();
		if (self.tick / 60) / 10 > effect.duration as usize {
			let new_idx = (self.effect_index + 1) % 4;
			effect = self.data.get_effect(self.bg_index, new_idx).clone();
			self.effect_index = new_idx;
			self.speed_x = self.data.backgrounds[self.bg_index].movement[0];
			self.speed_y = self.data.backgrounds[self.bg_index].movement[1];
			self.tick = 0;
		}
		log::debug!("animating background {}", self.bg_index);
		log::debug!("animating with effect {}: {effect:?}",self.data.backgrounds[self.bg_index].effects[self.effect_index]);
		log::debug!("animating with motion: {:?}", self.data.backgrounds[self.bg_index].movement);

		let accel_x = self.data.backgrounds[self.bg_index].movement[2];
		let accel_y = self.data.backgrounds[self.bg_index].movement[3];

		self.speed_x = self.speed_x.wrapping_add(accel_x);
		self.speed_y = self.speed_y.wrapping_add(accel_y);

		self.scroll_x = self.scroll_x.wrapping_add(self.speed_x as u32);
		self.scroll_y = self.scroll_y.wrapping_add(self.speed_y as u32);

		let t2 = (self.tick as f64) * 2.0;
		let amp = C1 * (effect.amplitude as f64 + effect.amp_accel as f64 * t2);
		let freq = C2 * (effect.frequency as f64 + effect.freq_accel as f64 * t2);
		let comp = 1.0 + (effect.compression as f64 + effect.comp_accel as f64 * t2) / 256.0;
		let spd = C3 * effect.speed as f64 * self.tick as f64;

		let s = |y: f64| -> i32 {
			(amp * f64::sin(freq * y + spd)).round() as i32
		};

		// virtual height is 2x terminal rows (top and bottom half of each cell)
		let virt_h = self.rows * 2;

		// palette cycling
		let bg = &self.data.backgrounds[self.bg_index];
		if bg.cycle_speed > 0 && self.tick.is_multiple_of((bg.cycle_speed as f64 / 1.5) as usize) { // 1.5 is a nice modifier for this i think
			if bg.cycle1_start < bg.cycle1_end {
				let start = bg.cycle1_start as usize;
				let end = bg.cycle1_end as usize;
				let last = self.palette[end];
				for i in (start..end).rev() {
					self.palette[i + 1] = self.palette[i];
				}
				self.palette[start] = last;
			}
			if bg.cycle2_start < bg.cycle2_end {
				let start = bg.cycle2_start as usize;
				let end = bg.cycle2_end as usize;
				let last = self.palette[end];
				for i in (start..end).rev() {
					self.palette[i + 1] = self.palette[i];
				}
				self.palette[start] = last;
			}
		}

		let palette = &self.palette;
		let framebuf = &self.framebuf;

		let sample = |src_y: usize, src_x: usize| -> Color {
			let offset = match effect.distortion_type {
				DistortionEffect::HORIZONTAL => s(src_y as f64),
				DistortionEffect::INTERLACE => {
					if src_y.is_multiple_of(2) { -s(src_y as f64) } else { s(src_y as f64) }
				}
				_ => s(src_y as f64),
			};

			let line = if effect.distortion_type == DistortionEffect::VERTICAL {
				modulo(offset + (src_y as f64 * comp).floor() as i32, 256)
			} else {
				modulo(src_y as i32, 256)
			};

			let px = if effect.distortion_type == DistortionEffect::HORIZONTAL
				|| effect.distortion_type == DistortionEffect::INTERLACE
			{
				modulo(src_x as i32 + offset, 256)
			} else {
				modulo(src_x as i32, 256)
			};

			let idx = framebuf[line * 256 + px] as usize;
			let [r, g, b] = palette[idx];
			Color::Rgb { r, g, b }
		};

		for (y, row) in cells.iter().enumerate() {
			// we have to subtract these by 12 for.. some reason.
			// if we don't, the image is offset 12 rows upward.
			let top_y = modulo(((y * 2) * 256 / virt_h + (self.scroll_y / Self::SPEED_MOD) as usize) as i32 - 12, 256);
			let bot_y = modulo(((y * 2 + 1) * 256 / virt_h + (self.scroll_y / Self::SPEED_MOD) as usize) as i32 - 12, 256);

			for (x, _) in row.iter().enumerate() {
				let src_x = x * 256 / self.cols + (self.scroll_x / (Self::SPEED_MOD * 2)) as usize;

				let fg = sample(top_y, src_x);
				let bg = sample(bot_y, src_x);

				self.interm.0[y][x] = Cell::default()
					.with_char('▀')
					.with_fg(fg)
					.with_bg(bg);
			}
		}

		self.tick += 1;
		self.interm.take()
	}

	fn is_done(&self) -> bool {
		false
	}

	fn resize(&mut self, w: usize, h: usize) {
		self.orig.resize(w, h);
		self.interm = Frame::with_capacity(w, h);
		self.cols = w;
		self.rows = h;
	}
}
