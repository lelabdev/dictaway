use std::sync::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};

pub struct Transcriber {
    state: Mutex<WhisperState>,
}

impl Transcriber {
    pub fn new(model_path: &str) -> Result<Self, String> {
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| format!("Failed to load model: {:?}", e))?;
        let state = ctx
            .create_state()
            .map_err(|e| format!("Failed to create state: {:?}", e))?;
        Ok(Self {
            state: Mutex::new(state),
        })
    }

    pub fn transcribe(&self, samples: &[f32]) -> Option<String> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("fr"));
        params.set_print_progress(false);
        params.set_print_timestamps(false);
        params.set_print_special(false);

        let mut state = self.state.lock().unwrap();
        state.full(params, samples).ok()?;

        let n = state.full_n_segments();
        let mut text = String::new();
        for i in 0..n {
            if let Some(seg) = state.get_segment(i) {
                if let Ok(s) = seg.to_str() {
                    let s = s.trim();
                    if !s.is_empty() {
                        if !text.is_empty() {
                            text.push(' ');
                        }
                        text.push_str(s);
                    }
                }
            }
        }

        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }
}
