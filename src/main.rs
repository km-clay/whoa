use std::{cell::RefCell, collections::HashMap, env, io::IsTerminal, path::PathBuf};

use clap::{Parser, Subcommand};
use crossterm::{cursor, execute, terminal};
use indoc::indoc;
use rand::seq::SliceRandom;
use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet, util::as_24_bit_terminal_escaped};
use toml::Value;

use crate::anim::{Animation, Animator, cos::Cosine, Gradient};

pub mod anim;

#[derive(Parser)]
struct WhoaArgs {
	#[arg(long, help = "Run in screensaver mode. Any input will cause the program to exit immediately")]
	screensaver: bool,

	#[arg(short, long, help = "Path to config file. If not provided, will look for config in the default config directory")]
	config: Option<PathBuf>,

	#[command(subcommand)]
	animation: Option<AnimCmd>,
}

#[derive(Subcommand)]
enum AnimCmd {
	Saturn {
		#[arg(long, help = "The background number to use. Takes a number between 1 and 327")]
		bg_index: Option<usize>,

		#[arg(long, help = "Filters out the spooky ones")]
		no_giygas: bool,

		#[arg(long, help = "If set, after this many seconds the background index will be re-rolled")]
		lifetime: Option<f32>
	},
	Perlin {
		#[arg(long, help = "The color gradient to use.")]
		gradient: Option<String>,

		#[arg(long, help = "The speed that the noise moves at.")]
		speed: Option<f32>
	},
	Cosine {
		#[arg(long, help = "How fast the animation progresses.")]
		speed: Option<f32>
	},
	Slime {
		#[arg(long, help = "The color gradient to use.")]
		gradient: Option<String>
	},
	Maelstrom {
		#[arg(long, help = "Number of seconds to wait before swirling")]
		wait_time: Option<f32>,

		#[arg(long, help = "Starting speed of the swirl")]
		speed_min: Option<f32>,

		#[arg(long, help = "Maximum speed of the swirl, the animation will accelerate between speed_min and speed_max")]
		speed_max: Option<f32>
	},
	Conway {
		#[arg(long, help = "Number of times a state can be repeated before game restarts with a different text source")]
		stale_ticks: Option<usize>,

		#[arg(long, help = "Number of game ticks to run per second")]
		tick_rate: Option<f32>
	},
	Collapse {
		#[arg(long, help = "The direction the text should fall towards. Options are: up, down, left, right, and random")]
		direction: Option<String>
	}
}

const DEFAULT_CONTENT: [&str;10] = [
	include_str!("./anim/collapse.rs"),
	include_str!("./anim/conway.rs"),
	include_str!("./anim/cos.rs"),
	include_str!("./anim/maelstrom.rs"),
	include_str!("./anim/mod.rs"),
	include_str!("./anim/perlin.rs"),
	include_str!("./anim/saturn/mod.rs"),
	include_str!("./anim/saturn/romparse.rs"),
	include_str!("./anim/slime.rs"),
	include_str!("./main.rs"),
];

/// A hat that you can pull items from, in a random sequence, without repeats.
/// After all of the items have been pulled, the hat is refilled and shuffled, so you can keep pulling indefinitely.
pub struct Hat<T: Clone> {
	items: Vec<T>,
	hat: Vec<usize>
}

impl<T: Clone> Hat<T> {
	pub fn new(items: Vec<T>) -> Self {
		let hat = Self::get_hat(items.len());
		Self { items, hat }
	}

	fn get_hat(len: usize) -> Vec<usize> {
		let mut hat: Vec<usize> = (0..len).collect();
		let mut rng = rand::rng();
		hat.shuffle(&mut rng);
		hat
	}
}

impl<T: Clone> Iterator for Hat<T> {
	type Item = T;
	fn next(&mut self) -> Option<Self::Item> {
		match self.hat.pop() {
			Some(idx) => Some(self.items[idx].clone()),
			None => {
				if self.items.is_empty() {
					return None;
				}
				// hat is empty
				self.hat = Self::get_hat(self.items.len());
				self.next()
			}
		}
	}
}


thread_local! {
	pub static GRADIENTS: RefCell<HashMap<String, Gradient>> = RefCell::new({
		let mut map = HashMap::new();
		map.insert("aurora".into(), Gradient::aurora());
		map.insert("ocean".into(), Gradient::ocean());
		map.insert("fire".into(), Gradient::fire());
		map.insert("vapor".into(), Gradient::vapor());
		map.insert("mono".into(), Gradient::mono());
		map
	});
	pub static SEED_CONTENT: RefCell<Hat<String>> = RefCell::new(Hat::new(vec![]));
}

