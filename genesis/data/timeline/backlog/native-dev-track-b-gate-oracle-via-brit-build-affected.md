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

## 2026-11-04 correction (session a340d2e2)

The oracle had a separate correctness defect, now fixed, that this track's receipt must also catch.
On a device with the bin store, `rakia` on PATH already resolves to the `bin-launch` alias of
`brit build`, so the name was not what made selection wrong. rakia-core 0.1.0's
`discover_manifests` walked the sibling worktrees inside the checkout (`.worktrees/`,
`.claude/worktrees/`): 13 copies and 296 manifests on this device. Every `resolved_depends` entry was
then duplicated, and `topo_sort_affected` never released a dependent, so the oracle selected exactly
what path selection does and a pin move did not re-select its consumers.

Fixed by rakia#3 (rakia-core/rakia-brit 0.1.1: tracked-manifest discovery, distinct in-degree) and
brit#4/#5 (brit-cli 0.1.7). Monorepo pins: rakia `a0dc6983f`, brit `b8a5f4b34`. Verified with all
13 sibling worktrees present: a `sophia` change plans `elohim:build-angular`, `build-site-image` and
`seed-content`; a DNA change plans `build-happ`, the edge image and the steward build.

**Receipt addition:** besides "a reuse hit with no `rakia` on PATH", dependents are planned with
sibling worktrees present, and gate-runner's shadow-oracle test is green
(workspace-discipline row 81).
