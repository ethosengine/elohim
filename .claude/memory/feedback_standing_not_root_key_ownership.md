---
id: feedback-standing-not-root-key-ownership
name: feedback-standing-not-root-key-ownership
title: "feedback-standing-not-root-key-ownership"
description: "Operator 2026-10-04 — authority over content comes from governance-recognized standing, never from the key that authored the root; keep provenance, recognition, standing and reach as separate planes"
metadata:
  type: feedback
cites:
  - "claim-backed-credentials-and-contextual-authority-design | §12 Authority by standing: the provenance/recognition/standing/reach plane ruling this memory relays | sha256:c16fffbadbe818e9 | path: genesis/docs/superpowers/specs/2026-09-30-claim-backed-credentials-and-contextual-authority-design.md"
---

Operator (2026-10-04, rejecting a plan that kept "only the root author's key may grant or accept a head"): "this violates the stewardship not ownership paradigm... the network itself must own the root, so anyone through the governance process can author to the root if the network recognizes the authority." And: "there are at least two planes here that are getting intermingled in legacy authorization architecture."

The planes the operator confirmed (my naming, their selection): **provenance** (which device produced the bytes — the device's cryptographic root, like brit's key; evidence only), **recognition** (whose device that is), **standing** (who may change a subject; held by people and collectives, granted through governance), **reach** (who may see). They said "I'm not sure all of them", so treat the list as open.

**Why:** a root-author check fuses provenance, a key and authority in one comparison; it is ownership by key. I designed from it twice in one session (per-item delegation, then "roots authored by Matthew's keys") before being corrected.

**How to apply:** never derive authority over content from who authored a root or from a device binding alone. Ask which plane a check belongs to before writing it; a check that reads two planes at once is the bug. Authoring a root makes someone at most the first steward. Related: [[feedback_framing_and_projection_guards]], [[project_collective_topology_author_stewards]], [[project_earned_reach_governance_pr_ceremony_vision]].
