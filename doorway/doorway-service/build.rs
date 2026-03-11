//! Build script for elohim-doorway
//!
//! Captures git commit hash at build time for version verification.

use std::process::Command;

fn main() {
    // Prefer env vars (set by Docker build args) over git commands (which need .git/)
    let git_hash = std::env::var("GIT_COMMIT_SHORT")
        .ok()
        .filter(|s| !s.is_empty() && s != "unknown")
        .unwrap_or_else(git_short_hash);

    println!("cargo:rustc-env=GIT_COMMIT_SHORT={git_hash}");

    let git_hash_full = std::env::var("GIT_COMMIT_FULL")
        .ok()
        .filter(|s| !s.is_empty() && s != "unknown")
        .unwrap_or_else(git_full_hash);

    println!("cargo:rustc-env=GIT_COMMIT_FULL={git_hash_full}");

    let timestamp = std::env::var("BUILD_TIMESTAMP")
        .ok()
        .filter(|s| !s.is_empty() && s != "unknown")
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());

    println!("cargo:rustc-env=BUILD_TIMESTAMP={timestamp}");

    // Rebuild if env vars or git HEAD changes
    println!("cargo:rerun-if-env-changed=GIT_COMMIT_SHORT");
    println!("cargo:rerun-if-env-changed=GIT_COMMIT_FULL");
    println!("cargo:rerun-if-env-changed=BUILD_TIMESTAMP");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/");
}

fn git_short_hash() -> String {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn git_full_hash() -> String {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
