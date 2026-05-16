---
name: cargo-target-dir-for-native-builds
description: "Native cargo builds in elohim-storage / sweettest / doorway accumulate target/ dirs at workspace roots (multi-G each) unless `CARGO_TARGET_DIR` points at the pool slot. Set it in every cargo invocation."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: ca911629-dfdd-46f5-8bb1-e936364bea8e
---

Native cargo builds (NOT WASM) should set `CARGO_TARGET_DIR` to the cargo-target-pool slot to avoid silently filling `/projects` with legacy `target/` dirs at each workspace root.

**Why:** session-start banner reminds operators to set CARGO_TARGET_DIR per workspace. Subagent dispatches often forget. The legacy `target/` accumulates and `cargo-pool legacy-targets --clean` only finds it after the fact. Sweettest's `elohim/holochain/target/debug` hit 29G in one shift — recovered post-hoc, but should have been pooled from the start.

**How to apply:** prepend or `env`-set in every native cargo invocation:

```bash
# elohim-storage native (dev)
cd elohim/elohim-storage && \
  RUSTFLAGS="" \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo nextest run --lib services::recovery_flow_projector

# elohim-storage native (release)
cd elohim/elohim-storage && \
  RUSTFLAGS="" \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/release \
  cargo build --release

# sweettest native (dev)
cd elohim/holochain/tests/sweettest && \
  env -u RUSTFLAGS \
  BINDGEN_EXTRA_CLANG_ARGS="-I/usr/lib/clang/20/include" \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__holochain__tests__sweettest/dev \
  cargo nextest run --no-run --tests

# steward/node native
cd steward/node && \
  RUSTFLAGS="" \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/steward__node/dev \
  cargo check

# doorway/doorway-service native
cd doorway/doorway-service && \
  RUSTFLAGS="" \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/doorway__doorway-service/dev \
  cargo check
```

**DO NOT redirect WASM workspaces.** Per CLAUDE.md: `elohim/holochain/dna/*` workspaces use plain cargo with no `CARGO_TARGET_DIR` override — `hc dna pack` canonicalizes `./target`. These stay at `elohim/holochain/dna/{elohim,imagodei}/target` (small WASM artifacts).

**Pool family rule:** family = git branch name. Currently `dev`. If working in a worktree, the family changes — query with `bash genesis/agentic/bin/cargo-pool key` from inside the worktree dir.

**Recovery if forgotten:** `bash genesis/agentic/bin/cargo-pool legacy-targets --clean --yes` finds and removes any non-pool `target/` dirs.

Linked: `project_devspace_disk_cleanup_procedure.md`; `feedback_pvc_threshold_and_recovery.md`.
