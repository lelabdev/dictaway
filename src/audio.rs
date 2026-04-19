use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use std::sync::{Arc, Mutex};

const TARGET_RATE: u32 = 16000;
const MAX_BUFFER_SECS: usize = 60;

pub struct AudioCapture {
    buffer: Arc<Mutex<Vec<f32>>>,
    _stream: cpal::Stream,
}

impl AudioCapture {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device found")?;

        let config = device
            .default_input_config()
            .map_err(|e| format!("Audio config error: {}", e))?;

        let sample_rate = config.sample_rate();
        let channels = config.channels();
        let fmt = config.sample_format();

        let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));

        let stream = match fmt {
            SampleFormat::F32 => {
                let buf = buffer.clone();
                device
                    .build_input_stream(
                        &config.into(),
                        move |data: &[f32], _| {
                            let converted = convert_chunk(data, channels, sample_rate);
                            let mut b = buf.lock().unwrap();
                            b.extend_from_slice(&converted);
                            trim_buffer(&mut b);
                        },
                        |err| eprintln!("Audio error: {}", err),
                        None,
                    )
                    .map_err(|e| format!("Stream error: {}", e))?
            }
            SampleFormat::I16 => {
                let buf = buffer.clone();
                device
                    .build_input_stream(
                        &config.into(),
                        move |data: &[i16], _| {
                            let f32_data: Vec<f32> =
                                data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                            let converted = convert_chunk(&f32_data, channels, sample_rate);
                            let mut b = buf.lock().unwrap();
                            b.extend_from_slice(&converted);
                            trim_buffer(&mut b);
                        },
                        |err| eprintln!("Audio error: {}", err),
                        None,
                    )
                    .map_err(|e| format!("Stream error: {}", e))?
            }
            _ => return Err(format!("Unsupported sample format: {:?}", fmt)),
        };

        stream
            .play()
            .map_err(|e| format!("Cannot start audio: {}", e))?;

        Ok(Self {
            buffer,
            _stream: stream,
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

    pub fn get_remaining(&self, offset: usize) -> Option<Vec<f32>> {
        let buf = self.buffer.lock().unwrap();
        if offset < buf.len() {
            Some(buf[offset..].to_vec())
        } else {
            None
        }
    }
}

fn convert_chunk(samples: &[f32], channels: u16, sample_rate: u32) -> Vec<f32> {
    let mono = to_mono(samples, channels);
    if sample_rate == TARGET_RATE {
        mono
    } else {
        resample(&mono, sample_rate, TARGET_RATE)
    }
}

fn to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return samples.to_vec();
    }
    samples
        .chunks(channels as usize)
        .map(|ch| ch.iter().sum::<f32>() / channels as f32)
        .collect()
}

fn resample(samples: &[f32], from: u32, to: u32) -> Vec<f32> {
    let ratio = to as f64 / from as f64;
    let new_len = (samples.len() as f64 * ratio) as usize;
    let mut result = Vec::with_capacity(new_len);
    for i in 0..new_len {
        let src_pos = i as f64 / ratio;
        let idx = src_pos as usize;
        let frac = src_pos - idx as f64;
        let s0 = samples.get(idx).copied().unwrap_or(0.0);
        let s1 = samples.get(idx + 1).copied().unwrap_or(0.0);
        result.push(s0 + (s1 - s0) * frac as f32);
    }
    result
}

fn trim_buffer(buf: &mut Vec<f32>) {
    let max = TARGET_RATE as usize * MAX_BUFFER_SECS;
    if buf.len() > max {
        buf.drain(0..buf.len() - max);
    }
}