pub fn pull_seed_content() -> String {
	if SEED_CONTENT.with_borrow(|c| c.items.is_empty()) {
		collect_seed_content().ok();
	}
	SEED_CONTENT.with_borrow_mut(|c| {
		c.next().unwrap()
	})
}

pub fn collect_seed_content() -> anyhow::Result<()> {
	let mut seed_content = vec![];

	if !std::io::stdin().is_terminal() {
		let mut buffer = String::new();
		while let Ok(bytes_read) = std::io::stdin().read_line(&mut buffer) {
			if bytes_read == 0 {
				break;
			}
		}
		SEED_CONTENT.with_borrow_mut(|c| {
			*c = Hat::new(vec![buffer])
		});
		return Ok(())
	}
	if let Ok(cmd) = env::var("WHOA_TEXT_CMD") {
		let text_cmd = std::process::Command::new("sh")
			.args(["-c", &cmd])
			.output();
		if let Ok(out) = text_cmd {
			if out.status.success() {
				let text = String::from_utf8_lossy(&out.stdout).to_string();
				seed_content.push(text);
			} else {
				anyhow::bail!("Text command '{cmd}' failed: {}", String::from_utf8_lossy(&out.stderr));
			}
		}
	}

	if let Ok(path) = env::var("WHOA_PATH") {
		let paths = path.split(':');
		let reader = env::var("WHOA_FILE_READER");
		for dir in paths {
			let Ok(entries) = std::fs::read_dir(dir) else { continue };
			for file in entries.flatten() {
				let path = file.path();
				if !path.is_file() { continue }
				let path = path.to_string_lossy().to_string();
				if let Ok(ref cmd) = reader {
					let read_cmd = std::process::Command::new("sh")
						.args(["-c", &format!("{cmd} \"$1\""), "--", &path])
						.output();
					if let Ok(out) = read_cmd {
						if out.status.success() {
							let text = String::from_utf8_lossy(&out.stdout).to_string();
							seed_content.push(text);
						} else {
							anyhow::bail!("Reader command '{cmd}' failed for file {}: {}", path, String::from_utf8_lossy(&out.stderr));
						}
					}
				} else {
					let Ok(content) = std::fs::read_to_string(&path) else {
						continue;
					};
					seed_content.push(content);
				}
			}
		}
	}

	if seed_content.is_empty() {
		// fall back to our default content
		// which is basically just going to be
		// the source code files but highlighted by syntect
		let ss = SyntaxSet::load_defaults_newlines();
		let ts = ThemeSet::load_defaults();
		let theme = &ts.themes["base16-eighties.dark"];

		let syntax = ss.find_syntax_by_extension("rs").unwrap();
		let mut h = HighlightLines::new(syntax, theme);
		let mut output = String::new();

		for content in DEFAULT_CONTENT {
			log::info!("Adding default content with {} lines", content.lines().count());
			for line in content.replace('\t', "    ").lines() {
				let ranges = h.highlight_line(line, &ss).unwrap();
				let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
				output.push_str(&escaped);
				output.push('\n');
			}

			seed_content.push(std::mem::take(&mut output));
		}

	}

	SEED_CONTENT.with_borrow_mut(|c| {
		*c = Hat::new(seed_content)
	});
	Ok(())
}

pub fn get_gradient(name: &str) -> Option<Gradient> {
	GRADIENTS.with_borrow_mut(|map| {
		map.get(name).cloned()
	})
}

pub fn register_gradient(name: &str, gradient: Gradient) {
	GRADIENTS.with_borrow_mut(|map| {
		map.insert(name.into(), gradient)
	});
}


