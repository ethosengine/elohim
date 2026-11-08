---
id: artifact-store-hc-fork-993cf597a-shem
status: measured
---

# hc fork 993cf597a built natively on shem (Sandy Bridge, no AVX2)

Config inspection (worktree /projects/elohim/.worktrees/hc-993cf597a, detached at 993cf597a): no `.cargo/config.toml` or `.cargo/config`; grep for target-cpu / target-feature / x86-64-v / avx found nothing; no ~/.cargo or /projects cargo config. rust-toolchain.toml pins 1.96.1. Release profile: opt-level "z", lto = true. Nothing emits AVX2.

Env: RUSTUP_TOOLCHAIN=1.96.1, RUSTFLAGS="-C force-frame-pointers=yes", CARGO_BUILD_JOBS=8, CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/hc-fork/holochain/release.

Commands (in the worktree):
1. `cargo build --release --locked --no-default-features --features encryption,wasmer-sys-cranelift,jemalloc -p holochain --bin holochain` -> EXIT=0, 17m 13s (second run; a first run was stopped at the coordinator's hold, incremental reuse).
2. `cargo build --release --locked --no-default-features --features wasmer-sys-cranelift -p holochain_cli --bin hc` -> EXIT=0, 4m 47s.

Wall-clock of the resumed chain: 1320 s (1794114731 -> 1794116051 epoch).

Installed (install -D, 0755): /projects/.claude-config/tools/hc-fork-993cf597a19b/bin/{holochain,hc}

Verification (no SIGILL):
- `holochain --version` -> `holochain 0.7.0` (EXIT=0)
- `hc --version` -> `holochain_cli 0.7.0` (EXIT=0)
