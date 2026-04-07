use std::{collections::HashMap, hash::{DefaultHasher, Hash, Hasher}, time::{Instant}};

use cellophane::{Animation, Cell, Frame};
use rand::Rng;

use crate::anim::{WhoaAnimation, seeded_frame};

pub struct Conway {
	last_tick: Instant,
	state_cache: HashMap<u64,usize>,
	tick_rate: usize,
	stale_ticks: usize,
	is_stale: bool,
	frame: Frame
}

impl Conway {

	pub fn new() -> Self {
		Self {
			last_tick: Instant::now(),
			state_cache: HashMap::new(),
			tick_rate: 10,
			stale_ticks: 15,
			is_stale: false,
			frame: Default::default()
		}
	}

	pub fn reproduce(cells: &[Cell]) -> Cell {
		let [parent1, parent2, parent3] = cells else { panic!("Exactly 3 cells must be provided") };
		let glyphs = [
			parent1.ch().clone(),
			parent2.ch().clone(),
			parent3.ch().clone()
		];
		let fg_colors = [
			parent1.fg(),
			parent2.fg(),
			parent3.fg()
		];
		let bg_colors = [
			parent1.bg(),
			parent2.bg(),
			parent3.bg()
		];
		let flags = [
			parent1.flags(),
			parent2.flags(),
			parent3.flags()
		];
		let glyph_i: usize = rand::thread_rng().gen_range(0..3);
		let fg_i: usize = rand::thread_rng().gen_range(0..3);
		let bg_i: usize = rand::thread_rng().gen_range(0..3);
		let flags_i: usize = rand::thread_rng().gen_range(0..3);
		Cell::new(
			glyphs[glyph_i].clone(),
			fg_colors[fg_i],
			bg_colors[bg_i],
			flags[flags_i]
		)
	}

	pub fn hash_frame(&mut self) {
		let mut hasher = DefaultHasher::new();
		self.frame.hash(&mut hasher);
		let hash = hasher.finish();
		let entry = self.state_cache.entry(hash).or_insert(0);
		*entry += 1;

		if *entry >= self.stale_ticks {
			// if we've seen this state 15 times, it's probably not going to change anymore
			self.is_stale = true;
		}
	}
}

impl Default for Conway {
	fn default() -> Self {
		Self::new()
	}
}

impl WhoaAnimation for Conway {
	fn configure(&mut self, config: &toml::Value) {
		let Some(config) = config.get("conway") else { return };
		let stale_ticks = config.get("stale_ticks")
			.unwrap_or(&toml::Value::Integer(15)).as_integer().unwrap_or(15) as usize;
		let tick_rate = config.get("tick_rate")
			.unwrap_or(&toml::Value::Integer(10)).as_integer().unwrap_or(10) as u64;

		self.stale_ticks = stale_ticks;
		self.tick_rate = tick_rate as usize;
	}
}

impl Animation for Conway {
	fn initial_frame(&self) -> Frame { seeded_frame() }
	fn init(&mut self, initial: Frame) {
		self.frame = initial;
	}

	fn update(&mut self) -> Frame {
		if self.last_tick.elapsed().as_millis() <= (1000 / self.tick_rate as u128) {
			return self.frame.clone();
		}
		let (rows,cols) = self.frame.dims().unwrap_or((0,0));
		let cells = self.frame.take().into_cells();
		let mut new = cells.clone();

		let mut neigh = vec![Cell::default();9];
		let mut is_alive: bool;
		let mut num_alive: usize;
		let mut dx: i32;
		let mut dy: i32;
		let mut cell_r: usize;
		let mut cell_c: usize;

		for (y,row) in cells.iter().enumerate() {
			for (x,cell) in row.iter().enumerate() {
				num_alive = 0;
				is_alive = !cell.is_empty();
				neigh = vec![Cell::default();9];

				for n in 0..9 {
					neigh[n] = Cell::default();
					dx = (n as i32 % 3) - 1; // horizontal offset
					dy = (n as i32 / 3) - 1; // vertical offset

					cell_r = (y as i32 + dy) as usize;
					cell_c = (x as i32 + dx) as usize;
					if !(0..rows).contains(&cell_r) || !(0..cols).contains(&cell_c)
					|| (dx == 0 && dy == 0) {
						continue;
					}

					let cell = cells[cell_r][cell_c].clone();
					if !cell.is_empty() {
						num_alive += 1;
						neigh[n] = cell;
					}
				}

				if is_alive && (num_alive == 2 || num_alive == 3) {
					// cell survives
					continue;
				} else if !is_alive && num_alive == 3 {
					// cells reproduce
					let parents = neigh.iter()
						.filter(|c| !c.is_empty()).
						cloned().
						collect::<Vec<_>>();
					let child = Self::reproduce(&parents);
					new[y][x] = child;
				} else if is_alive {
					// cell dies
					new[y][x] = Cell::default();
				}
			}
		}

		self.last_tick = Instant::now();
		self.frame = Frame::from_cells(new);
		self.hash_frame();
		self.frame.clone()
	}

	fn is_done(&self) -> bool {
		self.is_stale
	}

	fn resize(&mut self, w: usize, h: usize) {
		self.frame.resize(w, h);
	}
}
