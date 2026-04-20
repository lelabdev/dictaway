mod audio;
mod media;
mod overlay;
mod transcriber;
mod typer;

use clap::Parser;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const PID_FILE: &str = "/tmp/dictate.pid";
const STOP_FILE: &str = "/tmp/dictate-stop";
const BLOCK_SECS: usize = 3;
const TARGET_RATE: usize = 16000;

#[derive(Parser)]
#[command(name = "dictate", about = "Voice dictation for Wayland")]
struct Cli {
    /// Force stop
    #[arg(long)]
    stop: bool,

    /// Whisper model path
    #[arg(long)]
    model: Option<String>,

    /// PulseAudio source device
    #[arg(long, default_value = "default")]
    device: String,

    /// Language code (fr, en, de, auto) — default: from config or auto-detect
    #[arg(long)]
    lang: Option<String>,
}

fn config_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{}/.config/dictaway/config", home)
}

fn read_config() -> Option<String> {
    let path = config_path();
    let content = fs::read_to_string(&path).ok()?;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            match key.trim() {
                "lang" => return Some(value.trim().to_string()),
                _ => {}
            }
        }
    }
    None
}

fn main() {
    let cli = Cli::parse();

    if cli.stop {
        force_stop();
        return;
    }

    if is_running() {
        force_stop();
    } else {
        run(cli.model, &cli.device, cli.lang);
    }
}

fn is_running() -> bool {
    if let Ok(pid_str) = fs::read_to_string(PID_FILE) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            return Path::new(&format!("/proc/{}", pid)).exists();
        }
    }
    false
}

fn force_stop() {
    let _ = fs::write(STOP_FILE, "");
    println!("🛑 Stopping...");
}

fn run(model_override: Option<String>, device: &str, lang_override: Option<String>) {
    let _ = fs::remove_file(STOP_FILE);
    let _ = fs::write(PID_FILE, process::id().to_string());

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let model_dir = format!("{}/.local/share/whisper.cpp/models", home);

    // Resolve which model to use
    let model_path = match model_override {
        Some(p) => p,
        None => resolve_model(&model_dir),
    };

    // Download if missing
    if !Path::new(&model_path).exists() {
        if !download_model(&model_path) {
            cleanup();
            return;
        }
    }

    println!("🎤 Loading model...");
    let lang = match lang_override {
        Some(l) => {
            if l == "auto" { None } else { Some(l) }
        }
        None => match read_config() {
            Some(l) if l != "auto" => Some(l),
            _ => None,
        },
    };
    let transcriber = match transcriber::Transcriber::new(&model_path, lang) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("❌ Whisper: {}", e);
            cleanup();
            return;
        }
    };

    println!("🎤 Starting capture...");
    let capture = match audio::AudioCapture::new(device) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Audio: {}", e);
            cleanup();
            return;
        }
    };

    // Start overlay
    let ov = Arc::new(overlay::Overlay::new());
    let ov_clone = ov.clone();
    let overlay_stop = Arc::new(AtomicBool::new(false));
    let overlay_stop_clone = overlay_stop.clone();
    thread::spawn(move || {
        ov_clone.show(&overlay_stop_clone);
    });

    media::pause_all();
    println!("🎤 Listening... (run 'dictate' again to stop)");

    // Handle Ctrl+C
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_clone = stop_flag.clone();
    ctrlc::set_handler(move || {
        stop_flag_clone.store(true, std::sync::atomic::Ordering::SeqCst);
    }).ok();

    let block_samples = TARGET_RATE * BLOCK_SECS;
    let mut offset: usize = 0;

    while !Path::new(STOP_FILE).exists() && !stop_flag.load(std::sync::atomic::Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(100));

        // Compute volume from the very latest audio (62ms window for real-time feel)
        if let Some(recent) = capture.get_latest(TARGET_RATE / 16) {
            let rms = (recent.iter().map(|&s| s * s).sum::<f32>() / recent.len() as f32).sqrt();
            ov.update_volume(rms * 40.0);
        }

        // Transcribe complete blocks
        while let Some(block) = capture.get_block(offset, block_samples) {
            match transcriber.transcribe(&block) {
                Some(text) => {
                    let text = clean_whisper_text(&text);
                    if !text.is_empty() {
                        println!("📝 {}", text);
                        typer::type_text(&text);
                    }
                }
                None => {}
            }
            offset += block_samples;
        }
    }

    // Stop overlay
    overlay_stop.store(true, std::sync::atomic::Ordering::SeqCst);

    // Flush remaining audio
    if let Some(remaining) = capture.get_remaining(offset) {
        if remaining.len() > TARGET_RATE / 2 {
            if let Some(text) = transcriber.transcribe(&remaining) {
                let text = clean_whisper_text(&text);
                if !text.is_empty() {
                    println!("📝 {}", text);
                    typer::type_text(&text);
                }
            }
        }
    }

    media::play_all();
    println!("✅ Done");
    cleanup();
}

