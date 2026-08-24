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
