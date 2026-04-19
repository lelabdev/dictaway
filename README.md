# dictate

Voice dictation tool for Wayland. Toggle-based: press to start, press again to stop. Text is transcribed in real-time and typed directly into the focused application.

Records audio via `cpal`, transcribes with [whisper.cpp](https://github.com/ggerganov/whisper.cpp) (in-memory, no temp files), and types text using `wtype`. Automatically pauses media during dictation.

## Features

- Toggle-based: one shortcut to start/stop
- In-memory audio pipeline (no temp WAV files)
- Automatic media pause/resume via `playerctl`
- Resamples any input sample rate to 16kHz mono for whisper
- Flushes remaining audio on stop

## Requirements

- Rust toolchain
- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) model file (`ggml-base.bin`)
- `wtype` (Wayland keyboard simulator)
- `playerctl` (media control)
- ALSA (audio input)

## Install

```bash
# Clone
git clone <repo-url> ~/1Dev/Projects/dictate/rust

# Build
cd ~/1Dev/Projects/dictate/rust
cargo build --release

# Symlink
ln -sf ~/1Dev/Projects/dictate/rust/target/release/dictate ~/.local/bin/dictate
```

## Usage

Just run `dictate` — it toggles:

- **First call**: starts listening, pauses media, transcribes and types text in 3-second blocks
- **Second call**: stops, flushes remaining text, resumes media

```bash
dictate       # toggle on/off
dictate --stop # force stop
```

## Keybinding

Example for [MangoWM](https://mangowm.github.io/):

```
bind=SUPER,d,spawn,dictate
```

## Model

Download a whisper.cpp model to `~/.local/share/whisper.cpp/models/`:

```bash
mkdir -p ~/.local/share/whisper.cpp/models
# Download ggml-base.bin from https://huggingface.co/ggerganov/whisper.cpp
```

Default path: `~/.local/share/whisper.cpp/models/ggml-base.bin`

## Architecture

```
cpal (audio capture)
  → ring buffer (16kHz mono f32)
    → whisper-rs (transcription, 3s blocks)
      → wtype (type text)

playerctl --all-players pause/play (media control)
```

## License

MIT
