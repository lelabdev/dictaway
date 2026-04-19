use std::io::Write;
use std::process::{Command, Stdio};

pub fn type_text(text: &str) {
    let mut child = match Command::new("wtype")
        .stdin(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ wtype: {}", e);
            return;
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = write!(stdin, "{} ", text);
    }

    let _ = child.wait();
}
