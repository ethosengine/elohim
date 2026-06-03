---
title: "History/ADR: Experience-Story Discernment Gate — relocated-then-superseded TS-gate stub"
id: experience-story-discernment-gate
type: history-gotcha
status: Accepted
tier: history
created: 2026-06-02
topic: [experience-story, discernment-gate, elohim-agent, seven-valence, attestation]
# DISTILLS a plan stub whose TS-gate-first approach was reverted; discernment landed as a
# Rust Gate primitive in elohim-agent (Phase 3). Landing is structural-verified (files +
# tests exist); CI-green is NOT asserted. Raw stub retires to git; rakia submodule untouched.
distills:
  - genesis/docs/superpowers/plans/2026-04-18-experience-story-discernment-gate.md
canonical:
  - ../architecture/2026-04-18-experience-story-epr-design.md   # authoritative data-model + seven-valence reference
memory_anchors:
  - project_elohim_agent_sense_respond_architecture
  - project_elohim_subagent_specialists
  - project_reach_gate_is_elohim_mediated_matchmaking
  - project_signal_kind_extensible_protocol_class
---

# Experience-Story Discernment Gate — relocated-then-superseded plan stub (2026-04-18 → landed)

> **One-sentence lesson:** Do NOT re-draft this as a TypeScript gate — that path was tried and reverted.
> Gates are Rust primitives in `@elohim/elohim-agent`; rulesets are manifest-declared; the `.ts` surface
> is sense-and-respond only.

**What was attempted.** A plan to ship the v1 mechanical "discernment gate" for experience-story EPRs:
the seam that reads a2o pipeline moments (one persona × one scenario × one run) and mints discerned
story-point attestations carrying a seven-valence value function
(`progress`/`discovery`/`regression`/`validation`/`witness`/`refinement`/`confirmation`) rather than
signed numeric story-points. The first plan proposed the gate as a pure TypeScript function in
elohim-library.

**The turn (why).** During Batch F the architecture was course-corrected: discernment is a first-class
protocol primitive of `@elohim/elohim-agent` (a Rust `Gate` trait invoked by the SDK), NOT an app-layer
`.ts` concern. The `.ts` surface is legitimately only the sense-and-respond layer; the gate's
evaluation/registry/constitutional-reasoning coupling lives in Rust, with the specific ruleset declared
in the app manifest. The TS module (`80fe6c70`..`0ecef2ec`) was reverted (`dfadce0b`); the schemas,
contentType registrations, `experience-attestation` signal, and regenerated manifest types survived. The
genesis-side plan was first *relocated* to `rakia/docs/plans/`, leaving an 18-line cross-ref stub in
genesis — then the rakia body itself was marked SUPERSEDED and folded into the elohim-agent
gate-interface plan, shipping as that plan's Phase 3.

**Where it landed (living surface — structural-verified, CI-green HELD).** Seven-valence gate is real,
mechanical-from-day-one Rust: `elohim/elohim-agent/gate-client/src/dag/` (discernment_gate.rs,
seven_valence_rules.rs, executors/mechanical_ruleset.rs, attestation.rs, reach_aggregation.rs) driven by
a CID-addressed rules artifact `src/dag/rules/seven_valence_v1.json`, with a 14-case integration suite.
App vocabulary in lamad manifest content-types (`experience-story`/`experience-moment`) +
`experience-attestation` signal; avodah components consume experience-stories. Authoritative data-model
remains in the still-live `architecture/2026-04-18-experience-story-epr-design.md`. *Landing is
confirmed by files + tests existing; in-cluster/CI-green is not asserted in this record.*

**Watch-out for future planners.**
1. Do NOT re-draft this as a TypeScript gate — that path was tried and reverted; gates are Rust
   primitives in elohim-agent, rulesets are manifest-declared, `.ts` is sense-and-respond only.
2. Rule ordering is load-bearing: rule 3 (`@validates-failure-mode` → validation) must NOT shadow rule
   2 — a failure-mode scenario with a prior-passed attestation is deliberately classified by rule 2 as
   discovery/regression.
3. Steady-state (rule 7) mints nothing — silence is the baseline signal; the v1 ruleset never mints
   `confirmation`.
4. Sub-projects A (matthew peer persistence), C (doorway moment export/re-upload), D (inter-pipeline diff
   → shefa valueflow) and sophisticated discernment were explicitly deferred; the `attest()` output left
   room for an optional `linkedComputeContributionHashes` field so the compute↔evidence loop can close
   later without a schema break.

## Bidirectional links

- **This record → canonical:** [experience-story EPR design](../architecture/2026-04-18-experience-story-epr-design.md) (the still-live authoritative data-model + seven-valence reference).
- **Distilled-from (raw stub in git history):** the experience-story-discernment-gate plan stub (linked in frontmatter). The rakia submodule plan is out of this tree's scope — do not touch.
