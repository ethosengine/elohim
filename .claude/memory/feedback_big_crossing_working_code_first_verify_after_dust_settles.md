---
name: feedback_big_crossing_working_code_first_verify_after_dust_settles
title: "Big crossing — working code first, verify after the dust se…"
description: "Operator 2026-11-05 (re-genesis 1): on a whole-repo crossing the goal is working code across every track; gates/habits measure ONCE after all dev and integration dust settles."
metadata:
  node_type: memory
  title: "Big crossing — working code first, verify after the dust settles"
  type: feedback
  originSessionId: 184beb01-43bc-4c74-9212-9f7189301998
  modified: 2026-11-05T22:52:26.933Z
---

Operator steer during re-genesis 1 (the address crossing, 2026-11-05): "the goal of this large
change will be let's get everything refreshed with this new approach, the goal is working code;
it's ok if our verifications — measuring our habits / verified — come in after ALL the dev and
integration dust settles."

**Why:** a foundation-wide change touches every gate's closure; paying a ~2 h full hook per
closure (and chasing shared-slot artifacts) stalls the integration while nothing real is proven
until the tracks compose anyway. Verification is one pass over the composed result, not a toll
on each intermediate landing.

**How to apply:** on a declared big crossing — land closures with the operator-approved hook
bypass (they run the exact `HUSKY=0` push), keep workers on compile + their own crate's tests
rather than full gates, defer rust-style paydown / reuse-evidence fights, and schedule ONE full
pass (hook, CI, household stories, alpha receipts, habit DELTA flips) after the last closure.
Reds found along the way are journaled as fix-ups, not blockers. Ordinary sprints keep the
per-landing hook. Related: [[feedback_push_branch_discipline]],
[[feedback_atomic_wins_compound_velocity]], [[feedback_shift_drive_cure_not_stage]].
