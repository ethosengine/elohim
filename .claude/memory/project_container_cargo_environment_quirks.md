---
name: container-cargo-environment-quirks
description: "This container lacks cargo-nextest (gospel says it's installed — use plain cargo test) and /projects-volume cargo builds intermittently fail fingerprint writes (ENOENT) — /tmp target dirs work"
metadata: 
  node_type: memory
  type: project
  originSessionId: a30add8e-de4b-440d-abac-d63a71258153
---

Two container-environment facts that contradict expectations (observed 2026-06-06, epr/slice2 branch):

1. **`cargo-nextest` is NOT installed in this container** despite root CLAUDE.md saying it lives at `/opt/rust/cargo/bin/cargo-nextest`. `cargo nextest` fails with "no such command" — and when piped (`| tail`), the pipe masks the exit code, so a `&&` chain continues and verification silently doesn't run. Use plain `cargo test --lib --bins <filter>`; never pipe a gate command's output if its exit code feeds a `&&` chain.

2. **Cargo builds with target dirs on the `/projects` volume intermittently fail** with `failed to write .../.fingerprint/...: No such file or directory` mid-compile (seen in-tree AND in a fresh `/projects/.cargo-target-pool` slot; disk 52%, inodes 3%, no concurrent cargo, pool log idle). `/tmp` target dirs (`CARGO_TARGET_DIR=/tmp/cargo-<crate>`) build reliably. Subagents independently discovered the same workaround ("pool slot filesystem permission issue").

**Why:** verification claims based on nextest in this container are unreliable; and a red Rust gate may be the volume, not the code.

**How to apply:** for Rust gates here, run `cargo test` with `CARGO_TARGET_DIR=/tmp/cargo-<crate>` (keep `RUSTFLAGS=""` for native, the getrandom custom flag for elohim-storage), and check exit codes without pipes. Related: [[cargo-target-dir-for-native-builds]].
