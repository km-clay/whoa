// Ported from https://play.ertdfgcvb.xyz/#/src/contributed/slime_dish
// Original author: zspotter (IG @zzz_desu, TW @zspotter)
// Low-res physarum slime mold simulation
//
// With inspiration from:
// - https://sagejenson.com/physarum
// - https://uwe-repository.worktribe.com/output/980579
// - http://www.tech-algorithm.com/articles/nearest-neighbor-image-scaling

use std::{f64::consts::PI, time::Instant};

use cellophane::{Cell, Frame};
use crossterm::style::Color;
use glam::Vec2;

use crate::get_gradient;
use crate::anim::{Animation, WhoaAnimation, Cursor};
use crate::anim::Gradient;

const NUM_AGENTS: usize = 1500;

pub struct SlimeMold {
	ticks: usize,
	zoom_tick: usize,
	last_tick: Instant,
	sim: SlimeSim,
	rows: usize,
	cols: usize,
	gradient: Gradient,
	interm: Frame
}

impl SlimeMold {
	pub fn new() -> Self {
		Self {
			ticks: 0,
			zoom_tick: 0,
			last_tick: Instant::now(),
			sim: SlimeSim::new(0, 0),
			rows: 0,
			cols: 0,
			gradient: Gradient::aurora(),
			interm: Frame::default(),
		}
	}
}

impl Default for SlimeMold {
	fn default() -> Self {
		Self::new()
	}
}

impl WhoaAnimation for SlimeMold {
	fn configure(&mut self, config: &toml::Value) {
		let Some(config) = config.get("slime") else { return };
		let gradient_name = config.get("gradient")
			.cloned()
			.unwrap_or(toml::Value::String("aurora".to_string()));
		let gradient_name = gradient_name.as_str().unwrap_or("aurora");

		let gradient = get_gradient(gradient_name).unwrap_or(Gradient::aurora());
		self.gradient = gradient;
	}
}

impl Animation for SlimeMold {
	fn init(&mut self, initial: Frame) {
		let (rows, cols) = initial.dims().unwrap_or((0, 0));
		self.rows = rows;
		self.cols = cols;
		self.sim = SlimeSim::new(rows, cols);
		self.interm = Frame::with_capacity(cols, rows);
	}

	fn update(&mut self) -> Frame {
		if self.last_tick.elapsed().as_millis() < 25 {
			return self.interm.clone();
		}

		self.last_tick = Instant::now();
		let mut cursor = Cursor::default();

		let zoom = f64::sin(self.ticks as f64 / 80.0);
		if zoom > 0.3 {
			if self.zoom_tick == 0 {
				self.zoom_tick = self.ticks;
			}

			cursor.pressed = true;

			let zoom_x = f64::cos(self.zoom_tick as f64 / 180.0) * (self.cols as f64 / 4.0);
			let zoom_y = f64::sin(self.zoom_tick as f64 / 180.0) * (self.rows as f64 / 4.0);

			cursor.pos.x = self.cols as f32 / 2.0 + zoom_x as f32;
			cursor.pos.y = self.rows as f32 / 2.0 + zoom_y as f32;
		} else {
			self.zoom_tick = 0;
		}

		self.sim.step(&cursor);
		self.sim.render(&mut self.interm, &self.gradient);
		self.ticks += 1;

		self.interm.clone()
	}

	fn is_done(&self) -> bool {
		false
	}

	fn resize(&mut self, w: usize, h: usize) {
		self.rows = h;
		self.cols = w;
		self.sim.rows = h;
		self.sim.cols = w;
		self.interm = Frame::with_capacity(w, h);
	}
}

pub struct Agent {
	pos: Vec2,
	dir: Vec2,
	scatter: bool,
}

impl Agent {
	fn sense(&self, m: f64, chem: &[f64]) -> f64 {
		let angle = m * SlimeSim::SENS_ANGLE;
		let cos_a = angle.cos() as f32;
		let sin_a = angle.sin() as f32;
		let sense_vec = Vec2::new(
			self.dir.x * cos_a - self.dir.y * sin_a,
			self.dir.x * sin_a + self.dir.y * cos_a,
		) * SlimeSim::SENS_DIST as f32;

		let pos = Vec2::new(
			(self.pos.x + sense_vec.x).floor(),
			(self.pos.y + sense_vec.y).floor(),
		);

		if !bounded(pos) {
			return -1.0;
		}

		let idx = pos.y as i32 * SlimeSim::HEIGHT as i32 + pos.x as i32;
		if idx < 0 || idx as usize >= chem.len() {
			return -1.0;
		}

		let sensed = chem[idx as usize];
		if self.scatter {
			1.0 - sensed
		} else {
			sensed
		}
	}

