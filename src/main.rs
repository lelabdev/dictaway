mod audio;
mod media;
mod overlay;
mod transcriber;
mod typer;

use clap::Parser;
use std::fs;
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
        run(cli.model, &cli.device);
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

fn run(model_override: Option<String>, device: &str) {
    let _ = fs::remove_file(STOP_FILE);
    let _ = fs::write(PID_FILE, process::id().to_string());

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let default_model = format!("{}/.local/share/whisper.cpp/models/ggml-small.bin", home);
    let model_path = model_override.unwrap_or(default_model);

    println!("🎤 Loading model...");
    let transcriber = match transcriber::Transcriber::new(&model_path) {
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

        // Compute volume from recent audio
        if let Some(recent) = capture.get_block(offset, TARGET_RATE / 4) {
            let vol = recent.iter().map(|&s| s.abs()).sum::<f32>() / recent.len() as f32;
            ov.update_volume(vol * 10.0); // amplify for visibility
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
                let text = text.trim();
                if !text.is_empty() {
                    println!("📝 {}", text);
                    typer::type_text(text);
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
    let re = regex::Regex::new(r"\*[^*]+\*|\[[^\]]+\]|\.{2,}|…").unwrap();
    re.replace_all(text, "").trim().to_string()
}
