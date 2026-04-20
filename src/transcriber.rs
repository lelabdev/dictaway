use regex::Regex;
use std::sync::LazyLock;
use std::sync::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};

static NOISE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"\[(?i:musique|music)\]").unwrap(),
        Regex::new(r"\[(?i:bruit|noise|applaudissements|applause|rires|laughter)\]").unwrap(),
        Regex::new(r"\.{2,}").unwrap(),
        Regex::new(r"…+").unwrap(),
    ]
});

fn clean_segment(s: &str) -> Option<&str> {
    for re in NOISE_PATTERNS.iter() {
        if re.is_match(s) {
            return None;
        }
    }
    Some(s)
}

pub struct Transcriber {
    state: Mutex<WhisperState>,
    lang: Option<String>, // None = auto-detect
}

impl Transcriber {
    pub fn new(model_path: &str, lang: Option<String>) -> Result<Self, String> {
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| format!("Failed to load model: {:?}", e))?;
        let state = ctx
            .create_state()
            .map_err(|e| format!("Failed to create state: {:?}", e))?;
        Ok(Self {
            state: Mutex::new(state),
            lang,
        })
    }

    pub fn transcribe(&self, samples: &[f32]) -> Option<String> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(self.lang.as_deref());
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
                    if let Some(clean) = clean_segment(s) {
                        if !clean.is_empty() {
                            if !text.is_empty() {
                                text.push(' ');
                            }
                            text.push_str(clean);
                        }
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
