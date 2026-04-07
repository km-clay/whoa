use std::{str::FromStr, time::{Instant}};
use rand::Rng;

use crate::anim::{Animation, WhoaAnimation, Frame, seeded_frame};

#[derive(Default,Debug,Clone,Copy)]
enum Direction {
	#[default]
	Down,
	Up,
	Left,
	Right,
	Random
}

impl Direction {
	fn resolve(self) -> Self {
		if let Direction::Random = self {
			match rand::thread_rng().gen_range(0..4u8) {
				0 => Direction::Down,
				1 => Direction::Up,
				2 => Direction::Left,
				_ => Direction::Right,
			}
		} else {
			self
		}
	}
}

impl FromStr for Direction {
	type Err = String;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"up" => Ok(Self::Up),
			"down" => Ok(Self::Down),
			"left" => Ok(Self::Left),
			"right" => Ok(Self::Right),
			"random" => Ok(Self::Random),
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

impl WhoaAnimation for Collapse {
	fn configure(&mut self, config: &toml::Value) {
		let Some(config) = config.get("collapse") else { return };
		let Some(direction) = config.get("direction") else { return };
		let Some(direction) = direction.as_str() else { return };

		match direction.parse::<Direction>() {
			Ok(dir) => self.direction = dir,
			Err(e) => log::error!("Failed to parse direction: {e}")
		};
	}
}

impl Animation for Collapse {
	fn initial_frame(&self) -> Frame { seeded_frame() }
	fn init_with(&mut self, initial: Frame) {
		self.frame = initial;
	}
	fn update(&mut self) -> Frame {
		if self.last_tick.elapsed().as_millis() <= 100 {
			return self.frame.clone();
		}
		let Some((rows, cols)) = self.frame.dims() else {
			return self.frame.clone();
		};
		let mut moved = false;
		let mut cells = self.frame.take().into_cells();
		let direction = self.direction.resolve();
		match direction {
			Direction::Down => {
				for row in (1..rows).rev() {
					for col in 0..cols {
						let (upper, lower) = cells.split_at_mut(row);
						let above = &mut upper[row - 1][col];
						let this = &mut lower[0][col];
						if this.is_empty() {
							moved = true;
							std::mem::swap(this, above);
						}
					}
				}
			}
			Direction::Up => {
				for row in 0..rows.saturating_sub(1) {
					for col in 0..cols {
						let (upper, lower) = cells.split_at_mut(row + 1);
						let this = &mut upper[row][col];
						let below = &mut lower[0][col];
						if this.is_empty() {
							moved = true;
							std::mem::swap(this, below);
						}
					}
				}
			}
			Direction::Right => {
				for col in (1..cols).rev() {
					for row in 0..rows {
						let left = &cells[row][col - 1];
						let right = &cells[row][col];
						if right.is_empty() && !left.is_empty() {
							moved = true;
							cells[row].swap(col - 1, col);
						}
					}
				}
			}
			Direction::Left => {
				for col in 0..cols.saturating_sub(1) {
					for row in 0..rows {
						let left = &cells[row][col];
						let right = &cells[row][col + 1];
						if left.is_empty() && !right.is_empty() {
							moved = true;
							cells[row].swap(col, col + 1);
						}
					}
				}
			}
			Direction::Random => unreachable!()
		}

		if !moved {
			self.ticks_without_motion += 1;
		} else {
			self.ticks_without_motion = 0;
		}

		self.last_tick = Instant::now();
		self.frame = Frame::from_cells(cells);
		self.frame.clone()
	}

	fn resize(&mut self, w: usize, h: usize) {
		self.frame.resize(w, h);
	}

	fn is_done(&self) -> bool {
		self.ticks_without_motion >= 50
	}
}
