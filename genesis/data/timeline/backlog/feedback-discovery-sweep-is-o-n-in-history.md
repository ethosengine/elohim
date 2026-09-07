---
id: "backlog-feedback-discovery-sweep-is-o-n-in-history"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Feedback discovery on a long-lived peer is O(N) in the mesh's whole history — the projector rotates 8 subscription members per 60 s sweep with no cursor and republishes only after a clean sweep, so one act's convergence grew 107 s → 333 s across a day of mesh rounds (contract §3 promised cost ∝ subscribed set, not history)"
slug: "feedback-discovery-sweep-is-o-n-in-history"
written: "2026-09-07"
author: "overnight shift 2026-09-07 (slice-1 close-out finding)"
status: "open"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:dataplane-convergence"
tags: [accountable-correction, feedback-projector, discovery, scale, slice-2]
---

**Evidence (household mesh, 2026-09-07 rounds 045232Z/051633Z/055414Z):** `feedback_projector.rs` visits `MAX_MEMBERS_PER_SWEEP = 8` members per `SWEEP_INTERVAL_SECS = 60` tick and `publish_generation` republishes only on a tick that ends with nothing unvisited and nothing pending. The subscription set is every content record and every discovered correction the peer has ever seen (no retirement, no cursor): 14 → 44 → 127 members over the day; measured acceptance→application lag 106.6 s, 212.5 s, 252.6 s, 332.8 s = `ceil(N/8)` sweeps. By round 4 a single act took >25 min at the product default. The a2o lane pins `ELOHIM_FEEDBACK_SWEEP_SECONDS=5` and derives budgets from the peer's live N; the product default is untouched.

**Why it matters:** contract `2026-09-06-accountable-correction-contract.md` §3 says discovery cost stays proportional to the subscribed set, not to history — the set as implemented IS the history. A peer stewarding thousands of records would converge a correction in hours.

**Cure candidates (slice 2, D0 inventory row for the correction act class):** (a) retire members whose target has no open/unapplied acts after K clean sweeps and re-arm them on notification or on a new act reference; (b) sweep priority by "last new act seen" so hot targets rotate faster than cold ones; (c) raise the per-sweep budget adaptively to keep `ceil(N/8)·interval` under a declared convergence bound. Do not raise `MAX_MEMBERS_PER_SWEEP` blindly — it multiplies DHT `get_links` load.

**Done when:** a mesh round shows acceptance→application lag bounded by a declared constant regardless of the peer's retained member count (measure at N≈100 and N≈1000 seeded members), and the contract §3 wording matches the implementation.
