---
id: "backlog-lamad-dna-workspace-hc-rna-cdylib-link-breaks-coordinator-build"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "lamad DNA workspace cannot rebuild content_store — hc-rna cdylib fails wasm linking (undefined hdk host symbols), blocking coordinator-only releases"
slug: "lamad-dna-workspace-hc-rna-cdylib-link-breaks-coordinator-build"
written: "2026-09-02"
author: "session-2026-09-01-adoption-ceremony"
status: "open"
priority: "high"
jobs: [elohim-holochain]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "backlog-task-runtime-upgrade-a2o-receipt"
tags: [upgrade-propagation, dna-build, wasm, hc-rna, blocker]
---

## Measured (2026-09-02T00:0xZ, local dev container)

`cd elohim/holochain/dna/elohim && COORD_BUILD_MARKER=receipt-20260901-r2 just build`
(and `cargo build --release --target wasm32-unknown-unknown -p content_store`, with or
without `--keep-going`) fails:

```
Compiling hc-rna v0.1.0 (/projects/elohim/elohim/holochain/rna/rust)
error: linking with `rust-lld` failed: exit status: 1
  rust-lld: error: …/libhdk-d051b5007f55fc83.rlib(…): undefined symbol: __hc__agent_info_1
  rust-lld: error: … undefined symbol: __hc__open_chain_1 / __hc__close_chain_1 / __hc__count_links_1 / __hc__create_link_1 …
error: could not compile `hc-rna` (lib)
```

`hc-rna` (`elohim/holochain/rna/rust`, `crate-type = ["cdylib", "rlib"]` since 2025-12-14) is a
path dependency of the `content_store` zome; cargo compiles its lib target with BOTH crate types
in one rustc invocation, so the cdylib link failure fails the rlib the zome needs. The same
workspace produced `content_store.wasm` at 2026-09-01T20:16Z; rustc is 1.98.0 (toolchain dir
unchanged since 2026-08-26). No `.cargo/config.toml` in either crate. Cause not yet located
(suspects: hdk feature unification differing between the zome's `hdk.workspace = true` (=0.6.0)
and hc-rna's `hdk = "0.6"`; a fresh hdk fingerprint `d051b5…`; link-arg defaults).

## Why it matters

Rung-5 coordinator-only releases are cut by rebuilding the coordinator with
`COORD_BUILD_MARKER` (the `zome_build_info` extern's designed knob). With this break no
coordinator-only bundle can be produced locally; the 2026-09-01 adoption ceremony substituted a
wasm custom-section marker on the existing `content_store.wasm` (same code, different bytes) to
keep the mechanism receipt honest.

## Fix direction

Either make `hc-rna` rlib-only for the wasm target (a cdylib of a helper library that calls hdk
host fns is never a valid zome), or give the cdylib the zome-style link args. Verify by
`just build` producing a byte-different `content_store.wasm` with a byte-identical
`content_store_integrity.wasm` under a new `COORD_BUILD_MARKER`.
