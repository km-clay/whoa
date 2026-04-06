use cellophane::{Animation, Cell, Frame};
use crossterm::style::Color;
use noise::{NoiseFn, Perlin};

use crate::{anim::{Gradient, WhoaAnimation, braille_texture}, get_gradient};

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

impl WhoaAnimation for PerlinNoise {
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

		let gradient = get_gradient(gradient_name).unwrap_or(Gradient::aurora());
		self.gradient = gradient;
	}
}

impl Animation for PerlinNoise {
	fn init(&mut self, initial: Frame) {
		let (rows,cols) = initial.dims().unwrap_or((0,0));
		self.interm = initial.clone();
		self.orig = initial;
		self.rows = rows;
		self.cols = cols;
	}

	fn update(&mut self, dt: std::time::Duration) -> Frame {
		let seconds = dt.as_secs_f32();
		let cells = self.orig.cells();

		for (y, row) in cells.iter().enumerate() {
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
					cell.set_fg(color);
					cell.set_bg(g.bg.unwrap_or(Color::Reset));
					let Some(interm_cell) = self.interm.get_cell_mut(y, x) else { continue };
					*interm_cell = cell;
				} else {
					let cell = Cell::default().with_bg(g.bg.unwrap_or(Color::Reset));
					let Some(interm_cell) = self.interm.get_cell_mut(y, x) else { continue };

					*interm_cell = cell;
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