	fn react(&mut self, chem: &[f64]) {
		let forward_chem = self.sense(0.0, chem);
		let left_chem = self.sense(-1.0, chem);
		let right_chem = self.sense(1.0, chem);

		let rotate: f64 = if forward_chem > left_chem && forward_chem > right_chem {
			0.0
		} else if forward_chem < left_chem && forward_chem < right_chem {
			if rand::random::<f64>() < 0.5 {
				-SlimeSim::AGT_ANGLE
			} else {
				SlimeSim::AGT_ANGLE
			}
		} else if left_chem < right_chem {
			SlimeSim::AGT_ANGLE
		} else if right_chem < left_chem {
			-SlimeSim::AGT_ANGLE
		} else if forward_chem < 0.0 {
			PI / 2.0
		} else {
			0.0
		};

		let cos_r = rotate.cos() as f32;
		let sin_r = rotate.sin() as f32;
		self.dir = Vec2::new(
			self.dir.x * cos_r - self.dir.y * sin_r,
			self.dir.x * sin_r + self.dir.y * cos_r,
		);

		self.pos += self.dir * SlimeSim::AGT_SPEED as f32;
	}

	fn deposit(&self, chem: &mut [f64]) {
		let x = self.pos.x.floor() as i32;
		let y = self.pos.y.floor() as i32;
		let idx = y * SlimeSim::HEIGHT as i32 + x;
		if idx < 0 || idx as usize >= chem.len() {
			return;
		}
		chem[idx as usize] = (chem[idx as usize] + SlimeSim::DEPOSIT).min(1.0);
	}
}

fn bounded(v: Vec2) -> bool {
	let r = f32::min(SlimeSim::WIDTH as f32, SlimeSim::HEIGHT as f32) / 2.0;
	let dx = v.x - r;
	let dy = v.y - r;
	dx * dx + dy * dy <= r * r
}

fn blur(row: i32, col: i32, data: &[f64]) -> f64 {
	let mut sum = 0.0;
	for dy in -1..=1 {
		for dx in -1..=1 {
			let idx = (row + dy) * SlimeSim::HEIGHT as i32 + col + dx;
			if idx >= 0 && (idx as usize) < data.len() {
				sum += data[idx as usize];
			}
		}
	}
	sum / 9.0
}

fn rand_circle() -> Vec2 {
	let r = rand::random::<f64>().sqrt() as f32;
	let theta = rand::random::<f64>() as f32 * 2.0 * std::f32::consts::PI;
	Vec2::new(r * theta.cos(), r * theta.sin())
}

pub struct SlimeSim {
	ticks: usize,
	rows: usize,
	cols: usize,
	chem: Vec<f64>,
	wip: Vec<f64>,
	agents: Vec<Agent>,
	view_scale: Vec2,
	view_focus: Vec2,
}

impl SlimeSim {
	const HEIGHT: usize = 400;
	const WIDTH: usize = 400;

	const DECAY: f64 = 0.9;
	const MIN_CHEM: f64 = 0.0001;

	const ASPECT: f64 = 0.5017144097222223;
	const SENS_ANGLE: f64 = 45.0 * PI / 180.0;
	const SENS_DIST: f64 = 9.0;

	const AGT_SPEED: f64 = 1.0;
	const AGT_ANGLE: f64 = 45.0 * PI / 180.0;
	const DEPOSIT: f64 = 1.0;

	const TEXTURE0: [char; 6] = [' ', ' ', '`', '`', '^', '@'];
	const TEXTURE1: [char; 6] = [' ', '.', '.', '„', 'v', '0'];

	pub fn new(rows: usize, cols: usize) -> Self {
		let chem = vec![0.0; Self::HEIGHT * Self::WIDTH];
		let wip = vec![0.0; Self::HEIGHT * Self::WIDTH];

		let agents: Vec<Agent> = (0..NUM_AGENTS)
			.map(|_| {
				let pos = (rand_circle() * 0.5 + Vec2::ONE * 0.5) * Self::WIDTH as f32;
				let angle = rand::random::<f64>() as f32 * 2.0 * std::f32::consts::PI;
				Agent {
					pos,
					dir: Vec2::new(angle.cos(), angle.sin()),
					scatter: false,
				}
			})
			.collect();

		let mut s = Self {
			ticks: 0,
			rows,
			cols,
			chem,
			wip,
			agents,
			view_scale: Vec2::ONE,
			view_focus: Vec2::new(0.5, 0.5),
		};

		s.update_view(&Cursor::default());
		s
	}

