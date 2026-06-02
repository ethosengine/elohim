---
name: feedback_sweettest_native_build_env
description: "DNA Integration (sweettest) is a NATIVE build — it needs the native-build env (cmake/clang/LIBCLANG_PATH), the WASM RUSTFLAGS cleared, and a ~90/150-min stage/pipeline budget; three stacked failures, each revealing the next."
metadata:
  node_type: memory
  type: feedback
  originSessionId: 2026-04-24T15-20-orchestrator-pipeline-unstable
cites:
  - elohim/holochain/dna/elohim/flake.nix
  - elohim/holochain/Jenkinsfile
---

**DNA Integration (sweettest) is a NATIVE build and needs the native-build env, not the WASM env.** This generalizes the CLAUDE.md "RUSTFLAGS Override Required" gotcha from the local elohim-storage build to the CI sweettest stage. Three stacked failures shook out, each one only surfacing after the previous was fixed.

**Why (the three layers):**
1. `datachannel-sys` panics `is cmake not installed?` — the Nix devShell must provide `cmake, pkg-config, clang, libclang.lib, openssl, zlib, libsodium` plus `LIBCLANG_PATH`. Fixed in `elohim/holochain/dna/elohim/flake.nix` (`b2c471f5`).
2. After cmake lands, the link fails `undefined reference to __getrandom_v03_custom` — the WASM `RUSTFLAGS=--cfg getrandom_backend="custom"` (set system-wide for Holochain WASM) leaked into the native sweettest compile. The DNA Integration stage must **clear `RUSTFLAGS`** (`c6eb632a`).
3. After it links, the cold sweettest compile alone exceeds a 30-min stage budget — the DNA Integration stage timeout went to ~90min and the pipeline to ~150min (`e29e2e6a`).

**How to apply:**
- Treat the sweettest stage like any native Rust build: clear `RUSTFLAGS`, ensure the cmake/clang toolchain is present, budget for a cold compile.
- If a sweettest stage dies on `cmake not installed` or `__getrandom_v03_custom`, it is an env-leak from the WASM build profile, not a test bug.
- The ~90/150-min budget is the cold-compile floor; warm caches (sccache/nextest archive) bring it down, but never budget the stage at 30min.

Generalizes the root `CLAUDE.md` "RUSTFLAGS Override Required" gotcha to the CI sweettest stage. Pairs with [[project_sweettest_cost_anatomy]] (compile-cost breakdown) and [[feedback_cargo_target_dir_for_native_builds]].
