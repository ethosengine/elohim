---
name: signal_kind is the protocol's extensible feedback vocabulary
description: FeedbackSignal.signal_kind is a class-not-a-boolean — adding new variants (vouch, sponsor, acknowledge, witness) is the cheap path to extending the protocol's social vocabulary. Manifest declares debit weights; integrity validator whitelists.
type: project
originSessionId: 42abe5eb-4a48-4a2a-8142-604a4c7a1bd3
---
`FeedbackSignal.signal_kind` started as an enum of {squelch, correction, retraction, quarantine} but its real purpose is to be the **protocol's extensible vocabulary of feedback moves**. Adding a new variant is cheap:

- Schema: add to enum
- Integrity validator: add to whitelist (one DNA touch per batch of additions)
- ManifestDebitWeightPolicy: add to bootstrap manifest's `debitWeights` block (negative weights = recovery, positive = harm, zero = advisory-only)
- Standing projector: zero changes (consumes via DebitWeightPolicy trait)
- HTTP layer: zero changes (rides existing FeedbackSignal EPR ingest)

**Vouch is the first new variant after the bootstrap four** — adding `vouch` (with `vouch_kind: AcceptCorrection|Restitution` sub-field) demonstrates the pattern. Future variants likely include:
- `sponsor` — endorse a new voice's standing baseline
- `acknowledge` — receive-side confirmation of harm received (no judgment)
- `witness` — third-party attestation of an event without taking position
- `appeal` — request mishpat review of a prior signal

**Why this matters for design:**
- Don't bake "the four kinds" into Rust types as separate variants. Use a single `SignalKind` enum that's parsed from the manifest-declared whitelist, not hard-coded match arms.
- New social moves arrive through manifest + validator + bootstrap-policy, not through new entry types or HTTP routes.
- Phase 4 GraphQL surface should consume signals abstractly via DebitWeightPolicy, not enumerate kinds.

**How to apply:**
- When designing a new social/relational primitive, default to "this is a signal_kind variant" before considering a new entry type.
- DNA capacity preserved — Lamad at ~73/~100 stays high because the protocol's social vocabulary grows on FeedbackSignal, not on new entries.
