---
name: dockerfile-target-completeness
description: "Adding any [[bin]], [[bench]], or [[example]] to a Rust crate that builds in Docker requires updating both the dep-cache placeholder stage AND the real-build COPY block — local pre-push gate will not catch this."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 02e13765-e7d1-4ede-93b7-eb321ec8ab6e
---

When a Rust crate's `Cargo.toml` declares a `[[bin]]`, `[[bench]]`, or `[[example]]` target, every such target needs its source file present on disk at manifest-parse time (cargo aborts before compilation otherwise). For crates that build in Docker via the placeholder-then-real-source pattern (canonical example: `elohim/elohim-storage/Dockerfile`), this means **two** places must mirror every declared target:

1. **Dep-cache placeholder stage** — `RUN mkdir -p src benches && echo 'fn main(){}' > src/main.rs && ...` must materialize a file for every declared target. The placeholder build then warms the Docker layer cache for transitive deps.
2. **Real-build COPY block** — `COPY <crate>/<dir> ./<dir>` lines must restore the real sources for every declared target after the cleanup `rm -rf` step. Missing one re-hits the same manifest-parse error at the real-build stage.

**Why:** The local `.husky/pre-push` gate runs `cargo build` against the real workspace, where the real files exist on disk regardless of Cargo.toml declarations. Cargo passes locally. The Docker layer uses synthetic placeholders, so it fails at manifest parse — silently, in CI, 6+ minutes into the build. Saw this hit twice in the same shift (2026-05-17): once for the placeholder layer (39s failure), once for the COPY-back block (376s failure). The graph-native sprint passed pre-push and failed twice in CI for the same root cause split across two stages.

**How to apply:** Anytime a PR touches `Cargo.toml` to add a new `[[bin|bench|example]]` in a crate that has a `Dockerfile` (today: `elohim/elohim-storage`, `doorway/doorway-service`, potentially others), grep the Dockerfile for `mkdir -p` and `COPY <crate>/` blocks and verify both lists mirror the new target. Also: add the new target's directory pattern (e.g. `benches/**`) to the corresponding `build-manifest.json` source list so the orchestrator's change detection triggers a build when the target's files change.

Related: [[orchestrator_predictive_vision]] (build-manifest is the dispatch authority); [[signature_changes_grep_callers]] (small declaration change has callers across the build pipeline that aren't grep-discoverable from the diff).