fn cleanup() {
    let _ = fs::remove_file(PID_FILE);
    let _ = fs::remove_file(STOP_FILE);
}

fn clean_whisper_text(text: &str) -> String {
    const IGNORE_WORDS: &[&str] = &[
        "Musique",
        "Music",
        "Bruit",
        "Noise",
        "Applaudissements",
        "Applause",
        "Rires",
        "Laughter",
        "BLANK_AUDIO",
        "blank_audio",
    ];

    let re_brackets = regex::Regex::new(r"\[[^\]]*\]").unwrap();
    let re_asterisks = regex::Regex::new(r"\*[^*]+\*").unwrap();
    let re_dots = regex::Regex::new(r"(\.{2,}|…+)").unwrap();

    let mut cleaned = text.to_string();
    cleaned = re_brackets.replace_all(&cleaned, "").to_string();
    cleaned = re_asterisks.replace_all(&cleaned, "").to_string();
    cleaned = re_dots.replace_all(&cleaned, "").to_string();

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let custom_path = format!("{}/.config/dictaway/filters", home);
    if let Ok(content) = fs::read_to_string(&custom_path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Ok(re) = regex::Regex::new(line) {
                cleaned = re.replace_all(&cleaned, "").to_string();
            }
        }
    }

    cleaned.split_whitespace()
        .filter(|word| !IGNORE_WORDS.iter().any(|w| w.eq_ignore_ascii_case(word)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// List of available whisper models: (name, filename, size_label, vram, speed, quality)
const MODELS: &[(&str, &str, &str, &str, &str, &str)] = &[
    ("tiny",     "ggml-tiny.bin",     "75 MB",  "< 1 GB", "⚡⚡⚡", "Basic"),
    ("base",     "ggml-base.bin",     "142 MB", "~1 GB",  "⚡⚡",  "Decent"),
    ("small",    "ggml-small.bin",    "466 MB", "~2 GB",  "⚡",    "Good"),
    ("medium",   "ggml-medium.bin",   "1.5 GB", "~5 GB",  "Slow",  "Very good"),
    ("large-v3", "ggml-large-v3.bin", "2.9 GB", "~10 GB", "V slow","Excellent"),
];

/// Find an existing model, or run first-time setup to pick one.
fn resolve_model(model_dir: &str) -> String {
    // Check if any model already exists
    for (_, filename, _, _, _, _) in MODELS {
        let path = format!("{}/{}", model_dir, filename);
        if Path::new(&path).exists() {
            return path;
        }
    }

    // No model found — first-run setup
    eprintln!("🎤 No whisper model found. Let's pick one!\n");
    eprintln!("  #  Model       Size     GPU VRAM   Speed    Quality");
    eprintln!("  ──────────────────────────────────────────────────────");
    for (i, (_, _, size, vram, speed, quality)) in MODELS.iter().enumerate() {
        let marker = if i == 2 { " ← recommended" } else { "" };
        eprintln!("  {}  {:10}  {:7}  {:9}  {:7}  {}{}", i + 1, MODELS[i].0, size, vram, speed, quality, marker);
    }
    eprintln!();
    eprintln!("  💡 No GPU? All models work on CPU too (just slower).");
    eprintln!();

    loop {
        eprint!("  Pick a model [1-5] (default: 3): ");
        io::stderr().flush().ok();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            eprintln!("  Using small (default)");
            return format!("{}/{}", model_dir, MODELS[2].1);
        }

        let input = input.trim();
        let idx = if input.is_empty() {
            2 // default = small
        } else {
            match input.parse::<usize>() {
                Ok(n) if n >= 1 && n <= MODELS.len() => n - 1,
                _ => {
                    eprintln!("  Enter a number 1-5");
                    continue;
                }
            }
        };

        eprintln!("  ✅ Selected: {} ({})\n", MODELS[idx].0, MODELS[idx].2);
        return format!("{}/{}", model_dir, MODELS[idx].1);
    }
}

/// Download a model file from HuggingFace via curl.
fn download_model(model_path: &str) -> bool {
    let filename = Path::new(model_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "ggml-small.bin".to_string());

    let url = format!("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}", filename);
    let model_dir = Path::new(model_path).parent().unwrap_or(Path::new("."));

    eprintln!("⬇️  Downloading {}...", filename);
    fs::create_dir_all(model_dir).ok();

    let status = process::Command::new("curl")
        .args(["-L", "--progress-bar", "-o", model_path, &url])
        .stdin(process::Stdio::inherit())
        .stdout(process::Stdio::inherit())
        .stderr(process::Stdio::inherit())
        .status();

    match status {
        Ok(s) if s.success() => {
            eprintln!("✅ Model downloaded!\n");
            true
        }
        _ => {
            eprintln!("❌ Download failed. Run manually:");
            eprintln!("   mkdir -p {}", model_dir.display());
            eprintln!("   curl -L -o {} {}", model_path, url);
            fs::remove_file(model_path).ok();
            false
        }
    }
}
