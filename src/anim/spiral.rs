use std::{f32::consts::TAU, time::Instant};

use cellophane::{Animation, Cell, Frame};
use crossterm::style::Color;
use glam::Vec2;

use crate::{anim::{Gradient, WhoaAnimation, braille_texture, seeded_frame}, get_gradient};

pub struct Spiral {
	orig: Frame,
	gradient: Gradient,
	speed: f32,
	density_clamp: usize,
	start: Instant
}

impl Spiral {
	pub fn new() -> Self {
		let mut new = Self {
			orig: Frame::default(),
			gradient: Gradient::aurora(),
			speed: 1.0,
			density_clamp: 255,
			start: Instant::now()
		};
		new.init();
		new
	}
}

impl Default for Spiral {
	fn default() -> Self {
		log::debug!("Creating new spiral animation");
		Self::new()
	}
}

impl WhoaAnimation for Spiral {
	fn configure(&mut self, config: &toml::Value) {
		let Some(config) = config.get("spiral") else { return };
		self.density_clamp = config.get("density_clamp")
			.unwrap_or(&toml::Value::Integer(255)).as_integer().unwrap_or(255) as usize;

		self.speed = config.get("speed")
			.unwrap_or(&toml::Value::Float(1.0)).as_float().unwrap_or(1.0) as f32;
		let gradient_name = config.get("gradient")
			.cloned()
			.unwrap_or(toml::Value::String("aurora".to_string()));
		let gradient_name = gradient_name.as_str().unwrap_or("aurora");
		let gradient = get_gradient(gradient_name).unwrap_or(Gradient::aurora());
		self.gradient = gradient;
	}
}

impl Animation for Spiral {
	fn init_with(&mut self, initial: cellophane::Frame) {
		log::debug!("Initializing spiral animation");
		self.orig = initial;
	}

	fn update(&mut self) -> cellophane::Frame {
		log::debug!("Updating spiral animation");
		let mut frame = Frame::from_terminal();
		let rows = self.orig.height();
		let cols = self.orig.width();
		let density = braille_texture();

		// time factor
		let t = self.start.elapsed().as_millis() as f32 * 0.0002;
		// minimum dimension
		let m = rows.min(cols) as f32;
		// aspect ratio
		let a = rows as f32 / cols as f32;

		for row in 0..rows {
			for col in 0..cols {
				let Some(cell) = frame.get_cell_mut(row, col) else { continue };

				let x = 2.0 * (col as f32 - cols as f32 / 2.0) / m * a;
				let y = 2.0 * (row as f32 - rows as f32 / 2.0) / m;
				let vector = Vec2::new(x, y);
				let radius = vector.length().max(1e-6);
				let rotation = 0.03 * TAU * t;
				let turn = y.atan2(x) / TAU + rotation;

				let n_sub = 1.5;

				let turn_sub = n_sub * turn % n_sub;

				let k = 0.05;
				let s = k * (50.0 * (radius.powf(0.1) - 0.4 * t)).sin();
				let turn_sine = turn_sub + s;

				let i_turn = ((density.len() as f32 * turn_sine).rem_euclid(density.len() as f32)).floor() as i32;
				let i_radius = (1.5 / (radius * 0.5).powf(0.6) + 5.0 * t) as i32;
				let len = density.len() as f32;
				let phase = (i_turn as f32 + i_radius as f32) * 0.1; // or include time-based phase if you want animation
				let mut density_idx = ((phase.sin() + 1.0) * 0.5 * (len - 1.0)).round() as usize;
				let g = &self.gradient;
				let d = density_idx.max(1) as f32 / density.len() as f32;
				let color = g.sample(d);

				if density_idx > self.density_clamp {
					density_idx = 255;
				}

				*cell = Cell::from(density[density_idx as usize]);
				cell.set_fg(color);
				cell.set_bg(g.bg.unwrap_or(Color::Reset));
			}
		}

		frame
	}

	fn is_done(&self) -> bool { false }

	fn resize(&mut self, w: usize, h: usize) {
		self.orig.resize(w, h);
	}
}
