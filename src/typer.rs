use std::process::Command;

pub fn type_text(text: &str) {
    match Command::new("wtype").arg(text).spawn() {
        Ok(mut child) => {
            let _ = child.wait();
        }
        Err(e) => eprintln!("❌ wtype: {}", e),
    }
}
