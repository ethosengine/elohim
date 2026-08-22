---
index: false
id: project-sweettest-native-build-env
name: sweettest-native-build-env
title: Sweettest native build env
description: "Sweettest needs RUSTFLAGS=\"\" (WASM getrandom flag breaks native link), BINDGEN_EXTRA_CLANG_ARGS for clang-21; `just pack` (not build) refreshes the .dna bundle."
metadata: 
  node_type: memory
  type: project
  originSessionId: dda22ff0-818e-4f87-8398-38ed1ef4e174
cites:
  - elohim/holochain/tests/sweettest
---

Running `cargo test -p elohim_sweettest` (elohim/holochain/tests/sweettest) in this devspace image (verified 2026-06-05, Task-2 formation coordinators):

- **`RUSTFLAGS=""` required** — the ambient `--cfg getrandom_backend="custom"` (set for Holochain WASM builds) leaks into the native sweettest build and fails at link with `undefined symbol: __getrandom_v03_custom`. Same class as the doorway/steward rule in CLAUDE.md, but sweettest is easy to miss because it lives under elohim/holochain/.
- **`BINDGEN_EXTRA_CLANG_ARGS="-I/usr/lib/clang/21/include"` required** — `datachannel-sys` bindgen can't find `stdbool.h` because `clang -print-resource-dir` returns empty in this image.
- **`just pack`, not `just build`** — in the DNA workspace, building WASM alone leaves the old packed bundle at `dna/elohim/workdir/<name>.dna`; sweettests load the PACKED dna, so a stale bundle yields `ZomeFnNotExists` for freshly added externs. Pack before testing new zome fns.
- sccache spawn ENOENT flake recurred once (`could not execute process sccache … never executed`) — re-run fixes; matches [[sccache-spawn-enoent-rca]].
- Known harness flake: `qahal_collab_t0_test::two_conductor_t0_collab_end_to_end` can time out on `exchange_peer_info` (30s) under load — peer-discovery flake, not a code regression; verify by whether YOUR symbols appear in the failure before chasing.
