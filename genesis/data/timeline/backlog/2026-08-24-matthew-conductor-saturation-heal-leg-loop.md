---
title: "matthew conductor saturation starves the projection-reconcile heal leg — circuit trips every sweep, healed:0"
date: 2026-08-24
status: open
class: self-heal-exhaustion
habit: dataplane-convergence
requires_env: alpha-cluster-6peer
---

Post edge #1378 (00:45Z, commit 9acddc279) matthew's `projectionReconcile` leg cannot drain:
`healed:0` on every sampled sweep for 2.5h, `to_resolve` oscillating 829→1807→1766, breaker
"OPENED the unresponsive-conductor circuit" with `consecutive_timeouts:3` recurring, per-attempt
25s conductor-call timeouts. Conductor independently corroborated saturated: holochain_p2p
NETAUDIT `recv_validation_receipt_received elapsed_s=247.8`. NOT the new doorbell (9 announce
log hits in 2h22m, iroh apply counters flat zero, back-fill ran once clean). Same oscillation
shape existed on the PRE-restart pod at smaller magnitude (Prometheus divergent_actionable
0→14→…→0 across 5h pre-deploy) — pre-existing runtime condition, amplified by restart churn.

Known-class check first (conductor-arc memory): sys-validation CPU spin / CPU throttle starving
reads — check pod CPU throttling before assuming identity/DHT causes. Possible cures live at the
runtime seam (heal-leg batch size/timeout adaptivity; conductor resources), NOT the sync plane.
Gate impact: the quiesce gate reads matthew only → Dataplane Validation skips wholesale while
this holds; the 2026-08-24 fleet confirm of the doorbell arc is blocked behind it.

Secondary lead (same window, possibly compounding, possibly T4-provenance): inventory plane
`"Inventory delta gap"` ×1621 + `apply_snapshot: database is locked` ×90 on matthew. T4
(3738e611c, deployed in this same push) moved inventory scoring onto a command channel — compare
pre-deploy rates for this pattern before attributing; if the rate jumped with #1378, the T4 cut
owns it.

## Post-restart confirmation (2026-08-24 08:28Z) — RESTART DID NOT CURE IT

Edge #1379 (a5607e938) redeployed alpha at ~07:5xZ, restarting matthew's pod. The heal-leg
counters reset (healedTotal 27→4) but the stuck pattern returned immediately:
- 08:07Z (fresh pod): healedTotal 4, pending 866, divergentAnchor 907, sweeps 5
- 08:28Z (+21min):     healedTotal 6, pending 928, divergentAnchor 1004, sweeps 7
i.e. ~2 heals in 20 min while pending + divergentAnchor RISE. Identical to the pre-restart
signature. CONFIRMS this is a standing runtime condition (conductor saturation), NOT
post-restart transient churn — a pod restart resets the loop but does not fix the conductor.

CEILING — operator-owned. The dev-side dataplane code is deployed and locally proven; the
fleet Dataplane Validation gate reads matthew (storage-A) and cannot pass while this holds.
Candidate operator interventions (not dev-loop work): (a) inspect matthew's conductor CPU
throttle / resource limits (conductor-arc memory: sys-validation CPU spin starves reads —
check throttle BEFORE assuming identity/DHT cause); (b) heal-leg batch-size/timeout adaptivity
so a single batch fits under the 25s conductor-call ceiling (a real code cure, but speculative
while the conductor itself reports 247s receipt latency — likely treats a symptom); (c) a
deeper conductor-performance investigation (why is a 6-peer alpha conductor 247s-laggy).
