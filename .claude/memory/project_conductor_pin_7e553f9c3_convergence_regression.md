---
name: project_conductor_pin_7e553f9c3_convergence_regression
description: "Household A/B on 2026-09-24 — deliverability lane passes on fleet pin 25dd2d0be, misses doorway-B convergence 3× on 7e553f9c3; pin held, bisect owed"
metadata:
  node_type: memory
  title: "Conductor pin 7e553f9c3 misses the household's 75 s convergence"
  type: project
  originSessionId: 3a11e1a8-7c12-4116-80f9-fcf59b56702c
  modified: 2026-09-24T13:22:05.591Z
---

**The finding (2026-09-24, household /tmp/hh, same host state).** `epr-app-deliverability` scenario 5, step "within 75 seconds both doorways serve the new browser bundle by content address": on conductor fork **25dd2d0be** (the fleet pin) it passed 102/102 at 05:01Z and 05:25Z (doorway B converged in 27 s); on **7e553f9c3** it missed three times (04:19Z, 04:44Z, 04:47Z) — jessica applies the announced change within 1 s but her green row waits for the DHT election to carry the declaration, and that never landed inside 75 s. CORRECTED 13:05Z the same day: the fleet-pin lane also missed that step at host load 10→16 (the fleet's own roll on this node), and every new-pin run had a load spike inside its 75 s window. Evidence: old pin PASS at load ≤5.5 (3 runs), FAIL at ≥10 (2 runs); new pin FAIL ×3, all with load spikes. Load is the dominant factor; the pin's effect on convergence is **unresolved** — settle it only with a quiet-host A/B (both pins, same tree, load < 6 throughout).

**Suspects, in order:** `0efa40939` (publish selects only locally validated ops — could delay a fresh declaration's publish until validation completes) and `ff2ea44c6` (iroh per-connection receive throttle, 250 ms cooldown after a refused Publish frame). The other commits between the pins: 915acf6bc diagnostics, 8ccffdedb sargable arc reads + covering loc index (the leg-A cure), 61565f320 cap-grant auth, 06923b304/11fbf5e58 hardening, 0f26f6703 Prometheus exporter, cb61633c2 range-bounded chain reads.

**Bisect ready:** prebuilt pair `hc-fork-61565f320d0e` sits between them (contains diagnostics, throttle, sargable, cap-grant; excludes publish fix and later). `scratchpad/bisect-lane.sh 61565f320d0e mid` runs the lane on the batch C tree with an explicit `HOLOCHAIN_BIN` (the launcher treats it as an A/B run). Pass → regression in 06923b304..7e553f9c3 (publish); fail → in 915acf6bc..61565f320 (throttle). Lanes from these runs are evidence, never receipts.

**Also true:** 7e553f9c3 is NOT on the fork remote (`elohim-0.7` = 25dd2d0be); the conductor image job fetches HC_REF by exact SHA, so the pin cannot deploy until the fork branch is fast-forwarded (a push this environment is authorized to make; treated as the operator's call). Batch C landed with the gitlink held at 25dd2d0be (`d64ebe936`).

**Why:** the fleet's leg-A window (58–267 min per pod, 53 restarts in 72 h) is what blocks app delivery, and the pin's sargable-arc fix is its cure — but rolling a pin that regresses convergence trades one red for another.
**How to apply:** before any pin move, run the deliverability lane on the household on that pin (quiet host) AND a control on the fleet pin; bisect with the prebuilt pairs; then fork push + gitlink commit. See [[project_receipt_lane_quiet_host_worktree_prep]], [[project_conductor_fork_verification_traps]], [[project_holochain_evolution_epic]].
