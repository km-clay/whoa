use crate::anim::{Animation, WhoaAnimation, Frame, seeded_frame};

pub struct Cosine {
	orig: Frame,
	interm: Frame,
	speed: f64,
	rows: usize,
	cols: usize,
}

impl Cosine {
	pub fn new() -> Self {
		Self {
			orig: Frame::default(),
			interm: Frame::default(),
			speed: 1.0,
			rows: 0,
			cols: 0
		}
	}
}

impl Default for Cosine {
	fn default() -> Self {
		Self::new()
	}
}

impl WhoaAnimation for Cosine {
	fn configure(&mut self, config: &toml::Value) {
		let Some(config) = config.get("cosine") else { return };
		self.speed = config.get("speed")
			.unwrap_or(&toml::Value::Float(1.0)).as_float().unwrap_or(1.0);
	}
}

impl Animation for Cosine {
	fn initial_frame(&self) -> Frame { seeded_frame() }
	fn init(&mut self, initial: Frame) {
		let (rows,cols) = initial.dims().unwrap_or((0,0));
		self.orig = initial.clone();
		self.interm = initial;
		self.rows = rows;
		self.cols = cols;
	}

	#[allow(clippy::needless_range_loop)]
	fn update(&mut self, dt: std::time::Duration) -> Frame {
		let Frame(ref cells) = self.orig;
		let seconds = dt.as_secs_f64();

		for col in 0..self.cols {
			let wave = ((col as f64 / self.cols as f64) * std::f64::consts::PI * 4.0 + (seconds * self.speed)).cos();
			let factor = ((wave + 1.0) / 8.0) * ((seconds * self.speed) / 20.0) * self.rows as f64;
			let offset = factor as usize;

			for row in 0..self.rows {
				self.interm.0[(row + offset) % self.rows][col] = cells[row][col].clone();
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
		self.rows = h;
		self.cols = w;
	}
}
