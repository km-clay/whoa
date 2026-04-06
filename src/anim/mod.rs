use std::{fmt::{self, Display}, hash::{DefaultHasher, Hash, Hasher}, io::{Stdout, Write}, ops::{Div, Mul}, time::{Duration, Instant}};
use bitflags::bitflags;

use crossterm::{cursor, execute, queue, style::{Color, Stylize}, terminal};
use glam::{Vec2, Vec3};
use smallvec::SmallVec;
use toml::Value;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthChar;

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
	#[allow(clippy::get_first)]
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

pub trait Animation {
	fn init(&mut self, initial: Frame);
	fn initial_frame(&self) -> Frame { Frame::from_terminal() }
	fn update(&mut self, dt: Duration) -> Frame;
	fn is_done(&self) -> bool;
	fn resize(&mut self, w: usize, h: usize);
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

#[derive(Default,Clone,Debug,Hash)]
pub struct Frame(Vec<Vec<Cell>>);

type Rows = usize;
type Cols = usize;
impl Frame {
	pub fn take(&mut self) -> Self {
		let (rows,cols) = self.dims().unwrap_or((0,0));
		let new_cells = Self::with_capacity(cols, rows).0;
		Frame(std::mem::replace(&mut self.0, new_cells))
	}
	pub fn with_capacity(cols: usize, rows: usize) -> Self {
		Frame(vec![vec![Cell::default(); cols]; rows])
	}
	pub fn get_hash(&self) -> u64 {
		let mut hasher = DefaultHasher::new();
		self.hash(&mut hasher);
		hasher.finish()
	}
	pub fn from_terminal() -> Self {
		let (cols,rows) = crossterm::terminal::size().unwrap_or((80, 24));
		let mut builder = FrameBuilder::new(cols as usize, rows as usize);
		builder.feed_bytes(b"\x1b[?25l"); // hide cursor
		builder.feed_bytes(b"\x1b[2J"); // clear screen
		builder.feed_bytes(b"\x1b[H"); // move cursor to top-left
		let mut frame = builder.build();
		frame.resize(cols as usize, rows as usize);
		frame
	}
	pub fn from_command(mut command: std::process::Command) -> Self {
		let (cols,rows) = crossterm::terminal::size().unwrap_or((80, 24));
		let output = command
			.env("COLUMNS", cols.to_string())
			.output()
			.expect("Failed to execute command");

		let mut builder = FrameBuilder::new(cols as usize, rows as usize);
		builder.feed_bytes(&output.stdout);
		let mut frame = builder.build();
		frame.resize(cols as usize, rows as usize);
		frame
	}
	pub fn dims(&self) -> Option<(Rows,Cols)> {
		let rows = self.0.len();
		if rows == 0 {
			return None;
		}
		let cols = self.0[0].len();
		Some((rows, cols))
	}

