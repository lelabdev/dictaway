use std::process::Command;

pub fn pause_all() {
    let _ = Command::new("playerctl")
        .args(["--all-players", "pause"])
        .spawn();
}

pub fn play_all() {
    let _ = Command::new("playerctl")
        .args(["--all-players", "play"])
        .spawn();
}
