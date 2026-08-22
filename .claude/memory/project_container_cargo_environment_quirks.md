---
index: false
id: project-container-cargo-environment-quirks
name: container-cargo-environment-quirks
title: Container cargo quirks
description: "No nextest here (plain cargo test, unpiped exit codes); /projects target dirs ENOENT → use /tmp; WASM-warmed slot corrupts native builds; explicit cd per gate."
metadata: 
  node_type: memory
  type: project
  originSessionId: a30add8e-de4b-440d-abac-d63a71258153
  modified: 2026-07-31T01:06:16.488Z
cites:
  - .husky/pre-push
  - .claude/hooks/cargo-disk-guard.py
---

Two container-environment facts that contradict expectations (observed 2026-06-06, epr/slice2 branch):

1. **`cargo-nextest` is NOT installed in this container** despite root CLAUDE.md saying it lives at `/opt/rust/cargo/bin/cargo-nextest`. `cargo nextest` fails with "no such command" — and when piped (`| tail`), the pipe masks the exit code, so a `&&` chain continues and verification silently doesn't run. Use plain `cargo test --lib --bins <filter>`; never pipe a gate command's output if its exit code feeds a `&&` chain.

2. **Cargo builds with target dirs on the `/projects` volume intermittently fail** with `failed to write .../.fingerprint/...: No such file or directory` mid-compile (seen in-tree AND in a fresh `/projects/.cargo-target-pool` slot; disk 52%, inodes 3%, no concurrent cargo, pool log idle). `/tmp` target dirs (`CARGO_TARGET_DIR=/tmp/cargo-<crate>`) build reliably. Subagents independently discovered the same workaround ("pool slot filesystem permission issue").

Observed 7x total incl. the PRE-PUSH HOOK's doorway AND storage clippy gates (pool slots; 2026-06-06 it was PERSISTENT across retries, not intermittent — fingerprint-dir clearing did NOT help, a different crate ENOENTs each run).

**The non-bypass fix (validated, push landed on the 6th run):** replace each affected pool slot with a symlink to /tmp — `rm -rf /projects/.cargo-target-pool/family/<fam>/<slot>/<profile> && ln -s /tmp/cargo-target/pool-<slot>-<profile> <that path>` — the hook keeps its CARGO_TARGET_DIR, writes land on /tmp, gates run for real. Pre-empt ALL native slots at once (doorway, elohim-storage, steward__node, sweettest, crates) or you burn one gate-run per slot discovering them serially. /tmp is ephemeral: slots go cold on container recycle (acceptable). `git push --no-verify` remains the last resort only after gates were manually verified on /tmp.

**Cleaner one-shot variant (validated 2026-07-31, angular22 integration push):** the pre-push hook's `gate_pool_slot()` honors a `CARGO_TARGET_POOL_ROOT` env override — `CARGO_TARGET_POOL_ROOT=/tmp/cargo-pool-fallback git push …` reroutes EVERY gate slot to /tmp in one move, no symlinks, no pool mutation (pre-create `/tmp/cargo-pool-fallback/family/` — the function requires `$root/family` to exist — and pre-seed `family/<fam>/<flat-ws>/dev` with a warm /tmp build to skip the cold rebuild). Recurred right after a workspace restart; a freshly-created debug dir on /projects failed identically, so it's the volume, not stale state.

3. **DNA/WASM workspace contract (validated 2026-06-09, native-content-graph-seam, executing the substrate_signal migration).** The PreToolUse cargo hook DENIES a bare `just check` on the DNA workspace for lacking `CARGO_TARGET_DIR` — even though gospel says DNA/WASM workspaces are "plain cargo, use ./target." For a one-off DNA **type-check**, run raw: `RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/dna-check cargo check -p <zome> --target wasm32-unknown-unknown` (satisfies the hook + dodges the ENOENT; a one-off /tmp target does NOT affect `hc dna pack`, which uses ./target). For DNA **native unit tests** (e.g. integrity validators — they run on the host, not wasm): `RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dna-test cargo test -p <zome> <filter>` — RUSTFLAGS MUST be empty; the wasm getrandom flag breaks native linking. **`just pack` cannot use a /tmp target** (it canonicalizes ./target) so it hits the `/projects` fingerprint-ENOENT — i.e. DNA **pack-verification is practically deferred to deploy-time**; for a non-deploying slice the correctness gate is wasm-check-compiles + native-tests-pass, not pack.

4. **A native-flag build (`RUSTFLAGS=""`) in a pool slot previously built with the WASM `getrandom_custom` backend CORRUPTS that slot** (observed 2026-06-27, lens-market S1–S9 slice). The mixed-flag reuse invalidates the slot's fingerprints → overlay-fs whiteout → the same `failed to write .../.fingerprint/...: No such file or directory` ENOENT as point 2, but here it has a *cause*, not just bad luck: a single pool slot is NOT safe to share between native (`RUSTFLAGS=""`) and WASM (`getrandom_backend="custom"`) builds of elohim-storage. Fix = the documented `/tmp` target (point 2/3); it carried the rest of the slice cleanly. **Pre-empt:** for elohim-storage native gates, point `CARGO_TARGET_DIR` at `/tmp` from the first invocation rather than reusing a WASM-warmed pool slot.

5. **Persisted shell cwd silently tests the WRONG crate.** A `cd …/elohim-facings` from an earlier Bash call persisted (working directory carries between calls), so a later "run storage tests" ran against the *facings* crate twice before it was caught — green, but proving nothing about storage. **Same family as point 1** (a gate that looks like it ran but didn't measure what you think). Use an explicit absolute `cd <crate>` (or `cargo … --manifest-path`) per build; never rely on inherited cwd for a verification gate.

**Why:** verification claims based on nextest in this container are unreliable; a red Rust gate may be the volume (or a mixed-flag-poisoned slot), not the code; and a green gate may have measured the wrong crate.

**How to apply:** for Rust gates here, run `cargo test` with `CARGO_TARGET_DIR=/tmp/cargo-<crate>` (keep `RUSTFLAGS=""` for native, the getrandom custom flag for elohim-storage), and check exit codes without pipes. For the pre-push hook: slot-symlink workaround above. Related: [[cargo-target-dir-for-native-builds]].
