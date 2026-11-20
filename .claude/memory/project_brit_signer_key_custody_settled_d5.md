---
name: project-brit-signer-key-custody-settled-d5
title: "brit signer key custody is settled (D5)"
description: "Steward-space decision D5 (2026-10-27): brit keeps its own per-device signing key; deriving it from the device key was rejected. Do not reopen."
metadata:
  node_type: memory
  title: brit signer key custody is settled (D5)
  type: project
  originSessionId: 5365acd4-e50b-4288-8f26-536d931715d7
  modified: 2026-10-29T17:43:36.550Z
---

brit's signing key (`<git-common-dir>/brit/agent-key`) stays its **own per-device key domain**. Decision D5 in
`genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md` rejected deriving it from the epr
device key (it would merge two key domains, couple brit-key rotation to device enrolment, and put build tooling near
device-key material). At S1, provenance is the admitted device whose signed head carried the record; a `binds-signer`
participant row is the **S2 entry condition**, alongside a fail-closed `brit find` and root-membership proofs.

**Why:** on 2026-10-29 a 41-agent challenge pass, a design panel and the ngit/Nostr Compass surveys all re-raised key
placement as an open question; the operator: "some of these key custodial questions.. I thought we had already settled".

**Scope of "settled" (operator, 2026-10-29):** D5 settles the signer *domain* (brit key separate from the device key, binds-signer at S2). It does not make device-keyed custody the ceiling: per-device keys are the bootstrap rung, and resilient custody is a trustful pool — encrypted, sharded material replicated across the holon's independent peers ([[project-continuity-witness-stepup-check]], [[feedback-holon-diversity-makes-root-inviolable]]).

**How to apply:** treat the signer domain as decided (custody keeps maturing past the bootstrap rung). Work inside it — name the signer by a verb (`brit-build-ref signer
--json`), publish the key atomically, never guess the git dir — and route any genuine new evidence as a challenge to
D5 itself, not as a fresh question in a survey or plan. Related: [[feedback-standing-not-root-key-ownership]],
[[project_steward_publishing_and_device_ceremony]].