	pub fn resize(&mut self, w: usize, h: usize) {
		// adjust columns on existing rows
		for row in &mut self.0 {
			row.resize(w, Cell::default());
		}
		// adjust row count
		self.0.resize(h, vec![Cell::default(); w]);
	}
}

pub struct FrameBuilder {
	cells: Vec<Vec<Cell>>,
	row: usize,
	rows: usize,
	col: usize,
	cols: usize,
	current_fg: Color,
	current_bg: Color,
	current_flags: CellFlags,
	parser: vte::Parser
}

impl FrameBuilder {
	pub fn new(cols: usize, rows: usize) -> Self {
		Self {
			cells: vec![vec![Cell::default(); cols]; rows],
			row: 0,
			rows,
			col: 0,
			cols,
			current_fg: Color::Reset,
			current_bg: Color::Reset,
			current_flags: CellFlags::empty(),
			parser: vte::Parser::new()
		}
	}
	pub fn feed_bytes(&mut self, bytes: &[u8]) {
		let mut parser = std::mem::take(&mut self.parser);
		parser.advance(self, bytes);
		self.parser = parser;
	}
	pub fn feed_str(&mut self, s: &str) {
		self.feed_bytes(s.as_bytes());
	}
	pub fn build(self) -> Frame {
		Frame(self.cells)
	}
}

impl vte::Perform for FrameBuilder {
	fn print(&mut self, c: char) {
		if self.col >= self.cols {
			self.col = 0;
			self.row += 1;
		}
		if self.row >= self.rows {
			self.cells.push(vec![Cell::default(); self.cols]);
			self.rows += 1;
		}
	  let cell = Cell {
			ch: c.into(),
			fg: self.current_fg,
			bg: self.current_bg,
			flags: self.current_flags
		};
		self.cells[self.row][self.col] = cell;
		self.col += 1;
	}
	fn execute(&mut self, byte: u8) {
		match byte {
			b'\n' => {
				self.row += 1;
				self.col = 0;
				if self.row >= self.rows {
					self.cells.push(vec![Cell::default(); self.cols]);
					self.rows += 1;
				}
			}
			b'\r' => {
				self.col = 0;
			}
			_ => {}
		}
	}
	fn csi_dispatch(
		&mut self,
		params: &vte::Params,
		_intermediates: &[u8],
		_ignore: bool,
		action: char,
	) {
		if action != 'm' { return; }
		let params: Vec<u16> = params.iter()
			.flat_map(|p| p.iter().copied())
			.collect();

		let mut i = 0;
		while i < params.len() {
			let Some(param) = params.get(i) else { continue; };
			match param {
				0 => {
					self.current_fg = Color::Reset;
					self.current_bg = Color::Reset;
					self.current_flags = CellFlags::empty();
				}
				1 => self.current_flags.insert(CellFlags::BOLD),
				2 => self.current_flags.insert(CellFlags::DIM),
				3 => self.current_flags.insert(CellFlags::ITALIC),
				4 => self.current_flags.insert(CellFlags::UNDERLINE),
				7 => self.current_flags.insert(CellFlags::INVERSE),
				8 => self.current_flags.insert(CellFlags::HIDDEN),
				9 => self.current_flags.insert(CellFlags::STRIKETHROUGH),
				30..=37 => self.current_fg = Color::AnsiValue((params[i] - 30) as u8),
				38 | 48 => {
					let is_bg = *param == 48;
					i += 1;
					let Some(param2) = params.get(i) else { continue; };
					match param2 {
						5 => {
							i += 1;
							let Some(param3) = params.get(i) else { continue; };
							let color = Color::AnsiValue(*param3 as u8);
							if is_bg {
								self.current_bg = color;
							} else {
								self.current_fg = color;
							}
						}
						2 => {
							i += 1;
							let Some(param3) = params.get(i) else { continue; };
							i += 1;
							let Some(param4) = params.get(i) else { continue; };
							i += 1;
							let Some(param5) = params.get(i) else { continue; };

							let color = Color::Rgb { r: *param3 as u8, g: *param4 as u8, b: *param5 as u8 };
							if is_bg {
								self.current_bg = color;
							} else {
								self.current_fg = color;
							}
						}
						_ => {}
					}
				}
				39 => self.current_fg = Color::Reset,
				40..=47 => self.current_bg = Color::AnsiValue((params[i] - 40) as u8),
				49 => self.current_bg = Color::Reset,
				90..=97 => self.current_fg = Color::AnsiValue((params[i] - 90 + 8) as u8),
				100..=107 => self.current_bg = Color::AnsiValue((params[i] - 100 + 8) as u8),
				_ => { /* ignore unknown params */ }
			}
			i += 1;
		}
	}
}

pub struct RawModeGuard;

impl RawModeGuard {
	pub fn enter() -> std::io::Result<Self> {
		let mut stdout = std::io::stdout();
		execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
		terminal::enable_raw_mode()?;

		Ok(Self)
	}
}

impl Drop for RawModeGuard {
	fn drop(&mut self) {
		let mut stdout = std::io::stdout();
		execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show).ok();
		terminal::disable_raw_mode().ok();
	}
}

pub struct Animator {
	animation: Box<dyn Animation>,
	last_frame: Option<Frame>,
	raw_mode_state: Option<RawModeGuard>,
	frame_rate: usize,
	last_cols: u16,
	last_rows: u16,
	out_channel: Stdout,
	start: Instant
}

impl Animator {
	const TARGET_FRAME_RATE: u64 = 24;
	pub const WAIT_TIME: u64 = 3;

