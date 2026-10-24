//! Session identity stays below the runtime and transport layers.
#[test]
fn session_boundary() {
    let source = include_str!("../src/session.rs");
    for forbidden in [
        "tokio::",
        "reqwest::",
        "hyper::",
        "axum::",
        "iroh::",
        "libp2p::",
        "crate::",
        "super::",
        "mod ",
        "include!",
    ] {
        assert!(
            !source.contains(forbidden),
            "session primitive imports {forbidden}"
        );
    }
    for line in source.lines().filter(|line| line.starts_with("use ")) {
        assert!(
            line.starts_with("use std::") || line.starts_with("use serde::"),
            "non-std/serde import: {line}"
        );
    }
}
