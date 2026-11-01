---
epr-habit-version: 1
id: careless-reds-cost-seconds
invariant: >-
  A red that a seconds-cheap check can find is found, and is live on the pushing session's run,
  before any gate longer than two minutes starts. Gates run cheapest first by recorded p90. Every
  later check and gate still runs and reports unless the integrator stops the run or the declared
  policy (GATE_ON_RED, pool-policy.json gate_on_red) says stop; nothing decides that silently. An
  unreadable change range refuses, never reads as "nothing changed". Each closed run ends with a
  harvest naming the reds that recur across pushes, the cheap reds that arrived late, and the time
  spent after a first red, so the next integrator picks the slice up instead of re-learning it.
status: red
active: false
checks:
  - "node --test genesis/orchestrator/gate-preflight.test.mjs (each preflight check refuses its miss with a cure and stays silent for unaffected gates; REFUSED reaches the live run; on-red policy order; cheapest-first ordering; harvest slices)"
  - "node --test genesis/orchestrator/prepush-gate-live.test.mjs (run opens before the first check; a pre-gate red is live at once and the push continues; an early pass still refuses; GATE_ON_RED=stop ends at the first red; a stop request is seen; the close follows the hook's exit status and prints the harvest)"
  - "node --test genesis/orchestrator/cargo-affected.test.mjs genesis/orchestrator/serving-receipt.test.mjs (change detection never passes an unreadable range; mandatory receipt refusal still refuses)"
  - "node genesis/orchestrator/gate-live.mjs harvest --last 20 (live read of this device's run logs; green needs the slices it names to shrink across pushes)"
guard: >-
  Never let a recorded red reach `exit 0`: every early pass goes through pass_or_refuse. Never move a
  check into the preflight that is not seconds-cheap on a warm tree, and never make a WARN refuse.
  The preflight names cures; it never runs builds or installs itself. The harvest reads device-local
  run logs only and writes nothing; it is advice to the integrator, never a verdict.
refs:
  - genesis/orchestrator/gate-preflight.mjs
  - genesis/orchestrator/gate-live.mjs
  - genesis/orchestrator/gate-runner.mjs
  - .husky/pre-push.bash
  - genesis/agentic/pool-policy.json
  - genesis/orchestrator/.epr-meta/gate-failures-arrive-live.habit.md
retire-when: "the edit-time and commit-time hooks catch every class the preflight refuses, and the harvest names no recurring or late cheap red across 20 consecutive pushes"
---

DELTA 2026-11-01b (still RED; the red is now a decision, not a bare alarm): operator ruling — continue stays the default "so agents can decide whether to stop and fix or let it run for something they might be looking for; they need to get the signal from the noise." A red line now carries its cure, and a run's first red is followed once by an `outlook`: the gates still to run with their recorded p90 and the exact `gate-live.mjs stop --run` command (`[gate-live] the push keeps reporting: 3 gate(s) still to run, ~102 min (elohim-app ~13m, orchestrator ~5m, memory-ceremony ~84m) — to stop and fix now: …`), delivered through the same `unseen` path gate-live-inject relays. Checks: 171/171 across preflight, gate-live, prepush-gate-live, gate-runner, serving-receipt, cargo-affected. Real-hook e2e on 680f625ca: preflight clear in 0.9 s, gates in cost order, seam-guidance 5 s, epr-app-package 70 s, orchestrator 1095 s, agentic-shift 315 s.

DELTA 2026-11-01 (born RED; checks green, invariant unproven on a real push): the fresh-worktree checklist that agent memory re-learned on 10-15, 10-17, 10-19, 10-22, 10-27, 10-31 and 11-01 is now `gate-preflight.mjs`: submodules, node_modules, element dists (every declared dist export), the storage-client dist, the sophia UMD, the orchestrator's REAL read-set assertion (scoped to changed Rust/manifests: 15 s with a storage .rs change, skipped without one), cargo-deny bans (moved from after ALL CLEAR; WARNs that cargo-deny is absent on this device, so that leg never ran here), and a WARN when a reuse-enabled gate's closure is dirty and will record no evidence. On a fresh worktree it refused the three unbuilt dists that cost the 11-01 chrome landing a ~2 h attempt, in 36 s before scoping. pre-push opens the live run before its first check; 14 pre-gate legs now record reds (`leg_red`) and continue under `continue`; gates run cheapest first. First harvest over this device's last 20 runs: orchestrator/test (the REAL read-set red) in 3 runs, elohim-app/compile in 2, eprfs/compile in 4, and 126 min run after a first red across 11 failed pushes. Green needs a real push where a preflight or cheap red lands within its first two minutes and the next harvest's slices shrink. Checks: gate-preflight 11/11; prepush-gate-live, serving-receipt, gate-live 89/89.
