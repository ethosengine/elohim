---
title: Substrate Floor / Elohim Ceiling — the two-layer decision architecture
id: compute-commitment-substrate-floor-design
tier: architecture
status: Architecture pattern (governs every protocol surface where a decision is made)
created: 2026-05-04
pillar coupling: elohim (substrate floor), elohim-agent (discernment ceiling)
informed-by:
  - genesis/docs/architecture/rea-compute-commitment-primitive.md (the Commitment shape the floor executes)
informs:
  - All future protocol surfaces that gate, allocate, or record a decision (compute, reach, recovery, attribution, governance)
  - The elohim-agent's role boundary (enriches, never gates)
memory_anchors:
  - project_substrate_floor_elohim_ceiling
  - project_reach_gate_is_elohim_mediated_matchmaking
  - project_socially_derived_security
---

# Substrate Floor / Elohim Ceiling

Wherever the protocol must make a decision, it makes it in **two layers**. This is a
single architectural pattern, applied consistently across compute allocation, reach
gating, recovery, attribution, and governance. Name both layers explicitly when
designing any new surface; a surface that conflates them, or that requires the ceiling
to function, is a defect.

## §1 — The two layers

**Substrate floor** does the mechanical work. Capacity arithmetic; k8s-style scheduling
of incoming requests; deterministic execution of standing agreements (cron / threshold /
gossip-rule); recording verdicts as `Commitment` / `EconomicEvent` records. It **always
runs, requires no AI**, and returns a deterministic outcome — `Granted` / `Denied` /
`Pending` / `Fulfilled` / `Breached` — with an explicit reason. This is the protocol's
autonomic nervous system.

**Elohim ceiling** adds discernment, wisdom, value minting, and contextual override. It
authors, revises, and retires the standing agreements the floor then executes. It records
its contributions as `FeedbackSignal` entries that **link to** the substrate's verdict. It
runs only when a household's elohim is alive and has context. It **enriches; it never
gates**. This is the protocol's discerning mind.

## §2 — Why the floor must stand alone

Elohim absence is normal, not exceptional — phones are off, households are in transit,
agents are still being designed. The protocol cannot be brittle to elohim availability.
So the floor must be a **complete, functional answer on its own** (slower, simpler, less
wise), and the ceiling must be strictly additive.

This yields two hard rules:

- **Anti-pattern:** a substrate path that *requires* an elohim to produce an outcome.
- **Anti-pattern:** an elohim signal that can *retroactively erase* a substrate verdict.
  The ceiling may write parallel exceptions, but the floor's record stays as the public
  rationale.

## §3 — Allocation vs minting (the recurring split)

The two layers cleave along a recurring seam: **allocation is mechanical; minting is
contextual.** Allocation — capacity arithmetic, scheduling, gating — belongs to the floor.
Minting — *what is this contribution worth in our shared values?* — belongs to the ceiling.
The same seam appears at every surface:

| Surface | Floor (allocation) | Ceiling (minting / discernment) |
| --- | --- | --- |
| Compute | capacity arithmetic, scheduling, gating | values it as a contribution |
| Reach | gates `{Allowed, Blocked, Pending}` | recommends sponsors, adds context |
| Attribution | records the event | valuates the contribution |
| Recovery | confirms the cryptographic shares | discerns whether to trust |

## §4 — Standing agreements run on the floor

Once a standing agreement (a `delegates-compute` Commitment and its kin) lands on the DHT,
it **executes deterministically** with no further elohim involvement. The elohim's role is
at *authoring* time — author, revise, retire — never at *execution* time. This is precisely
what lets the protocol keep functioning while households sleep: the ceiling sets policy
while present; the floor enforces it always.

## §5 — Applying the pattern

When designing any new protocol surface, answer four questions before writing the route:

1. **What does the substrate do *without* an elohim?** It must be a complete, functional
   answer.
2. **What does the elohim add *on top*?** Always additive — discernment, contextual
   override with rationale, value minting, pattern-matching against history.
3. **What is the recording shape?** The floor writes the deterministic record; the elohim
   writes parallel `FeedbackSignal` entries that link to it.
4. **Does the floor still work if the elohim never shows up?** If not, the design has
   leaked discernment into the autonomic layer — move it back to the ceiling.

Without this separation, "stewardship" is rhetoric and the protocol is brittle to the
absence of its own minds. With it, the substrate is the protocol — and the elohim makes
it wise.
