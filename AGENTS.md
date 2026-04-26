# Dictaway — Agent Guide

Rust + Whisper + CUDA voice-to-text tool. Repo : https://github.com/lelabdev/dictaway

## Stack

- **Rust** (edition 2021)
- **Whisper.cpp** bindings (CUDA)
- **CLI** first, pas de GUI
- Config dans `~/.config/dictaway/`

## Architecture

```
Audio Input → Whisper (CUDA) → Raw Text → Two-Tier Filters → Clean Output
```

### Two-Tier Filter System

Deux couches de filtres indépendantes en séquence :

```
Raw Whisper Output
    ↓
1. INTERNAL FILTERS (hardcoded)
   - Always active, stable, tested
   - Regex: brackets [...], asterisks *...*
   - Words: "Music", "Noise", "Applause"
    ↓
2. PERSONAL FILTERS (config file)
   - User-editable, optional
   - File: ~/.config/dictaway/filters
   - Each line = regex pattern
    ↓
3. FINAL CLEANUP
   - Remove double spaces, normalize whitespace
    ↓
Clean Output
```

#### Internal Filters (Rust)

```rust
const INTERNAL_IGNORE: &[&str] = &[
    "Music", "Noise", "Applause",
];

fn apply_internal_filters(text: &str) -> String {
    let re_brackets = regex::Regex::new(r"\[[^\]]*\]").unwrap();
    let re_asterisks = regex::Regex::new(r"\*[^*]+\*").unwrap();

    let cleaned = re_brackets.replace_all(text, "").to_string();
    let cleaned = re_asterisks.replace_all(&cleaned, "").to_string();

    cleaned.split_whitespace()
        .filter(|word| !INTERNAL_IGNORE.iter().any(|w| w.eq_ignore_ascii_case(word)))
        .collect::<Vec<_>>()
        .join(" ")
}
```

#### Personal Filters (config)

```rust
fn apply_personal_filters(text: &str) -> String {
    let config_path = format!("{}/.config/dictaway/filters", std::env::var("HOME").unwrap());

    if let Ok(content) = fs::read_to_string(&config_path) {
        let mut cleaned = text.to_string();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            if let Ok(re) = regex::Regex::new(line) {
                cleaned = re.replace_all(&cleaned, "").to_string();
            }
        }
        cleaned
    } else {
        text.to_string()
    }
}
```

#### Config File Format

```bash
# ~/.config/dictaway/filters
# Each line = regex pattern to remove

# Ignore specific word
\bmon_mot_inutile\b

# Ignore multiple alternatives
\b(mot_a|mot_b|mot_c)\b
```

## Dev Workflow

```bash
# Build
cargo build --release

# Test
cargo test

# Run
./target/release/dictaway
```

## Remotes

- `origin` → lelabdev/dictaway
- `fork` → A0-42/dictaway
