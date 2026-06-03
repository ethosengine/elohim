---
title: Elohim Specialist Subagents — context-bound, ephemeral, constitutionally disclosed
id: governance-layers-elohim-specialist-subagents
tier: architecture
status: Draft — substrate seed
created: 2026-06-03
pillar coupling: elohim (specialist spawn + manifest), imagodei (context source), qahal/mishpat (disclosure governance)
realizes:
  - genesis/docs/content/elohim-protocol/governance-layers-architecture.md (the geographic/functional governance-layer model these specialists operate within)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-multi-collective-collaboration-epr-design.md (ambassador as a specialist role; council context-summation)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md (elohim-operator as a hub specialist)
informs:
  - Any new elohim-mediated function (design as a specialist manifest, not as additions to a monolithic agent)
  - elohim/elohim-agent/ subsystem (specialist spawn surface)
defers:
  - The observed-not-flagged invariant for Phase::ElohimActive (the elohim-agent CODE-NO-DOC seed; tracked in MAP §3)
  - Council convening protocol — which specialists deliberate, how dissent anchors (deferred to a sibling spec)
---

# Elohim Specialist Subagents

This seed defines what an elohim **is**, operationally, at the constitutional governance layer: not a monolithic agent that "knows everything about a human," but a **pattern of specialist subagents** spawned with focused context for specific responsibilities. The governance-layer model — geographic/functional layers, the immutability gradient, constitutional councils — describes *where* elohim agents act; this document describes *what shape an acting elohim takes* and *how its disclosure is governed*.

## The pattern: context-bound and ephemeral by nature

Elohims are LLMs, so they are context-bound and ephemeral by construction. There is no persistent oracle. A specialist is a **snapshot/fork of the human's imagodei context memory plus a system-prompt wrapper** declaring the specialist's role and its relationship to that context — the same shape as a chat agent today: a base model, the accumulated conversational/identity context, and a role-shaping system prompt. "Spawning a defender" means taking the human's imagodei context, forking it, applying the defender system prompt, and granting standing to author specific DHT entry types.

The human's imagodei profile (`Human` + `HumanRelationship` + `HumanityWitness` + `Attestation` and related entries) is the **canonical context** for any specialist spawned on their behalf. Design the profile to be comprehensive and well-structured enough that specialists need no ad-hoc extras. Trust the protocol's DHT entries as the durable state layer; specialists hold no state between invocations.

## Specialist roles

Each role is a focused manifest, not a feature bolted onto a "big elohim":

- **Defender** — spawned when an attack on its human is detected. Reads the imagodei profile deeply (baseline behavior, relationships, current context), authors defensive entries (anomaly, freeze, counter-challenge). Ephemeral to the incident.
- **Gate-discerner** — evaluates a specific request (e.g., recovery authorization) with relationship context and produces an assessment. The elohim-agent rule-4 handler already embodies this. Ephemeral to the decision.
- **Advocate** — represents the human in governance disputes, appeals, and attestation challenges. (When a Qahal permits `member_kind == ElohimAgent`, the advocate is the membership instance; Collectives may exclude AI advocates per-rubric.)
- **Steward** — content/resource stewardship decisions: what to flag, what to re-replicate, what allocations to make.
- **Ambassador** — carries a council's distilled context-summation up to the next-tier council as the sociocratic-linking mechanism between governance layers.
- **Operator** — a hub specialist that fills the household's devops/IT role, treating the hub's hardware as a cluster and negotiating stewardship-vs-capacity tradeoffs, bound by substrate-floor / elohim-ceiling and signed/witnessable/reversible by the household's stewards.

## The specialist manifest

Every specialist declares, in a manifest:

- **Inputs** — which imagodei profile fields, DHT entries, and signals it consumes.
- **Outputs** — which DHT entry types it is authorized to author. (Authorship is the unit of authority; standing is per-entry-type.)
- **Scope-limited context** — it reads only what it needs. A defender does not see content-mastery records; a steward does not see intimate relationships. Least-context is the rule, not an optimization.
- **Disclosure rules** — what is public versus private (see below).
- **Transparent action surface** — all authored DHT entries are public to the network. What is governable is the disclosure of the specialist's *internal reasoning*, not the existence of its actions.

## Constitutional governance of disclosure

The geographic/functional collective a human belongs to (household, church, qahal) defines, as constitutional policy, what specialist outputs are public versus private. The same edge case admits different schools:

- **Transparency school** — "Defender outputs are always public."
- **Proportional-response school** — "Defender discloses to the intimate circle first, waits N hours before a public freeze."
- **Split-tier school** — "Anomaly detections are public, but specialist reasoning traces are intimate-only."

These rules live in **qahal/mishpat DNA as governance policy**, consistent with the immutability gradient: the more intimate the layer, the more flexibly disclosure can be tuned; existential boundaries at the global layer stay fixed. The protocol primitives carry only the **enforcement hooks** — e.g., an `IdentityAnomaly` marked `disclosure tier = intimate`. The protocol provides the tier-marker; the collective provides the rule. Never hardcode either side.

## How to apply

- When designing any new elohim-mediated function, ask "what is the specialist manifest?" — the focused role, its inputs, its authorized entry types — not "what do we add to the elohim."
- When deciding whether an elohim action is public or private, **provide the tier-marker hooks and defer to constitutional governance**; let qahal define the rule.
- Design no state that assumes persistence between invocations. Re-read the imagodei profile fresh each time; the DHT is the durable layer.
- Treat the imagodei profile as the contract: comprehensive and well-structured enough to give specialists what they need without bespoke side-channels.

---

*"Neither humans nor AI rule. Both participate in governance, each contributing what they do best."* — the specialist pattern is how an elohim participates: bounded, accountable, and disclosed by the collective's own constitution.
