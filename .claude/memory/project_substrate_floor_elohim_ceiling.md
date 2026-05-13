---
name: Substrate-deterministic floor + elohim-discernment ceiling
description: Architectural pattern — substrate provides deterministic verdicts without AI; elohim layer enriches with discernment but never gates. Apply consistently to compute, reach, recovery, attribution, governance.
type: project
originSessionId: 195ee79b-20ed-438e-8388-af439b3a42a7
---
The protocol has two architectural layers wherever a decision must be made.

**Substrate floor** does the mechanical work: capacity arithmetic, k8s-style scheduling on incoming requests, deterministic execution of standing agreements (cron / threshold / gossip-rule), recording verdicts as Commitment / EconomicEvent records. Always runs. No AI required. Returns deterministic outcomes (Granted / Denied / Pending / Fulfilled / Breached) with explicit reason.

**Elohim ceiling** adds discernment, wisdom, value minting, contextual override. Authors / revises / retires standing agreements. Records as FeedbackSignal entries linked to substrate verdicts. Runs only when household elohim is alive and has context. Enriches, never gates.

**Why:** The substrate must function (slower, simpler, less wise) without elohims. Elohim absence is normal — humans have phones off, households are in transit, agents are still being designed. The protocol cannot be brittle to elohim availability. The substrate IS the protocol's autonomic nervous system; the elohim is its discerning mind. Both are needed at different moments; the autonomic layer must be self-sufficient.

This pattern was already governing reach gates (`project_reach_gate_is_elohim_mediated_matchmaking.md`), recovery flows (`project_socially_derived_security.md`), and attribution recording. Made explicit and consistent during the 2026-05-04 shem outage brainstorm — see `docs/superpowers/specs/2026-05-04-compute-commitment-substrate-floor-design.md`.

**How to apply:** When designing any new protocol surface, name both layers explicitly:

- What does the substrate do *without* elohim? It must be a complete, functional answer.
- What does the elohim add *on top*? Always additive — discernment, wisdom, contextual override with rationale, value minting, pattern matching against history.
- What's the recording shape? Substrate writes the deterministic record; elohim writes parallel FeedbackSignal entries that link to it.
- Anti-pattern: a substrate that requires elohim to function. Anti-pattern: elohim signals that can retroactively erase substrate verdicts (they can write parallel exceptions, but the substrate's record stays as the public rationale).

**Two corollaries that come up repeatedly:**

1. **Allocation vs minting are different jobs.** Allocation is mechanical (capacity arithmetic, scheduling, gating). Minting is contextual (what is this contribution worth in our shared values?). Substrate handles allocation; elohim handles minting. Same separation appears in reach (substrate gates / elohim recommends sponsors), in attribution (substrate records / elohim valuates), in recovery (substrate confirms shares / elohim discerns trust).

2. **Standing agreements run on substrate after elohim authors them.** Once a standing agreement is on DHT, it executes deterministically without further elohim involvement. Elohim's role at standing-agreement time is authoring, revising, retiring — not executing. This is what lets the protocol keep functioning when households are asleep.
