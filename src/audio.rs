use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

const TARGET_RATE: usize = 16000;
const MAX_BUFFER_SECS: usize = 60;

pub struct AudioCapture {
    buffer: Arc<Mutex<Vec<f32>>>,
    _ffmpeg: Child,
}

impl AudioCapture {
    pub fn new(device: &str) -> Result<Self, String> {
        let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));

        let mut ffmpeg = Command::new("ffmpeg")
            .args([
                "-f", "pulse",
                "-i", device,
                "-f", "s16le",
                "-acodec", "pcm_s16le",
                "-ar", &TARGET_RATE.to_string(),
                "-ac", "1",
                "-loglevel", "quiet",
                "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("ffmpeg: {}", e))?;

        let stdout = ffmpeg.stdout.take().ok_or("No ffmpeg stdout")?;
        let buf = buffer.clone();

        // Read ffmpeg output in background thread
        std::thread::spawn(move || {
            let mut reader = std::io::BufReader::new(stdout);
            let mut raw = [0u8; 2]; // i16 = 2 bytes
            loop {
                match reader.read_exact(&mut raw) {
                    Ok(()) => {
                        let sample = i16::from_le_bytes([raw[0], raw[1]]);
                        let f32_sample = sample as f32 / i16::MAX as f32;
                        let mut b = buf.lock().unwrap();
                        b.push(f32_sample);
                        let max = TARGET_RATE * MAX_BUFFER_SECS;
                        let len = b.len();
                        if len > max {
                            b.drain(0..len - max);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            buffer,
            _ffmpeg: ffmpeg,
        })
    }

    pub fn get_block(&self, offset: usize, len: usize) -> Option<Vec<f32>> {
        let buf = self.buffer.lock().unwrap();
        if offset + len <= buf.len() {
            Some(buf[offset..offset + len].to_vec())
        } else {
            None
        }
    }

    /// Get the latest `len` samples from the buffer end (for real-time volume)
    pub fn get_latest(&self, len: usize) -> Option<Vec<f32>> {
        let buf = self.buffer.lock().unwrap();
        if buf.len() >= len {
            Some(buf[buf.len() - len..].to_vec())
        } else {
            None
        }
    }

    pub fn get_remaining(&self, offset: usize) -> Option<Vec<f32>> {
        let buf = self.buffer.lock().unwrap();
        if offset < buf.len() {
            Some(buf[offset..].to_vec())
        } else {
            None
        }
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        let _ = self._ffmpeg.kill();
        let _ = self._ffmpeg.wait();
    }
}
