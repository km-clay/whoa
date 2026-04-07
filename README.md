# whoa

A terminal screensaver with multiple visual effects, powered by [cellophane](https://github.com/km-clay/cellophane).

![whoa](https://github.com/user-attachments/assets/d1a4614b-718f-4baf-81d8-b64d5cb2b6ef)

## Animations

- **perlin** - Flowing Perlin noise visualized with Braille characters and color gradients
- **slime** - Physarum slime mold simulation with emergent patterns
- **maelstrom** - Text that swirls into a spiral vortex
- **conway** - Conway's Game of Life with terminal cell genetics
- **collapse** - Text collapses under gravity in a configurable direction
- **cosine** - Undulating wave effect driven by cosine functions
- **saturn** - EarthBound battle backgrounds rendered in the terminal with parallax scrolling and distortion

## Installation

```bash
cargo install whoa
```

## Usage

```bash
# Cycle through all enabled animations
whoa

# Run a specific animation
whoa saturn
whoa perlin --gradient ocean
whoa collapse --direction up

# Screensaver mode (exits on any input)
whoa --screensaver
```

### Animation Options

| Animation   | Flags |
|-------------|-------|
| `saturn`    | `--bg-index <N>`, `--no-giygas`, `--lifetime <seconds>` |
| `perlin`    | `--gradient <name>`, `--speed <float>` |
| `slime`     | `--gradient <name>` |
| `maelstrom` | `--wait-time <seconds>`, `--speed-min <float>`, `--speed-max <float>` |
| `conway`    | `--stale-ticks <N>`, `--tick-rate <float>` |
| `collapse`  | `--direction <up\|down\|left\|right\|random>` |
| `cosine`    | `--speed <float>` |

## Configuration

Whoa looks for a config file at `~/.config/whoa/config.toml`. A default one is created on first run.

```toml
enabled_animations = ["saturn", "perlin", "slime", "maelstrom", "conway", "collapse", "cosine"]
animation_time = 30.0   # seconds before switching (0.0 = no limit)
screensaver_mode = false

[perlin]
gradient = "aurora"
speed = 1.0

[collapse]
direction = "down"
```

### Gradients

Built-in gradients: `aurora`, `ocean`, `fire`, `vapor`, `mono`

Custom gradients can be defined in the config:

```toml
[gradients.custom_name]
colors = [[255, 0, 0], [0, 255, 0], [0, 0, 255]]
bg = [0, 0, 0]
```

## Text Sources

Animations that display text (maelstrom, conway, collapse, cosine) pull content from several sources:

1. **Piped stdin** - `cat file.rs | whoa maelstrom`
2. **`WHOA_TEXT_CMD`** - environment variable specifying a command to generate text (e.g. `man bash`)
3. **`WHOA_PATH`** - directory to scan for source files
4. **`WHOA_FILE_READER`** - custom command for reading files (e.g. `bat --force-colorization`)
5. **Fallback** - whoa's own source code, highlighted by [syntect](https://github.com/trishume/syntect)

## License

MIT
