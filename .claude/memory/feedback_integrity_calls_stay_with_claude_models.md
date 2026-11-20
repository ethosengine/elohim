---
name: feedback-integrity-calls-stay-with-claude-models
title: "Integrity calls stay with Claude models"
description: "Operator 2026-10-29: Codex/other families add diversity, but an integrity-class design contention is ruled by a Claude model."
metadata:
  node_type: memory
  title: Integrity calls stay with Claude models
  type: feedback
  originSessionId: 5365acd4-e50b-4288-8f26-536d931715d7
  modified: 2026-10-29T16:43:18.394Z
---

Model diversity is wanted: Codex (via Haiku wrappers), Opus and Sonnet can all draft, implement
and review, and cross-family review catches what one family misses. But when a design contention
touches **integrity** — authority, signing and key handling, attestation and witness semantics,
standing/consent, what counts as evidence, anything that could launder an unverified claim into a
trusted one — the deciding call is made by a Claude model, and a Codex-authored change on such a
surface gets a Claude review before it lands.

**Why:** operator, 2026-10-29: "I trust the value alignment of claude models a bit more than
codex.. so keep that in mind if there's any point of contension that arises in the design that
might be an integrity concern." Stated as a trust judgment about values, not capability.

**How to apply:**
- In judge panels, a split where one side is Codex and the issue is integrity-class: the session
  model rules, and says in the record which side it took and why.
- Codex write legs on integrity surfaces (key placement, signature verification, roster/acceptance,
  attestation writers, lease/CAS fences) always get a Claude-family adversarial review.
- Mechanical or throughput work needs no such tie-break; diversity applies freely there.

Related: [[feedback-haiku-wraps-codex-in-workflows]], [[feedback_controller_holds_vision_opus_implements]], [[feedback_sealed_decisions_must_not_outrun_evidence]].
