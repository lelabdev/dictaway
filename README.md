# dictaway

Voice dictation for Wayland. Press a key, speak, press again — text appears in your focused app.

Captures audio via `ffmpeg`, transcribes with [whisper.cpp](https://github.com/ggerganov/whisper.cpp) (CUDA GPU accelerated, no temp files), types with `wtype`. Shows a real-time waveform overlay. Pauses media while dictating.

## Features

- Toggle-based: one shortcut to start/stop
- Real-time waveform overlay (GTK4 Layer Shell, amber→teal gradient)
- Auto-detects language (French, English, German, etc.)
- Auto-downloads whisper model on first run
- CUDA GPU acceleration via `whisper-rs`
- Automatic media pause/resume via `playerctl`
- In-memory audio pipeline (no temp WAV files)
- Whisper artifacts filtered (music tags, ellipsis)
- Configurable model, audio device, and language via flags

## Requirements

- Rust toolchain + Cargo
- `ffmpeg` (audio capture)
- `wtype` (Wayland keyboard simulator)
- `playerctl` (media control)
- NVIDIA GPU + CUDA (optional, for GPU acceleration)

## Install

Download the latest binary (Linux x86_64, CUDA enabled):

```bash
curl -L https://github.com/lelabdev/dictaway/releases/latest/download/dictate -o ~/.local/bin/dictate && chmod +x ~/.local/bin/dictate
```

Or build from source:

```bash
cargo install --git https://github.com/lelabdev/dictaway --features cuda
```

<details>
<summary>Build from source (manual)</summary>

```bash
git clone https://github.com/lelabdev/dictaway.git
cd dictaway
cargo build --release --features cuda
cp target/release/dictate ~/.local/bin/
```

</details>

## Usage

```bash
dictate                                        # toggle on/off
dictate --lang en                              # force English this time
dictate --model ~/path/to/ggml-medium.bin      # use a specific model
dictate --device alsa_input.pci-001            # use a specific audio device
dictate --stop                                 # force stop
```

- **First call**: starts listening, pauses media, transcribes and types text in 3-second blocks
- **Second call** (or Ctrl+C): stops, flushes remaining text, resumes media

## Configuration

Create `~/.config/dictaway/config`:

```
# Default language (fr, en, de, auto)
lang=fr
```

Available languages: `fr`, `en`, `de`, `es`, `it`, `pt`, `nl`, `hi`, `auto` (auto-detect).

Override per-run with `dictate --lang en` or `dictate --lang auto`.

## First Run

On first launch, if no model is found, you'll see an interactive picker:

```
🎤 No whisper model found. Let's pick one!

  #  Model       Size     GPU VRAM   Speed    Quality
  ──────────────────────────────────────────────────────
  1  tiny        75 MB    < 1 GB    ⚡⚡⚡   Basic
  2  base        142 MB   ~1 GB     ⚡⚡    Decent
  3  small       466 MB   ~2 GB     ⚡      Good ← recommended
  4  medium      1.5 GB   ~5 GB     Slow    Very good
  5  large-v3    2.9 GB   ~10 GB    V slow  Excellent

  💡 No GPU? All models work on CPU too (just slower).

  Pick a model [1-5] (default: 3):
```

The model is downloaded automatically and reused on future runs.

## Keybinding

Example for [MangoWM](https://mangowm.github.io/):

```
bind=SUPER,d,spawn,dictate
```

## Architecture

```
ffmpeg (PulseAudio, 16kHz mono)
  → ring buffer (f32 samples)
    ├→ volume meter → overlay (GTK4 Layer Shell, real-time)
    └→ whisper-rs + CUDA (transcription, 3s blocks)
         → text filter (remove artifacts)
           → wtype (type text)

playerctl --all-players pause/play (media control)
```

### Overlay

A floating waveform appears at the bottom of the screen during dictation:
- 9 animated bars with amber→teal color gradient based on voice level
- Real-time response (62ms audio window, 40fps rendering)
- Pulsing REC indicator dot
- Auto-dismisses when dictation stops

## License

MIT