	fn update_view(&mut self, cursor: &Cursor) {
		let target_scale = if cursor.pressed {
			Vec2::new(1.0, 1.0 / Self::ASPECT as f32)
		} else if (self.rows as f64) / Self::ASPECT < self.cols as f64 {
			// Fit to wide window
			let base = 1.1 * Self::WIDTH as f64 / self.rows as f64;
			Vec2::new(
				(base * Self::ASPECT) as f32,
				base as f32,
			)
		} else {
			// Fit to tall window
			let base = 1.1 * Self::WIDTH as f64 / self.cols as f64;
			Vec2::new(
				base as f32,
				(base / Self::ASPECT) as f32,
			)
		};

		// Smooth transition to new scale
		self.view_scale += 0.1 * (target_scale - self.view_scale);

		let target_focus = if cursor.pressed {
			Vec2::new(
				cursor.pos.x / self.cols as f32,
				cursor.pos.y / self.rows as f32,
			)
		} else {
			Vec2::new(0.5, 0.5)
		};

		// Smooth transition to new focus
		self.view_focus += 0.1 * (target_focus - self.view_focus);
	}

	pub fn step(&mut self, cursor: &Cursor) {
		// Diffuse & decay
		for row in 0..Self::HEIGHT as i32 {
			for col in 0..Self::WIDTH as i32 {
				let val = Self::DECAY * blur(row, col, &self.chem);
				let val = if val < Self::MIN_CHEM { 0.0 } else { val };
				self.wip[(row as usize) * Self::HEIGHT + col as usize] = val;
			}
		}

		// Swap buffers
		std::mem::swap(&mut self.chem, &mut self.wip);

		// Sense, rotate, and move agents
		let is_scattering = f64::sin(self.ticks as f64 / 150.0) > 0.8;
		for agent in &mut self.agents {
			agent.scatter = is_scattering;
			agent.react(&self.chem);
		}

		// Deposit by agents
		for agent in &self.agents {
			agent.deposit(&mut self.chem);
		}

		// Update view parameters
		self.update_view(cursor);

		self.ticks += 1;
	}

	fn render_cell(&self, row: usize, col: usize) -> (char, f32) {
		let offset_x = (self.view_focus.x as f64
			* (Self::WIDTH as f64 - self.view_scale.x as f64 * self.cols as f64))
			.floor();
		let offset_y = (self.view_focus.y as f64
			* (Self::HEIGHT as f64 - self.view_scale.y as f64 * self.rows as f64))
			.floor();

		let x = col as f64;
		let y = row as f64;

		let sample_from = Vec2::new(
			(offset_x + (x * self.view_scale.x as f64).floor()) as f32,
			(offset_y + (y * self.view_scale.y as f64).floor()) as f32,
		);

		let sample_to = Vec2::new(
			(offset_x + ((x + 1.0) * self.view_scale.x as f64).floor()) as f32,
			(offset_y + ((y + 1.0) * self.view_scale.y as f64).floor()) as f32,
		);

		if !bounded(sample_from) || !bounded(sample_to) {
			return (' ', 0.0);
		}

		let sample_h = f64::max(1.0, (sample_to.y - sample_from.y) as f64);
		let sample_w = f64::max(1.0, (sample_to.x - sample_from.x) as f64);

		let mut max_val: f64 = 0.0;
		let mut sum_val: f64 = 0.0;

		let sx = sample_from.x as f64;
		let sy = sample_from.y as f64;
		let mut cx = sx;
		while cx < sx + sample_w {
			let mut cy = sy;
			while cy < sy + sample_h {
				let idx = cy as usize * Self::HEIGHT + cx as usize;
				if idx < self.chem.len() {
					let val = self.chem[idx];
					max_val = max_val.max(val);
					sum_val += val;
				}
				cy += 1.0;
			}
			cx += 1.0;
		}

		let mut val = sum_val / (sample_w * sample_h);
		val = (val + max_val) / 2.0;
		val = val.powf(1.0 / 3.0);

		let tex_row = (col + row) % 2;
		let tex_col = (val * (Self::TEXTURE0.len() - 1) as f64).ceil() as usize;
		let tex_col = tex_col.min(Self::TEXTURE0.len() - 1);

		let ch = if tex_row == 0 {
			Self::TEXTURE0[tex_col]
		} else {
			Self::TEXTURE1[tex_col]
		};
		(ch, val as f32)
	}

	pub fn render(&self, out: &mut Frame, gradient: &Gradient) {
		for row in 0..self.rows {
			for col in 0..self.cols {
				let (ch, val) = self.render_cell(row, col);
				let color = gradient.sample(val);
				let mut cell = Cell::from(ch);
				cell.set_fg(color);
				cell.set_bg(gradient.bg.unwrap_or(Color::Reset));
				let Some(out_cell) = out.get_cell_mut(row, col) else { continue };
				*out_cell = cell;
			}
		}
	}
}