	pub fn new(animation: Box<dyn Animation>) -> Self {
		let (last_cols, last_rows) = crossterm::terminal::size().unwrap_or((80, 24));
		Self {
			animation,
			last_frame: None,
			raw_mode_state: None,
			frame_rate: 24,
			last_cols,
			last_rows,
			out_channel: std::io::stdout(),
			start: Instant::now()
		}
	}

	pub fn target_fps(mut self, fps: usize) -> Self {
		self.frame_rate = fps;
		self
	}

	pub fn enter_with(animation: Box<dyn Animation>) -> std::io::Result<Self> {
		let mut new = Self::new(animation);
		new.enter()?;
		Ok(new)
	}

	pub fn enter(&mut self) -> std::io::Result<()> {
		let guard = RawModeGuard::enter();
		self.raw_mode_state = Some(guard?);
		Ok(())
	}

	pub fn leave(&mut self) {
		self.raw_mode_state = None; // dropping the guard will restore the terminal state
	}

	pub fn tick(&mut self) -> anyhow::Result<bool> {
		let tick_start = Instant::now();

		let (cols, rows) = crossterm::terminal::size().unwrap_or((self.last_cols, self.last_rows));
		if cols != self.last_cols || rows != self.last_rows {
			self.animation.resize(cols as usize, rows as usize);
			self.last_frame = None; // force full redraw
			self.last_cols = cols;
			self.last_rows = rows;
		}

		let frame = self.animation.update(self.start.elapsed());
		self.render(frame)?;

		let tick_duration = tick_start.elapsed().as_millis();
		let target = 1000 / Self::TARGET_FRAME_RATE; // ms per frame
		let sleep_time = target.saturating_sub(tick_duration as u64);

		if sleep_time > 0 {
			std::thread::sleep(Duration::from_millis(sleep_time));
		}

		Ok(!self.animation().is_done())
	}

	pub fn animation(&self) -> &dyn Animation {
		&*self.animation
	}

	#[allow(clippy::needless_range_loop)]
	fn render(&mut self, frame: Frame) -> anyhow::Result<()> {
		let Frame(cells) = frame;
		let rows = cells.len();
		if rows == 0 {
			return Ok(());
		}
		let cols = cells[0].len();

		for row in 0..rows {
			for col in 0..cols {
				if let Some(Frame(last_frame)) = self.last_frame.as_ref() {
					if last_frame.get(row).and_then(|r| r.get(col)) != Some(&cells[row][col]) {
						// move to row, col, write the cell
						let cell = &cells[row][col];
						queue!(self.out_channel, cursor::MoveTo(col as u16, row as u16))?;
						write!(self.out_channel, "{cell}")?;
					}
				} else {
					let cell = &cells[row][col];
					queue!(self.out_channel, cursor::MoveTo(col as u16, row as u16))?;
					write!(self.out_channel, "{cell}")?;
				}
			}
		}
		self.out_channel.flush()?;

		self.last_frame = Some(Frame(cells));

		Ok(())
	}
}


#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A single grapheme. Graphemes can be composed of multiple chars, but are always treated as a single unit for display and editing purposes.
/// Using a SmallVec<[char; 4]> allows us to organize most multi-byte codepoints while maintaining both ownership and stack allocation.
/// If we ever run into a Grapheme made of more than 4 chars, just that Grapheme will gracefully spill over onto the heap
pub struct Grapheme(SmallVec<[char; 4]>);

impl Grapheme {
  pub fn chars(&self) -> &[char] {
    &self.0
  }
  /// Returns the display width of the Grapheme, treating unprintable chars as width 0
  pub fn width(&self) -> usize {
    self.0.iter().map(|c| c.width().unwrap_or(0)).sum()
  }
  /// Returns true if the Grapheme is wrapping a linefeed ('\n')
  pub fn is_lf(&self) -> bool {
    self.is_char('\n')
  }
  /// Returns true if the Grapheme consists of exactly one char and that char is equal to `c`
  pub fn is_char(&self, c: char) -> bool {
    self.0.len() == 1 && self.0[0] == c
  }
  /// If the Grapheme consists of exactly one char, returns that char. Otherwise, returns None.
  /// All callsites that use this method operate on ascii, so never returning anything for multibyte sequences is fine.
  pub fn as_char(&self) -> Option<char> {
    if self.0.len() == 1 {
      Some(self.0[0])
    } else {
      None
    }
  }

