---
id: "backlog-resilience-tier-content-declared-floor"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Content-declared resilience FLOOR — value-tier as an axis orthogonal to reach; self-aware content signals the durability it requires (NEEDS /brainstorm)"
slug: "resilience-tier-content-declared-floor"
written: "2026-06-14"
author: "agentic-developer (operator architectural note during the grandma felt-status shift)"
status: "backlog"
priority: "high"
ci_status: blocked
tags: [architecture, resilience, durability, reach, content-schema, rea-commitment, p2p-design-gate, felt-resilience, brainstorm, anti-capture]
cites:
  - elohim/elohim-storage/src/services/epr_nav_context_view.rs
  - elohim/sdk/schemas/v1/views/epr-nav-context-view.schema.json
  - elohim/elohim-storage/src/services/household_resilience.rs
  - elohim/sdk/schemas/v1/views/resilience-snapshot-view.schema.json
  - genesis/docs/superpowers/specs/2026-05-29-durability-topology-felt-resilience.md
  - genesis/docs/superpowers/plans/2026-06-14-vision-gap-grandma-vertical-stub.md
relatedNodeIds:
  - backlog-reach-vocabulary-frontend-strand
---

# Content-declared resilience FLOOR (value-tier ⟂ reach)

> Operator architectural note (2026-06-14, during the grandma felt-status shift):
> *"content needs to express its floor for its resilience that it will signal its
> resolution for… my mortgage, my will, my wedding photos, my bridge data that
> represent my fiat bank statements, medical records … will express a higher
> resilience tier than say a map that populates a dropdown form (which are like
> ephemeral cache tier, with a reference in commons archive somewhere). What
> something is should also have some self awareness as to where it thinks it
> should be."*

## The principle

Resilience is **content-relative**. "Protected" is not an absolute — it is
*protected relative to this content's declared floor*. Content carries a
**self-declared resilience tier** (a durability floor) expressing how irreplaceable
it is and therefore how durably the network must hold it:

