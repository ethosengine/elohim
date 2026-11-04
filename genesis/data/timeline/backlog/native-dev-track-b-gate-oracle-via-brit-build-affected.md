---
id: "backlog-native-dev-track-b-gate-oracle-via-brit-build-affected"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Track B: gate-oracle resolves selection through brit build affected, not the deprecated rakia PATH binary"
slug: "native-dev-track-b-gate-oracle-via-brit-build-affected"
written: "2026-11-04"
author: "claude (design pass, native-dev-plane spec; steward human:matthew)"
status: "backlog"
priority: "high"
relatedNodeIds:
  - "native-dev-plane-one-derivation-earned-reach-design"
tags: [native-dev-plane, gate, reuse, brit, track-b, push-gate-pays-once]
---

Source spec: `native-dev-plane-one-derivation-earned-reach-design` (genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md).

## Item (spec §11 Track B)

Write-set: `genesis/orchestrator/gate-oracle.mjs`, `gate-runner.mjs`. Selection resolves via `brit build affected`.

**Verified 2026-11-04.** `gate-oracle.mjs:24-37` is `resolveRakiaBin` (RAKIA_BIN, else `rakia` on PATH, else null). `elohim/rakia/CLAUDE.md` confirms a binary named `rakia` is the deprecated compatibility path and the CLI lives under `brit build`.

**Existing coverage.** The symptom is already filed as `gate-reuse-dark-without-rakia-cli` (status open, high); a dated note citing this spec was added there. This item is the design-aligned cure (brit, not a shim). Close both together.

**Receipt.** A reuse hit with no `rakia` on PATH. Expected: up to a full gate saved (e.g. 5,757 s to 56 s) on a device without the shim.

**retire-when.** `resolveRakiaBin` and the `rakia unavailable` refusal are deleted.
