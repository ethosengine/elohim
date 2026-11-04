---
id: "backlog-native-dev-d3-fork-preserving-space-head-and-ff-only-mirror"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "D3 [integrity]: fork-preserving space head (a set of tips) and a fast-forward-only forge mirror"
slug: "native-dev-d3-fork-preserving-space-head-and-ff-only-mirror"
written: "2026-11-04"
author: "claude (design pass, native-dev-plane spec; steward human:matthew)"
status: "backlog"
priority: "high"
relatedNodeIds:
  - "native-dev-plane-one-derivation-earned-reach-design"
tags: [native-dev-plane, integrity, landing, space-head, mirror, d3, s1b, f3]
---

Source spec: `native-dev-plane-one-derivation-earned-reach-design` (genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md), section 8 (ruling D3), join F3.

## Item (spec section 8) [integrity: Claude-family adversarial review before landing]

2026-11-04 (adversarial review). The steward-space signed `dev` head is the landing record; the forge is a mirror. Two defects make that unsafe as first drafted.

1. **The space head is a set of tips.** After the override delay the lease view exempts the overridden device from acknowledgement and freshness checks (`elohim/eprfs/eprfs-peer/src/lease_view.rs:781`), so two partitioned devices can each accept a landing from the same prior H. When two landings share a prior, both tips are retained as named heads, durably across restarts and reconciliation; neither is dropped or replaced. The fork resolves only by a merge whose candidate contains both tips, landing as an ordinary CAS.
2. **The mirror only fast-forwards.** The forge `dev` is pushed after the space head moves, accepted only if the candidate descends from the forge's current tip. While the space head holds more than one tip, mirroring halts until the merge lands. A retry may refresh the expected prior only to a tip the candidate contains. A forge-side change refuses the push and surfaces as a fork. Mirror lag is shown in the watch view and never waited on.

**Dependencies.** Rides S1b (closure-keyed landing plan: Phase V gates hold nothing, Phase L holds for the CAS window) because both touch the same hook. Depends on track C's push.rs fix (`elohim/brit/gitoxide-core/src/repository/push.rs:707`: an explicit lease skips the fast-forward check). Track D lands first so `claimed` never lands silently. D3 is ranked for the trajectory, not as a wall-clock move (the 3,321 s lease p90 is gate time inside the lease, removed by S1b).

**Receipt.** The worst interleaving: two devices partition at H, both record an override, both land. Then: both tips survive a restart; mirroring refuses until a merge contains both; the merge lands and mirrors as a fast-forward. Also: landing with the forge unreachable.

**retire-when.** The head is a durable tip set, the mirror is fast-forward-only with the partition receipt green, and no explicit-lease push bypasses the fast-forward check.
