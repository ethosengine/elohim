---
id: feedback-no-legacy-correct-architecture-first
name: feedback-no-legacy-correct-architecture-first
title: "feedback-no-legacy-correct-architecture-first"
description: "Operator 2026-10-04 — do not preserve legacy paths, interim bridges or compatibility arms; replace the wrong design outright"
metadata:
  type: feedback
cites:
  - "claim-backed-credentials-and-contextual-authority-design | §12 of the authorization spec: the ruling that deleted its interim-bridge arm (no compatibility arm) | sha256:c16fffbadbe818e9 | path: genesis/docs/superpowers/specs/2026-09-30-claim-backed-credentials-and-contextual-authority-design.md"
---

Operator (2026-10-04, rejecting a plan that kept an "interim bridge" on the old delegation rule and tests pinning legacy bytes): "I do not care about maintaining anything legacy, I want us to get onto the correct architecture the first time."

**Why:** the networks are pre-compatibility; a kept legacy arm becomes a second authority path that must be maintained and can be exploited.

**How to apply:** when a design replaces a rule, plan the deletion of the old rule, its wire mirrors and its tests, and state the migration consequence plainly (e.g. existing records are re-issued). Do not offer "A now, B later" where A extends the thing being replaced. Scope: the operator said this about the authorization architecture; my reading is that it holds generally while the networks are pre-compatibility. Related: [[feedback-cleanup-toward-p2p-dataplane-trajectory]], [[feedback_atomic_wins_compound_velocity]].
