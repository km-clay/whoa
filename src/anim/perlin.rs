use crossterm::style::Color;
use glam::Vec3;
use noise::{NoiseFn, Perlin};

use crate::{GRADIENTS, anim::{Animation, Cell, Frame, braille_texture}};

#[derive(Clone, Debug)]
pub struct Gradient {
	pub bg: Option<Color>,
	pub stops: Vec<Vec3>
}

impl Gradient {
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


pub struct PerlinNoise {
	orig: Frame,
	interm: Frame,
	gradient: Gradient,
	noise: Perlin,
	texture: Vec<char>,
	speed: f32,
	scale: f64,
	x_delta: f64,
	y_delta: f64,
	rows: usize,
	cols: usize
}

impl PerlinNoise {
	pub fn new() -> Self {
		let now = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_secs() as u32;
		let texture = braille_texture().to_vec();
		let x_delta = rand::random::<f64>() - 0.5;
		let y_delta = rand::random::<f64>() - 0.5;
		Self {
			orig: Default::default(),
			interm: Default::default(),
			gradient: Gradient::aurora(),
			noise: Perlin::new(now),
			speed: 0.30,
			scale: 1.0,
			texture,
			x_delta,
			y_delta,
			rows: 0,
			cols: 0
		}
	}
}

impl Default for PerlinNoise {
	fn default() -> Self {
		Self::new()
	}
}

impl Animation for PerlinNoise {
	fn configure(&mut self, config: &toml::Value) {
		let Some(config) = config.get("perlin") else { return };
		self.speed = config.get("speed")
			.unwrap_or(&toml::Value::Float(0.30)).as_float().unwrap_or(0.30) as f32;
		self.scale = config.get("scale")
			.unwrap_or(&toml::Value::Float(1.0)).as_float().unwrap_or(1.0);
		let gradient_name = config.get("gradient")
			.cloned()
			.unwrap_or(toml::Value::String("aurora".to_string()));
		let gradient_name = gradient_name.as_str().unwrap_or("aurora");

		let gradient = GRADIENTS.get(gradient_name).cloned().unwrap_or(Gradient::aurora());
		self.gradient = gradient;
	}

	fn init(&mut self, initial: Frame) {
		let (rows,cols) = initial.dims().unwrap_or((0,0));
		self.interm = initial.clone();
		self.orig = initial;
		self.rows = rows;
		self.cols = cols;
	}

	fn update(&mut self, dt: std::time::Duration) -> Frame {
		let seconds = dt.as_secs_f32();

		for (y, row) in self.orig.0.iter().enumerate() {
			for (x, _) in row.iter().enumerate() {
				let nx = x as f64 / self.cols as f64 * self.scale + seconds as f64 * self.speed as f64 * self.y_delta;
				let ny = y as f64 / self.rows as f64 * self.scale + seconds as f64 * self.speed as f64 * self.x_delta;
				let value = self.noise.get([nx,ny]) + 0.2;
				let index = (value * self.texture.len() as f64) as i32;

				let t = index as f32 / 255.0;
				let g = &self.gradient;
				let color = g.sample(t);

				let index = index.clamp(0,self.texture.len() as i32 - 1);
				if index >= 0 {
					let mut cell = Cell::from(self.texture[index as usize]);
					cell.fg = color;
					cell.bg = g.bg.unwrap_or(Color::Reset);
					self.interm.0[y][x] = cell;
				} else {
					let cell = Cell { bg: g.bg.unwrap_or(Color::Reset), ..Default::default() };
					self.interm.0[y][x] = cell;
				}
			}
		}

		self.interm.take()
	}

	fn is_done(&self) -> bool {
		false
	}

	fn resize(&mut self, w: usize, h: usize) {
		self.orig.resize(w, h);
		self.interm.resize(w, h);
	}
}
