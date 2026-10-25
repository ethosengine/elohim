---
id: "backlog-che-pinned-toolchain-lacks-wasm32-target"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The workspace image's pinned 1.96.1 toolchain has no wasm32-unknown-unknown target, so WASM legs of pinned cargo gates fail"
slug: "che-pinned-toolchain-lacks-wasm32-target"
written: "2026-10-25"
author: "claude-opus-5-5"
status: "backlog"
priority: "medium"
area: "devspace / gates"
tags: [devspace, toolchain, wasm, pre-push, gates]
cites:
  - genesis/data/timeline/backlog/che-background-shell-git-and-toolchain-env.md
  - genesis/agentic/pool-policy.json
---

# The pinned 1.96.1 toolchain has no wasm32 target

`genesis/agentic/pool-policy.json` `cargo_env_overrides."*"` pins `RUSTUP_TOOLCHAIN=1.96.1` for every
cargo gate project. The workspace image installs 1.96.1 (`che-devworkspaces`
`containers/rust-dev/Dockerfile` lines 68–70) without the `wasm32-unknown-unknown` target. So a gate
recipe that builds for wasm under the pin fails with `error[E0463]: can't find crate for std … the
wasm32-unknown-unknown target may not be installed`:

- `_gate-seam-contracts`: `cargo build --target wasm32-unknown-unknown --no-default-features`.
- `dna-extern-lints` fails the same way when the pin leaks into it, for example from an exported
  `RUSTUP_TOOLCHAIN`.

**Observed:** the landing push of `feat/gate-evidence-reuse`, 2026-10-25, gate `seam-contracts`.

**Cured on this workspace by hand:** `rustup target add wasm32-unknown-unknown --toolchain 1.96.1`.
`/opt/rust` lives on the container overlay root, so the cure is lost at the next workspace restart.

**Durable fix:** the image adds the target for the pinned toolchain, beside its existing 1.96.1 install
(`rustup target add wasm32-unknown-unknown --toolchain 1.96.1`), in both
`containers/rust-dev/Dockerfile` and `containers/udi-plus-mem-rust-nix/Dockerfile`.

That change lives in the che-devworkspaces submodule, which CI does not auto-watch, so force it with
the `[build:conductor]` dispatch tag as CLAUDE.md describes.

Until the image carries it, `run-local-gate.sh` could refuse before launch, naming the cure. It
already refuses a pinned toolchain that is not installed, and could do the same for a gate whose
recipe names `--target wasm32-unknown-unknown`.

**Done when:** a fresh workspace runs `just gate seam-contracts` green with no manual rustup step.
