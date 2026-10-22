//! Printing into a pipe that has already closed must end quietly, not panic.

#![cfg(unix)]

use std::process::{Command, Stdio};

#[test]
fn a_closed_stdout_does_not_panic() {
    let config = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/providers.json");
    let mut child = Command::new(env!("CARGO_BIN_EXE_pampas"))
        .args(["usage", "--config", config])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("starting pampas");
    // Close the read end before the program writes (it may occasionally write first, which also passes).
    drop(child.stdout.take());
    let out = child.wait_with_output().expect("waiting for pampas");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked"), "{stderr}");
}