fn gen_config_file() -> anyhow::Result<()> {
	let content = indoc! {r#"
		# Animation config

		# Animations to be chosen at random
		enabled_animations = [
			"saturn",
			"perlin",
			"slime",
			"maelstrom",
			"conway",
			"collapse",
			"cosine"
		]

		# Time in seconds before the animation is forced to end and a new one is chosen.
		# 0.0 means no time limit.
		animation_time = 30.0

		# Screensaver mode will make the program exit upon receiving any user input at all
		screensaver_mode = false

		# Some animations can use text files as seeds for the animation.
		# Maelstrom for instance will use a file's content for it's swirly text effect.
		# Any dirs listed here will be searched for text files to use.
		# If none are given, whoa includes some defaults built in.
		text_source_dirs = [
			# /path/to/text/files
		]

		# This option lets you specify a command to use to get the text from the source files.
		# Internally, it runs in a subshell like '<command> <file>', the full command string
		# will be subject to the usual shell word splitting.
		file_reader_cmd="cat"

		# This option lets you provide a command to use as a text provider
		# text_cmd = "man bash"

		# If both text_source_dirs and text_cmd are set, the outputs are all placed in the same pool
		# and chosen at random

		# Some animations use gradients for colorization
		# There are some builtin gradients: ocean, fire, vapor, mono, and aurora.
		# You can provide a background color, and any number of color stops on the spectrum.
		[gradients.forest]
		bg = [0, 10, 0] # very dark green
		stops = [
			[0, 20, 0], # dark green
			[0, 100, 30], # medium green
			[50, 200, 50], # light green
			[150, 255, 150] # pale green
		]

		[saturn] # earthbound battle backgrounds
		no_giygas = true # filters out the spooky ones
		lifetime = 20.0  # seconds before rolling a new background

		[perlin] # perlin noise
		gradient = "ocean"
		speed = 0.3

		[cosine] # cosine wave
		speed = 1.0

		[slime] # slime mold simulation
		gradient = "forest"

		[maelstrom] # makes some text swirly
		wait_time = 1 # time before animation actually starts
		speed_min = 0.05
		speed_max = 0.20

		[conway] # game of life
		stale_ticks = 15 # Number of repeated states before animation ends
		tick_rate = 10 # Number of ticks per second

		[collapse] # makes some text fall
		# Available collapse directions: up, down, left, right, random
		direction = "down"
	"#};

	let Some(path) = config_file_path() else {
		eprintln!("Could not find config directory");
		return Ok(());
	};

	if !path.exists() {
		std::fs::create_dir_all(path.parent().unwrap())?;
		std::fs::write(&path, content)?;
		log::info!("Generated default config at {}", path.display());
	} else {
		log::info!("Config file already exists at {}, skipping generation", path.display());
	}

	Ok(())
}

fn config_file_path() -> Option<PathBuf> {
	dirs::config_dir().map(|dir| dir.join("whoa").join("config.toml"))
}

fn init_panic_handler() {
	let default_hook = std::panic::take_hook();
	std::panic::set_hook(Box::new(move |info| {
		let mut stdout = std::io::stdout();
		execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show).unwrap();
		terminal::disable_raw_mode().ok();
		default_hook(info)
	}));
}

fn get_config() -> anyhow::Result<toml::Value> {
	let args = WhoaArgs::parse();

	let config_content = std::fs::read_to_string(args.config.unwrap_or(config_file_path().unwrap()))?;
	let mut config: toml::Value = match toml::from_str(&config_content) {
		Ok(cfg) => cfg,
		Err(e) => {
			anyhow::bail!("Failed to read config file: {e}");
		}
	};

	if let Some(cmd) = args.animation {
		// The user has requested a specific animation
		// So we ignore the enabled_animations list and just put the requested one in there
		if let Some(Value::Array(anims)) = config.get_mut("enabled_animations") {
			anims.clear()
		}
		if let Some(Value::Float(lifetime)) = config.get_mut("animation_time") {
			*lifetime = 0.0;
		}
		match cmd {
			AnimCmd::Saturn { bg_index, no_giygas, lifetime } => {
				if let Some(Value::Array(anims)) = config.get_mut("enabled_animations") {
					anims.push(Value::String("saturn".to_string()));
				}
				if let Some(Value::Table(saturn_cfg)) = config.get_mut("saturn") {
					saturn_cfg.insert("no_giygas".to_string(), Value::Boolean(no_giygas));
					if let Some(idx) = bg_index {
						saturn_cfg.insert("bg_index".to_string(), Value::Integer(idx as i64));
					}
					if let Some(lifetime) = lifetime {
						saturn_cfg.insert("lifetime".to_string(), Value::Float(lifetime as f64));
					}
				}
			}
			AnimCmd::Perlin { gradient, speed } => {
				if let Some(Value::Array(anims)) = config.get_mut("enabled_animations") {
					anims.push(Value::String("perlin".to_string()));
				}
				if let Some(Value::Table(perlin_cfg)) = config.get_mut("perlin") {
					if let Some(gradient) = gradient {
						perlin_cfg.insert("gradient".to_string(), Value::String(gradient));
					}
					if let Some(speed) = speed {
						perlin_cfg.insert("speed".to_string(), Value::Float(speed as f64));
					}
				}
			}
			AnimCmd::Cosine { speed } => {
				if let Some(Value::Array(anims)) = config.get_mut("enabled_animations") {
					anims.push(Value::String("cosine".to_string()));
				}
				if let Some(Value::Table(cosine_cfg)) = config.get_mut("cosine")
				&& let Some(speed) = speed {
					cosine_cfg.insert("speed".to_string(), Value::Float(speed as f64));
				}
			}
			AnimCmd::Slime { gradient } => {
				if let Some(Value::Array(anims)) = config.get_mut("enabled_animations") {
					anims.push(Value::String("slime".to_string()));
				}
				if let Some(Value::Table(slime_cfg)) = config.get_mut("slime")
				&& let Some(gradient) = gradient {
					slime_cfg.insert("gradient".to_string(), Value::String(gradient));
				}
			}
			AnimCmd::Maelstrom { wait_time, speed_min, speed_max } => {
				if let Some(Value::Array(anims)) = config.get_mut("enabled_animations") {
					anims.push(Value::String("maelstrom".to_string()));
				}
				if let Some(Value::Table(maelstrom_cfg)) = config.get_mut("maelstrom") {
					if let Some(wait_time) = wait_time {
						maelstrom_cfg.insert("wait_time".to_string(), Value::Float(wait_time as f64));
					}
					if let Some(speed_min) = speed_min {
						maelstrom_cfg.insert("speed_min".to_string(), Value::Float(speed_min as f64));
					}
					if let Some(speed_max) = speed_max {
						maelstrom_cfg.insert("speed_max".to_string(), Value::Float(speed_max as f64));
					}
				}
			}
			AnimCmd::Conway { stale_ticks, tick_rate } => {
				if let Some(Value::Array(anims)) = config.get_mut("enabled_animations") {
					anims.push(Value::String("conway".to_string()));
				}
				if let Some(Value::Table(conway_cfg)) = config.get_mut("conway") {
					if let Some(stale_ticks) = stale_ticks {
						conway_cfg.insert("stale_ticks".to_string(), Value::Integer(stale_ticks as i64));
					}
					if let Some(tick_rate) = tick_rate {
						conway_cfg.insert("tick_rate".to_string(), Value::Float(tick_rate as f64));
					}
				}
			}
			AnimCmd::Collapse { direction } => {
				if let Some(Value::Array(anims)) = config.get_mut("enabled_animations") {
					anims.push(Value::String("collapse".to_string()));
				}
				if let Some(Value::Table(collapse_cfg)) = config.get_mut("collapse")
				&& let Some(direction) = direction {
					collapse_cfg.insert("direction".to_string(), Value::String(direction));
				}
			}
		}
	}

	Ok(config)
}

