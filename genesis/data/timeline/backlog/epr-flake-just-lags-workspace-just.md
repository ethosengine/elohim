---
id: "backlog-epr-flake-just-lags-workspace-just"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "CI's flake just (1.28, nixos-24.05) lags the workspace just (1.58), so new justfile syntax reds only CI"
slug: "epr-flake-just-lags-workspace-just"
written: "2026-10-27"
author: "claude (overnight integration shift, session 3f9afd7a, steward human:matthew)"
status: "open"
priority: "medium"
ci_status: none
jobs: [elohim-eprfs]
tags: [ci, nix, flake, just, toolchain-drift]
---

# CI's flake just lags the workspace just

## What is wrong

`elohim/epr/flake.nix` takes nixpkgs from the floating `nixos-24.05` branch with no lock,
which ships `just` 1.28.0. Workspaces run `just` 1.58.0. Every syntax just accepted after
1.28 passes locally and in pre-push, then fails in the eprfs pipeline, where
`genesis/agentic/session_envelope_test.py` drives the root justfile.

elohim-eprfs #80 (2026-10-27) was the first instance: `mesh action="status" *args:` is
refused by 1.28 ("Non-default parameter `args` follows default parameter"). The recipe now
declares `*args=""`, which both versions accept and which renders identically.

## Done when

Either the flake's `just` and the workspace's `just` are the same declared version, or a
pre-push check parses the root justfile with the flake's `just` and refuses on error.
`rust-toolchain-pin.test.mjs` is the model: one declared version, one test that refuses a mismatch.
