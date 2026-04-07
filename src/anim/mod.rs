use std::ops::{Div, Mul};
use cellophane::{Animation, Frame, FrameBuilder};

use crossterm::style::Color;
use glam::{Vec2, Vec3};
use toml::Value;

use crate::pull_seed_content;

pub mod saturn;
pub mod slime;
pub mod cos;
pub mod perlin;
pub mod collapse;
pub mod maelstrom;
pub mod conway;

#[derive(Clone, Debug)]
pub struct Gradient {
	pub bg: Option<Color>,
	pub stops: Vec<Vec3>
}

impl Gradient {
	pub fn from_value(val: &Value) -> anyhow::Result<Self> {
		let Some(cfg) = val.as_table() else {
			anyhow::bail!("Gradient config must be a table");
		};
		let bg = cfg.get("bg").and_then(|bg| bg.as_array()).cloned().unwrap_or({
			vec![
				Value::Integer(0),
				Value::Integer(0),
				Value::Integer(0),
			]
		});

		let u8_range = 0..256;

		let Value::Integer(r) = bg.get(0).unwrap_or(&Value::Integer(0)) else {
			anyhow::bail!("Gradient bg must be an array of 3 integers");
		};
		let Value::Integer(g) = bg.get(1).unwrap_or(&Value::Integer(0)) else {
			anyhow::bail!("Gradient bg must be an array of 3 integers");
		};
		let Value::Integer(b) = bg.get(2).unwrap_or(&Value::Integer(0)) else {
			anyhow::bail!("Gradient bg must be an array of 3 integers");
		};
		if !u8_range.contains(r) || !u8_range.contains(g) || !u8_range.contains(b) {
			anyhow::bail!("Gradient bg color values must be between 0 and 255");
		}
		let bg = Color::Rgb { r: *r as u8, g: *g as u8, b: *b as u8 };

		let Some(stops_cfg) = cfg.get("stops").and_then(|s| s.as_array()) else {
			anyhow::bail!("Gradient config must have a stops array");
		};

		let mut stops = vec![];
		for stop in stops_cfg {
			let Value::Array(stop) = stop else {
				anyhow::bail!("Gradient stops must be arrays of 3 integers");
			};

			let Value::Integer(r) = stop.get(0).unwrap_or(&Value::Integer(0)) else {
				anyhow::bail!("Gradient stops must be arrays of 3 integers");
			};
			let Value::Integer(g) = stop.get(1).unwrap_or(&Value::Integer(0)) else {
				anyhow::bail!("Gradient stops must be arrays of 3 integers");
			};
			let Value::Integer(b) = stop.get(2).unwrap_or(&Value::Integer(0)) else {
				anyhow::bail!("Gradient stops must be arrays of 3 integers");
			};
			if !u8_range.contains(r) || !u8_range.contains(g) || !u8_range.contains(b) {
				anyhow::bail!("Gradient stop color values must be between 0 and 255");
			}
			stops.push(Vec3 { x: *r as f32, y: *g as f32, z: *b as f32 });
		}

		Ok(Self { bg: Some(bg), stops })
	}
	pub fn ocean() -> Self {
		Gradient {
			bg: Some(Color::Rgb { r: 0, g: 0, b: 20 }),
			stops: vec![
				Vec3 { x: 0.0, y: 0.0, z: 50.0 },
				Vec3 { x: 0.0, y: 120.0, z: 200.0 },
				Vec3 { x: 200.0, y: 240.0, z: 255.0 }
			]
		}
	}
	pub fn fire() -> Self {
		Gradient {
			bg: Some(Color::Rgb { r: 10, g: 0, b: 0 }),
			stops: vec![
				Vec3 { x: 20.0, y: 0.0, z: 0.0 },
				Vec3 { x: 200.0, y: 30.0, z: 0.0 },
				Vec3 { x: 255.0, y: 150.0, z: 0.0 },
				Vec3 { x: 255.0, y: 255.0, z: 100.0 },
			]
		}
	}
	pub fn vapor() -> Self {
		Gradient {
			bg: Some(Color::Rgb { r: 15, g: 0, b: 30 }),
			stops: vec![
				Vec3 { x: 40.0, y: 0.0, z: 80.0 },
				Vec3 { x: 200.0, y: 0.0, z: 150.0 },
				Vec3 { x: 255.0, y: 100.0, z: 200.0 },
				Vec3 { x: 0.0, y: 255.0, z: 255.0 },
			]
		}
	}
	pub fn mono() -> Self {
		Gradient {
			bg: Some(Color::Rgb { r: 0, g: 0, b: 0 }),
			stops: vec![
				Vec3 { x: 20.0, y: 20.0, z: 20.0 },
				Vec3 { x: 255.0, y: 255.0, z: 255.0 },
			]
		}
	}
	pub fn aurora() -> Self {
		Gradient {
			bg: Some(Color::Rgb { r: 0, g: 0, b: 15 }),
			stops: vec![
				Vec3 { x: 0.0, y: 0.0, z: 40.0 },
				Vec3 { x: 0.0, y: 200.0, z: 100.0 },
				Vec3 { x: 0.0, y: 150.0, z: 200.0 },
				Vec3 { x: 150.0, y: 0.0, z: 200.0 },
			]
		}
	}
	pub fn sample(&self, t: f32) -> Color {
		let t = t.clamp(0.0, 1.0);
		let segments = (self.stops.len() - 1) as f32;
		let scaled = t * segments;
		let i = (scaled as usize).min(self.stops.len() - 2);
		let local_t = scaled - i as f32;
		let c = self.stops[i].lerp(self.stops[i + 1], local_t);
		Color::Rgb { r: c.x as u8, g: c.y as u8, b: c.z as u8 }
	}
}



fn braille_texture() -> [char; 256] {
	let mut chars: [(char, u32); 256] = [(char::default(), 0); 256];
	let mut i = 0;
	while i < 256 {
		let c = char::from_u32(0x2800 + i as u32).unwrap();
		chars[i] = (c, (i as u32).count_ones());
		i += 1;
	}
	chars.sort_by_key(|&(_, dots)| dots);
	let mut result = [' '; 256];
	let mut i = 0;
	while i < 256 {
		result[i] = chars[i].0;
		i += 1;
	}
	result
}

pub fn to_device(v: Vec2) -> Vec2 {
	// Normalize screen coordinates from [0,1] range to [-1,1] range
	// Y is flipped because the top of the screen is 0
	Vec2 {
		x: (2.0 * v.x) - 1.0,
		y: 1.0 - (2.0 * v.y)
	}
}

pub fn from_device(v: Vec2) -> Vec2 {
	Vec2 {
		x: (v.x + 1.0) / 2.0,
		y: 1.0 - (v.y + 1.0) / 2.0
	}
}

pub fn with_dev_coords<F>(v: Vec2, s: Vec2, f: F) -> Vec2
where F: FnOnce(Vec2) -> Vec2 {
	let norm = v.div(s);
	let dev = to_device(norm);

	let res = f(dev);

	from_device(res).mul(s)
}

pub trait WhoaAnimation: Animation {
	fn configure(&mut self, config: &toml::Value);
}

#[derive(Default,Clone,Debug)]
pub struct Cursor {
	pub pressed: bool,
	pub pos: Vec2
}

pub fn seeded_frame() -> Frame {
	let content = pull_seed_content();
	let (cols,rows) = crossterm::terminal::size().unwrap_or((80, 24));
	let mut builder = FrameBuilder::new(cols as usize, rows as usize);
	builder.feed_bytes(content.as_bytes());
	let mut frame = builder.build();
	frame.resize(cols as usize, rows as usize);
	frame
}
