use std::{str::FromStr, time::{Duration, Instant}};
use crate::anim::{Animation, Frame};

#[derive(Default,Debug)]
enum Direction {
	#[default]
	Down,
	Up,
	Left,
	Right
}

impl FromStr for Direction {
	type Err = String;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"up" => Ok(Self::Up),
			"down" => Ok(Self::Down),
			"left" => Ok(Self::Left),
			"right" => Ok(Self::Right),
			_ => Err(format!("Invalid direction: {s}"))
		}
	}
}

pub struct Collapse {
	frame: Frame,
	last_tick: Instant,
	direction: Direction,
	ticks_without_motion: usize
}

impl Collapse {
	pub fn new() -> Self {
		Self {
			frame: Default::default(),
			last_tick: Instant::now(),
			direction: Default::default(),
			ticks_without_motion: 0
		}
	}
}

impl Default for Collapse {
	fn default() -> Self {
		Self::new()
	}
}

impl Animation for Collapse {
	fn configure(&mut self, config: &toml::Value) {
		let Some(config) = config.get("collapse") else { return };
		let Some(direction) = config.get("direction") else { return };
		let Some(direction) = direction.as_str() else { return };

		match direction.parse::<Direction>() {
			Ok(dir) => self.direction = dir,
			Err(e) => log::error!("Failed to parse direction: {e}")
		};
	}
	fn initial_frame(&self) -> Frame { Frame::seeded() }
	fn init(&mut self, initial: Frame) {
		self.frame = initial;
	}
	fn update(&mut self, _dt: Duration) -> Frame {
		if self.last_tick.elapsed().as_millis() <= 100 {
			return self.frame.clone();
		}
		let Some((rows, cols)) = self.frame.dims() else {
			return self.frame.clone();
		};
		let mut moved = false;
		let Frame(mut cells) = self.frame.clone();
		for row in (1..rows).rev() {
			for col in 0..cols {
				let (upper,lower) = cells.split_at_mut(row);
				let above = &mut upper[row - 1][col];
				let this = &mut lower[0][col];
				if this.is_empty() {
					moved = true;
					std::mem::swap(this,above);
				}
			}
		}

		if !moved {
			self.ticks_without_motion += 1;
		} else {
			self.ticks_without_motion = 0;
		}

		self.last_tick = Instant::now();
		self.frame = Frame(cells);
		self.frame.clone()
	}

	fn resize(&mut self, w: usize, h: usize) {
		self.frame.resize(w, h);
	}

	fn is_done(&self) -> bool {
		self.ticks_without_motion >= 50
	}
}
