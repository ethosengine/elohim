---
id: project-container-cargo-environment-quirks
name: container-cargo-environment-quirks
description: "This container lacks cargo-nextest (gospel says it's installed — use plain cargo test) and /projects-volume cargo builds intermittently fail fingerprint writes (ENOENT) — /tmp target dirs work"
metadata: 
  node_type: memory
  type: project
  originSessionId: a30add8e-de4b-440d-abac-d63a71258153
cites:
  - .husky/pre-push
  - .claude/hooks/cargo-disk-guard.py
---

Two container-environment facts that contradict expectations (observed 2026-06-06, epr/slice2 branch):

1. **`cargo-nextest` is NOT installed in this container** despite root CLAUDE.md saying it lives at `/opt/rust/cargo/bin/cargo-nextest`. `cargo nextest` fails with "no such command" — and when piped (`| tail`), the pipe masks the exit code, so a `&&` chain continues and verification silently doesn't run. Use plain `cargo test --lib --bins <filter>`; never pipe a gate command's output if its exit code feeds a `&&` chain.

2. **Cargo builds with target dirs on the `/projects` volume intermittently fail** with `failed to write .../.fingerprint/...: No such file or directory` mid-compile (seen in-tree AND in a fresh `/projects/.cargo-target-pool` slot; disk 52%, inodes 3%, no concurrent cargo, pool log idle). `/tmp` target dirs (`CARGO_TARGET_DIR=/tmp/cargo-<crate>`) build reliably. Subagents independently discovered the same workaround ("pool slot filesystem permission issue").

Observed 7x total incl. the PRE-PUSH HOOK's doorway AND storage clippy gates (pool slots; 2026-06-06 it was PERSISTENT across retries, not intermittent — fingerprint-dir clearing did NOT help, a different crate ENOENTs each run).

**The non-bypass fix (validated, push landed on the 6th run):** replace each affected pool slot with a symlink to /tmp — `rm -rf /projects/.cargo-target-pool/family/<fam>/<slot>/<profile> && ln -s /tmp/cargo-target/pool-<slot>-<profile> <that path>` — the hook keeps its CARGO_TARGET_DIR, writes land on /tmp, gates run for real. Pre-empt ALL native slots at once (doorway, elohim-storage, steward__node, sweettest, crates) or you burn one gate-run per slot discovering them serially. /tmp is ephemeral: slots go cold on container recycle (acceptable). `git push --no-verify` remains the last resort only after gates were manually verified on /tmp.

**Why:** verification claims based on nextest in this container are unreliable; and a red Rust gate may be the volume, not the code.

**How to apply:** for Rust gates here, run `cargo test` with `CARGO_TARGET_DIR=/tmp/cargo-<crate>` (keep `RUSTFLAGS=""` for native, the getrandom custom flag for elohim-storage), and check exit codes without pipes. For the pre-push hook: slot-symlink workaround above. Related: [[cargo-target-dir-for-native-builds]].
