---
name: Standing composes from multiple evidence streams
description: Standing is not just FeedbackSignal-derived debits; imagodei profile/psyche instruments + lamad emergent traits feed the same projection. Manifests declare which streams compose.
type: project
originSessionId: 42abe5eb-4a48-4a2a-8142-604a4c7a1bd3
---
Standing as a derived view should compose from MULTIPLE evidence streams, not only FeedbackSignal-debit accumulation. The legitimate on-ramp out of `Standing::Unknown` is:

1. **imagodei surface** — profile, self-attestation via psyche instruments (Sophia discovery/reflection mode), journaling-derived emergent observable traits
2. **lamad surface** — recognition events from learning paths, mastery proofs
3. **FeedbackSignal-derived** — the Phase 3.5 substrate (squelch/correction/retraction/quarantine debits)

A new peer transitions from "we don't know you" → "we can know who you are" *before* attempting community/public reach by doing self-knowledge work. The standing-policy manifest declares which evidence streams compose into the score and how they weight.

**Why:** Without this, the reach gate becomes a sponsor-friendly first-mover-advantage. Ungrudging service (project_ungrudging_service.md) requires that fresh keypairs from disconnected social graphs can earn their voice through psyche/lamad work, not only by knowing someone with standing. Avoids "weaponizable obscurity attack" where attackers withhold sponsorship from outsiders.

**How to apply:**
- Reach-earning gate's `unknownTreatment` policy should be schema-extensible to include an `evidenceSources: []` array; bootstrap ships empty (FeedbackSignal-only); imagodei/lamad bridges fill it later.
- Standing projector's `DebitWeightPolicy` is *one* policy; standing computation should accommodate `EvidenceCompositionPolicy` that multiplexes streams.
- Don't bake "FeedbackSignal is the only standing input" into Rust types. Schema-first: name the inputs, let the manifest declare what counts.
- Forward-compat is what we ship in this sprint; the actual bridge is a future sprint.
