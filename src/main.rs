mod audio;
mod media;
mod transcriber;
mod typer;

use clap::Parser;
use std::fs;
use std::path::Path;
use std::process;
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
        run();
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

fn run() {
    let _ = fs::remove_file(STOP_FILE);
    let _ = fs::write(PID_FILE, process::id().to_string());

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let model_path = format!("{}/.local/share/whisper.cpp/models/ggml-base.bin", home);

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
    let capture = match audio::AudioCapture::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Audio: {}", e);
            cleanup();
            return;
        }
    };

    media::pause_all();
    println!("🎤 Listening... (run 'dictate' again to stop)");

    let block_samples = TARGET_RATE * BLOCK_SECS;
    let mut offset: usize = 0;

    while !Path::new(STOP_FILE).exists() {
        thread::sleep(Duration::from_millis(500));

        while let Some(block) = capture.get_block(offset, block_samples) {
            match transcriber.transcribe(&block) {
                Some(text) => {
                    let text = text.trim();
                    if !text.is_empty() {
                        println!("📝 {}", text);
                        typer::type_text(text);
                    }
                }
                None => print!("·"),
            }
            offset += block_samples;
        }
    }

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