	pub fn is_whitespace(&self) -> bool {
		self.0.iter().all(|c| c.is_whitespace())
	}
}

impl From<char> for Grapheme {
  fn from(value: char) -> Self {
    let mut new = SmallVec::<[char; 4]>::new();
    new.push(value);
    Self(new)
  }
}

impl From<&str> for Grapheme {
  fn from(value: &str) -> Self {
    assert_eq!(value.graphemes(true).count(), 1);
    let mut new = SmallVec::<[char; 4]>::new();
    for char in value.chars() {
      new.push(char);
    }
    Self(new)
  }
}

impl From<String> for Grapheme {
  fn from(value: String) -> Self {
    Into::<Self>::into(value.as_str())
  }
}

impl From<&String> for Grapheme {
  fn from(value: &String) -> Self {
    Into::<Self>::into(value.as_str())
  }
}

impl Display for Grapheme {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for ch in &self.0 {
      write!(f, "{ch}")?;
    }
    Ok(())
  }
}

pub fn to_graphemes(s: impl ToString) -> Vec<Grapheme> {
  let s = s.to_string();
  s.graphemes(true).map(Grapheme::from).collect()
}
bitflags! {
	#[derive(Default,Clone,Copy,Debug,PartialEq,Eq,Hash)]
	pub struct CellFlags: u32 {
		const BOLD = 0b00000001;
		const ITALIC = 0b00000010;
		const UNDERLINE = 0b00000100;
		const INVERSE = 0b00001000;
		const HIDDEN = 0b00010000;
		const STRIKETHROUGH = 0b00100000;
		const DIM = 0b01000000;
		const BLINK = 0b10000000;
	}
}

#[derive(Clone,Debug,PartialEq,Eq,Hash)]
pub struct Cell {
	pub ch: Grapheme,
	pub fg: Color,
	pub bg: Color,
	pub flags: CellFlags
}

impl Default for Cell {
	fn default() -> Self {
		Self {
			ch: ' '.into(),
			fg: Color::Reset,
			bg: Color::Reset,
			flags: CellFlags::empty()
		}
	}
}

impl Cell {
	pub fn is_empty(&self) -> bool {
		self.ch.is_whitespace() && self.bg == Color::Reset
	}

	pub fn with_bg(mut self, bg: Color) -> Self {
		self.bg = bg;
		self
	}

	pub fn with_fg(mut self, fg: Color) -> Self {
		self.fg = fg;
		self
	}

	pub fn with_flags(mut self, flags: CellFlags) -> Self {
		self.flags = flags;
		self
	}

	pub fn with_char(mut self, ch: char) -> Self {
		self.ch = ch.into();
		self
	}

	pub fn set_bg(&mut self, bg: Color) {
		self.bg = bg;
	}

	pub fn set_fg(&mut self, fg: Color) {
		self.fg = fg;
	}

	pub fn set_flags(&mut self, flags: CellFlags) {
		self.flags = flags;
	}

	pub fn set_char(&mut self, ch: char) {
		self.ch = ch.into();
	}
}

impl Display for Cell {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let Cell { ch, fg, bg, flags } = self;

		let mut styled = crossterm::style::style(ch)
			.with(*fg)
			.on(*bg);

		if flags.contains(CellFlags::BOLD) {
			styled = styled.bold();
		}
		if flags.contains(CellFlags::ITALIC) {
			styled = styled.italic();
		}
		if flags.contains(CellFlags::UNDERLINE) {
			styled = styled.underlined();
		}
		if flags.contains(CellFlags::INVERSE) {
			styled = styled.reverse();
		}
		if flags.contains(CellFlags::HIDDEN) {
			styled = styled.hidden();
		}
		if flags.contains(CellFlags::STRIKETHROUGH) {
			styled = styled.crossed_out();
		}
		if flags.contains(CellFlags::DIM) {
			styled = styled.dim();
		}
		if flags.contains(CellFlags::BLINK) {
			styled = styled.slow_blink();
		}

		write!(f, "{styled}")
	}
}

impl From<char> for Cell {
	fn from(value: char) -> Self {
		Self {
			ch: value.into(),
			fg: Color::Reset,
			bg: Color::Reset,
			flags: CellFlags::empty()
		}
	}
}
