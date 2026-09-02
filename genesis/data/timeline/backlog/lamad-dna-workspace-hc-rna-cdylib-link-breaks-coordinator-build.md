---
id: "backlog-lamad-dna-workspace-hc-rna-cdylib-link-breaks-coordinator-build"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "lamad DNA workspace cannot rebuild content_store — hc-rna cdylib fails wasm linking (undefined hdk host symbols), blocking coordinator-only releases"
slug: "lamad-dna-workspace-hc-rna-cdylib-link-breaks-coordinator-build"
written: "2026-09-02"
author: "session-2026-09-01-adoption-ceremony"
status: "in-tree"
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

## Root cause + fix (2026-09-02, systematic-debugging pass)

**Reproduced** as filed (`cargo build --release --target wasm32-unknown-unknown -p content_store`
→ rust-lld undefined `__hc__*` in `libhdk-d051b5…`). **Then falsified the title's scope:** an
integrity zome with no hc-rna dependency (`imagodei_integrity`, separate workspace) fails the
same way (`__hc__dna_info_2`, `__hc__zome_info_1` from `libhdi`). Root cause is toolchain-wide:
`holochain_wasmer_guest::host_externs!` declares host functions as plain `extern "C"` (no
`#[link(wasm_import_module)]`), and rustc 1.98's rust-lld no longer treats undefined symbols in
a wasm shared object as imports by default. hc-rna was only the first cdylib the elohim
workspace links. CI is unaffected because it builds under the holonix pin with `RUSTFLAGS=""`.

**Fix (two changes, both in-tree):**
1. `hc-rna` is `crate-type = ["rlib"]` — nothing anywhere consumes a cdylib of it (grep for
   `hc_rna.wasm`/`libhc_rna`: none), and a helper that calls hdk host fns is never a valid zome.
2. Every zome crate (10, across the five DNA workspaces) gains a `build.rs` that prints
   `cargo:rustc-link-arg=--import-undefined` **only when `CARGO_CFG_TARGET_ARCH == wasm32`**,
   so a native `cargo test` of a workspace never hands the flag to a non-wasm linker. A
   `.cargo/config.toml` `[target.wasm32-unknown-unknown] rustflags` entry was rejected: the dev
   container exports `RUSTFLAGS` (getrandom backend) and an env `RUSTFLAGS` overrides every
   config rustflags, so it would have been silently inert exactly where it is needed.

**Verified:** all five workspaces `cargo build --release --target wasm32-unknown-unknown` →
`Finished` (elohim 1m19s, imagodei 54s, mishpat 2m06s, node-registry 27s, infrastructure 53s).
Coordinator-only release property: `COORD_BUILD_MARKER=fixcheck-B` → `fixcheck-C` recompiled
ONLY `content_store`; `content_store.wasm` moved (`14c7c015…` → new, marker string present) and
`content_store_integrity.wasm` stayed byte-identical (`2549ddbd…`). Two forced relinks of the
integrity zome with identical inputs produced identical bytes (deterministic). One caveat
recorded honestly: the very first build after the build scripts were introduced produced a
different integrity wasm (`ab658001…`) than every build since; the crate was relinked once more
on the next build and has been stable at `2549ddbd…` for four builds — treat the first build
after a build-script change as a warm-up, not a release. Native-target behaviour of the guard is
argued, not exercised (no `cargo test` run in the DNA workspaces beside the resident mesh).