fn main() -> anyhow::Result<()> {
	init_panic_handler();
	env_logger::init();
	ctrlc::set_handler(|| {
		let mut stdout = std::io::stdout();
		execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show).unwrap();
		terminal::disable_raw_mode().ok();
		std::process::exit(0);
	}).unwrap();
	gen_config_file().unwrap();

	let config = get_config()?;

	if let Some(gradients) = config.get("gradients").and_then(|v| v.as_table()) {
		for (name, gradient_cfg) in gradients {
			let Ok(gradient) = Gradient::from_value(gradient_cfg) else {
				anyhow::bail!("Invalid gradient config for '{}'", name);
			};
			register_gradient(name, gradient);
		}
	}

	// we store these factory closures instead of the actual boxes
	// because once they are behind the Animation trait object
	// we cant clone them anymore. passing closures around sidesteps ownership issues
	let animations = config.get("enabled_animations")
		.and_then(|v| v.as_array())
		.map(|arr| {
			arr.iter().filter_map(|v| Some(match v.as_str()? {
				"saturn" => || { Box::new(anim::saturn::Saturn::new()) as Box<dyn Animation> },
				"perlin" => || { Box::new(anim::perlin::PerlinNoise::new()) as Box<dyn Animation> },
				"slime" => || { Box::new(anim::slime::SlimeMold::new()) as Box<dyn Animation> },
				"maelstrom" => || { Box::new(anim::maelstrom::Maelstrom::new()) as Box<dyn Animation> },
				"conway" => || { Box::new(anim::conway::Conway::new()) as Box<dyn Animation> },
				"collapse" => || { Box::new(anim::collapse::Collapse::new()) as Box<dyn Animation> },
				"cosine" => || { Box::new(Cosine::new()) as Box<dyn Animation> },
				_ => {
					eprintln!("Invalid animation name: {}, defaulting to saturn",v.as_str()?);
					|| { Box::new(anim::saturn::Saturn::new()) as Box<dyn Animation> }
				}
			})).collect::<Vec<_>>()
		}).unwrap_or_default();

	let hat = Hat::new(animations);

	for anim_fn in hat {
		// call the closure to produce the boxed Animation
		let animation = anim_fn();

		let mut animator = Animator::new(animation, &config);
		if !animator.play() {
			break
		};
	}
	Ok(())
}
