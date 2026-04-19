# dictate

Voice dictation tool for Wayland. Toggle-based: press to start, press again to stop. Text is transcribed in real-time and typed directly into the focused application.

Captures audio via `ffmpeg`, transcribes with [whisper.cpp](https://github.com/ggerganov/whisper.cpp) (in-memory, no temp files, CUDA GPU accelerated), and types text using `wtype`. Automatically pauses media during dictation.

## Features

- Toggle-based: one shortcut to start/stop
- In-memory audio pipeline (no temp WAV files)
- CUDA GPU acceleration via `whisper-rs`
- Automatic media pause/resume via `playerctl`
- Whisper artifacts filtered (music tags, ellipsis)
- Configurable whisper model via `--model` flag
- Graceful Ctrl+C handling

## Requirements

- Rust toolchain
- `ffmpeg` (audio capture)
- `wtype` (Wayland keyboard simulator)
- `playerctl` (media control)
- NVIDIA GPU + CUDA (optional, for GPU acceleration)
- Whisper model file (see below)

## Install

```bash
# Clone
git clone <repo-url> && cd dictate

# Build (with CUDA support)
cargo build --release

# Symlink
ln -sf $(pwd)/target/release/dictate ~/.local/bin/dictate
```

## Usage

```bash
dictate                                        # toggle on/off (default model)
dictate --model ~/path/to/ggml-medium.bin      # use a specific model
dictate --stop                                 # force stop
```

- **First call**: starts listening, pauses media, transcribes and types text in 3-second blocks
- **Second call** (or Ctrl+C): stops, flushes remaining text, resumes media

## Keybinding

Example for [MangoWM](https://mangowm.github.io/):

```
bind=SUPER,d,spawn,dictate
```

## Model

Default: `~/.local/share/whisper.cpp/models/ggml-small.bin`

```bash
mkdir -p ~/.local/share/whisper.cpp/models

# Small (default, 466 MB, good quality)
curl -L -o ~/.local/share/whisper.cpp/models/ggml-small.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin

# Base (142 MB, faster, decent quality)
curl -L -o ~/.local/share/whisper.cpp/models/ggml-base.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin
```

| Model | Size | Speed | Quality |
|-------|------|-------|---------|
| `tiny` | 75 MB | ⚡⚡⚡ | Basic |
| `base` | 142 MB | ⚡⚡ | Decent |
| `small` | 466 MB | ⚡ | Good ← default |
| `medium` | 1.5 GB | Slow | Very good |
| `large-v3` | 2.9 GB | Very slow | Excellent |

## Architecture

```
ffmpeg (PulseAudio, 16kHz mono)
  → ring buffer (f32 samples)
    → whisper-rs + CUDA (transcription, 3s blocks)
      → text filter (remove artifacts)
        → wtype (type text)

playerctl --all-players pause/play (media control)
```

## License

MIT
