---
name: Bootstrap social security upgrades to elohim-integrated security
description: Protocol security matures along a gradient — structural/social rules are sufficient at bootstrap; elohim-integrated checks layer in as elohim become embedded in the network. Do not force full elohim-enforcement at zome-validation time when coordinator-trust + defender-escalation is adequate for the current network stage.
type: project
originSessionId: 5ba0c4a3-96ec-40af-913d-cb7ebf8d7a3c
---
Security in the Elohim Protocol follows a staged gradient, not a single-shot cryptographic gate:

**Stage 1 — structural / social bootstrapping.** Validators enforce what they can cheaply verify: shape, signatures, counts, consistency. Correctness obligations that require deep social-graph traversal (eligibility, relationship verification, behavioral baselines) live in the coordinator layer. Normal social rules carry the trust weight — human witnesses signing their own attestations is enough.

**Stage 2 — elohim integration.** As elohim become embedded as first-class participants (elohim-as-counsel, defender specialists, agent-attested anomaly detection), defensive entries from elohim tighten the floor. A malicious coordinator becomes observable; defender escalation happens at machine speed; floor rises automatically.

**Stage 3 — fully elohim-enforced.** Eventually validators can reference elohim-signed attestations as first-class integrity evidence. The structural rules remain as the floor; elohim layers add richness, not replacement.

**Why:** Forcing Stage 3 rigor at Stage 1 (e.g., requiring full social-graph traversal in a zome validator) produces brittle performance-sensitive code, weakens the DHT validator's atomicity guarantees, and centralizes correctness logic in a place that's hard to evolve. Allowing coordinator-trust with defender-escalation keeps the bootstrap path open while the richer mechanisms mature.

**How to apply:**
- When a validator rule is expensive or requires cross-entry graph traversal, ask: can the coordinator own this correctness obligation + can an elohim defender observe and escalate if it's violated? If yes, structural-only validation is sufficient for the current stage. Document the tradeoff explicitly in the spec.
- Security decisions should name their stage. "This is Stage 1 — we'll tighten in M5 when defender specialists land" is a legitimate and preferred framing over "this is good enough forever."
- Reject the temptation to pre-build Stage 3 enforcement before the elohim infrastructure is ready to support it — it's premature optimization masquerading as rigor.
- Recovery Protocol Phase 2 is a canonical instance: M2 validator is structural/social; M3 coordinator tightens; M5 elohim defender adds the Stage 2 layer.
