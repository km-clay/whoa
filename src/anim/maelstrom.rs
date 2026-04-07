use std::{ops::Add, time::{Instant}};

use glam::{Mat2, Vec2};

use crate::anim::{Animation, WhoaAnimation, Frame, seeded_frame, with_dev_coords};

pub struct Maelstrom {
	orig: Frame,
	interm: Frame,
	wait_time: f32,
	speed_min: f32,
	speed_max: f32,
	rows: usize,
	cols: usize,
	start: Instant,

	center: Vec2
}

impl Maelstrom {
	pub fn new() -> Self {
		let x = 0f32;
		let y = 0f32;
		let center = Vec2::new(x, y);
		Self {
			orig: Default::default(),
			interm: Default::default(),
			wait_time: 1.0,
			speed_min: 1.0,
			speed_max: 1.0,
			rows: 0,
			cols: 0,
			start: Instant::now(),
			center
		}
	}
}

impl Default for Maelstrom {
	fn default() -> Self {
		Self::new()
	}
}

impl WhoaAnimation for Maelstrom {
	fn configure(&mut self, config: &toml::Value) {
		let Some(config) = config.get("maelstrom") else { return };
		self.speed_min = config.get("speed_min")
			.unwrap_or(&toml::Value::Float(1.0)).as_float().unwrap_or(1.0) as f32;
		self.speed_max = config.get("speed_max")
			.unwrap_or(&toml::Value::Float(1.0)).as_float().unwrap_or(1.0) as f32;
		self.wait_time = config.get("wait_time")
			.unwrap_or(&toml::Value::Float(1.0)).as_float().unwrap_or(1.0) as f32;
	}
}

impl Animation for Maelstrom {
	fn init(&mut self, initial: Frame) {
		let (rows,cols) = initial.dims().unwrap_or((0,0));
		self.orig = initial.clone();
		self.interm = initial;
		self.rows = rows;
		self.cols = cols;
	}

	fn initial_frame(&self) -> Frame { seeded_frame() }

	fn update(&mut self) -> Frame {
		if self.start.elapsed().as_secs_f32() < self.wait_time {
			return self.interm.clone();
		}
		let cells = self.orig.cells();
		let seconds = self.start.elapsed().as_secs_f32() - self.wait_time;

		for (y,row) in cells.iter().enumerate() {
			for (x,_) in row.iter().enumerate() {
				let v = Vec2::new(x as f32, y as f32);
				let s = Vec2::new(self.cols as f32, self.rows as f32);
				let res = with_dev_coords(v, s, |dev_v| {
					let centered = dev_v - self.center;
					let speed = (0.0025 * self.speed_min) + ((0.175 * self.speed_max) * (seconds / 15.0).min(1.0));
					let angle = (speed * seconds) / centered.length().max(0.1);
					let r_v = Mat2::from_angle(angle) * centered;
					r_v.add(self.center)
				});
				let (res_x, res_y) = (res.x.round() as usize, res.y.round() as usize);
				if res_x < self.cols && res_y < self.rows {
					let Some(cell) = self.interm.get_cell_mut(y, x) else { continue };
					*cell = cells[res_y][res_x].clone();
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
		self.interm = Frame::with_capacity(w, h);
		self.rows = h;
		self.cols = w;
	}
}
