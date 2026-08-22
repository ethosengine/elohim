---
name: scope-flag-beats-prose-note
title: Scope flag beats prose note
description: "The `available:` flag + scope-reconcile beat prose/stale memory on whether a cap is down; @requires:<cap> = satisfiable-when-available, not held."
metadata:
  node_type: memory
  type: feedback
---

A stale prose `note: "offline … OUT OF SCOPE … held"` on `shem` in `cluster-state.yaml` contradicted its
`available: true` flag for ~3 weeks (the 2026-06-01 offline note was never cleared when shem returned). The
structured machinery was correct the whole time (`scope-reconcile`: shem available, `held: 0`; the
SessionStart focus baseline listed shem AVAILABLE). But the prose note — plus stale memory — made me (and
several recon subagents) treat shem as down, and I propagated a false **"held until shem"** through an
entire 4-wave design arc (the Doorway Membrane spec + plans).

**Why:** scope-reconcile reconciles STRUCTURED scope (held/ tree, `requires_env`, HELD-CITE via cites) but
NOT prose. A cluster-state `note` and spec/plan prose ("held until X", "design-only until X") are
unreconciled scope — they can drift from the `available` flag with nothing catching it. `@requires:<cap>`
is a *requirement* tag: when `<cap>` is available it means **satisfiable now**, never "blocked."

**How to apply:** when any source says a capability is down/held, confirm against the LIVE signal before
believing it — `cluster-state.yaml` `available:` + `scope-reconcile.py` (held/live counts) + the focus
baseline — in that order of authority; the prose `note` and your own memory are the LEAST authoritative and
the most likely stale. If they disagree, the flag wins, and fix the note. The wished-for automated check is
[[scope-prose-coherence-switch]] (the note↔flag + prose↔live lint). Don't gate cross-node work on shem
reflexively: household-nodes is *local* P2P (~0 RTT); shem is the live cross-node canvas — and DNS/anycast/:53
are operator-INFRA, never shem (a separate category).
