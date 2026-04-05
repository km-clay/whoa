use std::{collections::HashMap, path::PathBuf, sync::LazyLock};

use crossterm::{cursor, execute, terminal};
use indoc::indoc;

use crate::anim::{Animation, Animator, cos::Cosine, perlin::Gradient};

pub mod anim;

pub static GRADIENTS: LazyLock<HashMap<String, Gradient>> = LazyLock::new(|| {
	let mut map = HashMap::new();
	map.insert("aurora".into(), Gradient::aurora());
	map.insert("ocean".into(), Gradient::ocean());
	map.insert("fire".into(), Gradient::fire());
	map.insert("vapor".into(), Gradient::vapor());
	map.insert("mono".into(), Gradient::mono());
	map
});

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

		# Some animations can use text files as seeds for the animation.
		# Maelstrom for instance will use a file's content for it's swirly text effect.
		# Any dirs listed here will be searched for text files to use.
		# If none are given, whoa includes some defaults built in.
		text_source_dirs = [
			# /path/to/text/files
		]


		# Some animations use gradients for colorization
		# There are some builtin gradients: ocean, fire, vapor, mono, and aurora.
		# You can also define custom gradients.
		# You can provide a background color, and any number of color stops on the spectrum.
		# Available default gradients: ocean, fire, vapor, mono, aurora
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
		default_hook(info)
	}));
}

fn main() {
	init_panic_handler();
	env_logger::init();
	ctrlc::set_handler(|| {
		let mut stdout = std::io::stdout();
		execute!(stdout, terminal::LeaveAlternateScreen, cursor::Show).unwrap();
		std::process::exit(0);
	}).unwrap();
	gen_config_file().unwrap();

	let config_content = std::fs::read_to_string(config_file_path().unwrap()).unwrap();
	let config: toml::Value = match toml::from_str(&config_content) {
		Ok(cfg) => cfg,
		Err(e) => {
			eprintln!("Failed to read config file: {e}");
			return
		}
	};

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

	let anim_idx = rand::random_range::<usize, std::ops::Range<usize>>(0..animations.len());
	let animation = animations[anim_idx]();

	let mut animator = Animator::new(animation, config);
	animator.play();
	println!("Animation complete!");
}
