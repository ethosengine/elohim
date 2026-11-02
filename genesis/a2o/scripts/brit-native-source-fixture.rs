//! Native fixture: no helper process or network; the harness edits B while A waits.
use std::{
    env, fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

fn main() {
    let args: Vec<_> = env::args_os().collect();
    let input = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    fs::write(output.join("ready"), b"ready").expect("ready marker");
    let deadline = Instant::now() + Duration::from_secs(30);
    while !output.join("continue").exists() {
        assert!(Instant::now() < deadline, "harness continuation missing");
        thread::sleep(Duration::from_millis(10));
    }
    fs::write(
        output.join("answer"),
        fs::read(input.join("value")).expect("fixed input"),
    )
    .expect("declared answer");
}
