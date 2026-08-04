---
id: "backlog-holochain-iroh-dep-verification-pack"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Verification research pack: dalek 5.0.0 digest::crypto_common fix, kitsune2 bootstrap wire stability 0.6.0→0.6.3, signal_url vs relay_url semantics under transport-iroh"
slug: "holochain-iroh-dep-verification-pack"
written: "2026-08-04"
author: "holochain-iroh convergence campaign (Wave 1 Lane D)"
status: "backlog"
priority: "medium"
tags: [research, read-only, iroh, kitsune2, dalek, bootstrap, wave-1, codex-claimable]
cites:
  - genesis/docs/superpowers/plans/2026-08-04-holochain-iroh-convergence-upgrade-campaign.md
---

# Dep verification research pack (Wave 1, Lane D — claimable by any agent; read-only)

Task D1 of the convergence campaign plan. Write-set: ONE new doc,
`genesis/docs/content/elohim-protocol/history/2026-08-04-holochain-iroh-dep-verification-pack.md`.
Everything else is read-only. Disjoint from Lanes A/B/C.

## The three open questions (each answer: CONFIRMED / REFUTED / STILL-UNKNOWN, with URLs or file:line evidence — no guesses)

1. **Did `curve25519-dalek 5.0.0` final (published 2026-07-06) actually fix the published-source bug** (`digest::crypto_common` path mismatch) that existed in `5.0.0-pre.1` (2025-09-04) and motivated elohim-storage's `iroh =0.92` freeze? Read the dalek-cryptography changelog/commit history between `5.0.0-pre.1` and `5.0.0`. This gates confidence in the Lane-A pin lift (which proceeds regardless on `cargo test` evidence, but the changelog answer belongs on record).
2. **Did the kitsune2 bootstrap wire protocol change at all between the kitsune2 versions Holochain 0.6.0 and 0.6.3 consume** (0.4.0-dev.2 → 0.4.1)? Our custom bootstrap server implementation is `doorway/doorway-service/src/bootstrap/k2.rs`. Diff the kitsune2 bootstrap crate (`kitsune2_bootstrap_srv` and the client side) across those versions; identify any request/response shape or endpoint change and check each against our k2.rs implementation. Believed-stable (bootstrap is architecturally separate from signal/relay) but unverified.
3. **What are the precise `signal_url` vs `relay_url` semantics in Holochain 0.6.3 when `transport-iroh` is active?** The 0.6→0.6.1 upgrade guide shows both fields in the example conductor YAML without explaining the split. Read `kitsune2_transport_iroh` 0.4.1 config source: which fields does the iroh transport consume (`relay_url`, `base64_auth_material`, `network.advanced.irohTransport.*`), and which are tx5-only leftovers (`signal_url`, `webrtc_config`). This feeds the Wave-2 transport-flip design directly.

## DoD

The pack doc written with per-question verdict + evidence, committed path-limited. Useful starting URLs: `https://raw.githubusercontent.com/holochain/kitsune2/v0.4.1/...`, `https://github.com/dalek-cryptography/curve25519-dalek` (CHANGELOG.md), holochain tags `holochain-0.6.0`/`holochain-0.6.3` for their kitsune2 pin values. Read-only elsewhere; commit-only.