| Tier (illustrative) | Examples | Floor posture |
|---|---|---|
| **vault / sovereign** | will, mortgage, medical records, bridged fiat bank statements | many replicas, diverse holders (cross-household + cross-region), strict K-of-N, long retention — must survive loss of any single steward/region |
| **keepsake** | wedding photos, family memories | high replication, household-diverse, durable |
| **standard** (default) | ordinary personal content | moderate (today's `≥3 households` floor) |
| **ephemeral / cache** | a map that populates a dropdown form; derived/regenerable data | near-zero personal durability; holds **a reference into a commons archive**, re-fetch on loss |

The achieved-resilience computation must measure against the content's **own**
floor or it lies in **both** directions:
- **False alarm:** calling an ephemeral dropdown-map "at-risk" because it has 1 holder (it only needs a commons reference).
- **False reassurance:** calling an under-protected will "protected" because it cleared the flat `≥3 households` bar when its vault floor wants 5+ diverse holders.

This is the natural completion of the felt-resilience honesty discipline
(`durability-topology-felt-resilience.md`): not-yet-seen (no measurement) **+**
floor-relative (measured against the *right* denominator).

## The drift this corrects (live in the code today)

`epr_nav_context_view.rs:31 reach_to_resilience_tier()` derives a resilience tier
**from reach breadth**:

```
commons | public          → "high"
community | familiar       → "medium"
trusted | intimate | self  → "low"
private                   → "unknown"
```

This is **backwards for the personal-vault case** and conflates two orthogonal
axes. A private will is the *most* irreplaceable yet maps to `"low"`; a commons
dropdown-map maps to `"high"`. **Reach (who may see it) ≠ resilience tier (how
durably the owner needs it held).**

| Axis | Question | Source | Example: my will |
|---|---|---|---|
| **Reach** | who may see it | already a content property (`reach` enum) | `private` / `intimate` |
| **Resilience tier** | how durably must it survive | **MISSING — this item** | `vault` |

(These can correlate but must not be derived from each other. Sibling drift:
`reach-vocabulary-frontend-strand.md` + the 3-vocabulary reach reconciliation,
storage CLAUDE.md "roadmap item 13".)

## Slots that already exist (compose, don't re-invent)

- `PlacementGapView.requested_steward_count` vs `achieved_steward_count` +
  `contract_coverage` — the floor-vs-achieved is **already modeled per shard**.
  What's missing is the **content-declared tier that SETS `requested_steward_count`**
  per value-class (today it comes from a flat default at `p2p/mod.rs:1551`).
- `household_resilience.rs:191` — `let desired = 7; // Per-content override
  deferred to Plan 3.` The per-content override **is** this item; "Plan 3" is its
  forward reference.
- `epr-nav-context-view.schema.json` `resilienceTier` (high/medium/low/unknown) —
  the existing tier vocabulary; reconcile/replace with the value-declared tier.
- The grandma felt-status shift (2026-06-14) lands a **floor-aware seam**:
  `FeltStatusView.floor { tier, tierDeclared, wantsHouseholds, hasHouseholds }`
  with the tier→floor mapping built + unit-tested and `tier` defaulting to
  `"standard"` / `tierDeclared:false` until this primitive lights. **This item is
  what flips `tierDeclared` to true and sets the real floor.**

## p2p-design-gate (pre-answered, to confirm in /brainstorm)

- **Declared tier = Cat-A author truth** — part of what the content IS; the owner
  asserts it. Candidate homes: (a) a notarized field on the content/EPR atom;
  (b) a `requires-resilience` instance of the `Mishpat::Commitment` family
  (consistent with the gospel REA-compute-commitment primitive — the author
  *commits* the content to a tier; the placement engine + felt surface read it).
  (b) is anti-capture-clean: the **household** declares what matters, not a
  central SLA-setter.
- **Achieved resilience = Cat-C projection** — already what `household_resilience.rs`
  computes.
- No new identity; keyed by existing `contentId`.

## Open design questions (the /brainstorm gate)

1. **Where does the tier live?** Notarized content field vs `requires-resilience`
   REA commitment vs both (declaration as commitment, cached on the atom).
2. **What is the tier vocabulary?** (`vault/keepsake/standard/ephemeral`?) and how
   does it reconcile the existing `high/medium/low/unknown` reach-derived one.
3. **Self-awareness — who/what sets the default?** Author-declared (explicit) vs
   content-type default (a `will` content-type carries vault; a `dropdown-data`
   type carries ephemeral) vs introspective. The operator's "self awareness"
   framing wants the *content/type* to know its class, overridable by the owner —
   NOT an external operator decision, NOT a reach derivation.
4. **How does the floor drive placement?** tier → `requested_steward_count` +
   diversity requirements (cross-household, cross-region) → the placement engine
   honors it; gaps fire against the declared floor.
5. **Ephemeral semantics.** "a reference in commons archive somewhere" — does an
   ephemeral tier mean *don't personally replicate, keep a commons pointer*? That
   is a distinct holding posture (pointer-not-payload), adjacent to the
   pantry-temperature axis (hot/warm/cold = access pattern, **also distinct** from
   value-tier).

## Why it's high-leverage

It is the denominator that makes the whole felt-resilience surface *honest* — and
it is anti-capture by construction (households declare their own floors; no central
authority rates importance). It connects O1 (grandma's photos felt safe — to the
*right* standard), O7 (cybernetic discipline measured against the right target),
O8/O9 (capture-resistance). It is **mostly composition** — the slots
(`requested_steward_count`, `desired` override, `resilienceTier`, the felt floor
seam) already exist; the missing piece is the declared-tier primitive and its
placement wiring.

**Next move:** `/brainstorm` the 5 questions, then a `/shift` to land the declared
tier + placement wiring; the felt surface already speaks `floor` and will light it
automatically.
