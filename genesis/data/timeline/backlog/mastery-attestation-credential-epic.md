---
id: "backlog-mastery-attestation-credential-epic"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "EPIC seed: wire the mastery-attestation credential mechanism (quiz→attestation:mastery minter → role/content gating) — currently declared scaffolding, not built"
slug: "mastery-attestation-credential-epic"
written: "2026-06-20"
author: "surfaced by the mastery-attestation understand pass while scoping the trust-badge deliverable; the operator's vision is mostly unwired"
status: "open"
priority: "high"
tags: [attestation, mastery, lamad, gating, governance, bloom-taxonomy, epic, p2p-design-gate, dna-migration]
cites:
  - .claude/data/mastery-attestation-current-state-2026-06-20.md
  - elohim/sdk/domains/lamad/manifest/attestations.json
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md
  - elohim/elohim-storage/src/epr_service.rs
---

# Mastery-attestation credential mechanism — EPIC seed

**The operator's vision (authoritative):** attestation is core; lamad's *meaning* for it is manifest-defined.
A **mastery attestation** is an earned credential that **gates roles across EPRs** — pass lamad learning-path
quizzes → earn Khan-Academy **"recall"-level** mastery (Bloom's: can-do / understand-basics / repeat) → that
**opens governance participation, community contribution, conversations on content, and access to more
sensitive/higher-level subjects.** Higher Bloom's tiers (analysis/synthesis) are earned via contribution +
peer-review, not quizzes.

**Current reality (mapped 2026-06-20, see `.claude/data/mastery-attestation-current-state-2026-06-20.md`):**
the vocabulary + primitives exist (`attestation:mastery` declared, DNA `issue_attestation` primitive, the
unified `attestations` surface, `content_mastery`), but the **connecting policy/minters/gates are NOT wired.**
This is the gap between the vision and the code.

## The slices (each a real build, sequence in a /plan after the p2p-design-gate)
1. **The mint policy (quiz → `attestation:mastery`).** A learner passing a lamad quiz currently emits an
   EconomicEvent *label* (`signal-harness.service.ts:49` `resourceConformsTo:"mastery-attestation"`), never an
   attestation. Build the policy that fires `issue_attestation(attestation:mastery)` from ContentMastery when
   the learner reaches the **"recall" threshold** (map recall → a `mastery_level`; `ATTESTATION_GATE_LEVEL=4`
   APPLY exists in `db/models.rs` but feeds only stats). subject=agent.
2. **Gate alignment — public credential vs private progress.** The rebuilt prereq gate
   (`epr_service.rs:619 check_prerequisite_mastery`) reads PRIVATE `content_mastery` at a loose threshold
   (any engagement ≠ not_started). DECISION: align it to the PUBLIC `attestation:mastery` credential at the
   recall bar (the vision), or keep content_mastery as the gate and mint attestation:mastery as the *public
   proof* projection. (Note the keying mismatch: content_mastery is `(human_id, content_id)`; an attestation
   has one `subject_cid`.)
3. **Mastery → governance/community/role gating (NOT wired).** No qahal/governance path consults mastery as a
   precondition; governance is open. `attestation:governance-role` is declared, consumed by nothing. Wire
   recall-level mastery as the precondition that opens governance participation / contribution / conversations.
4. **The mastery DISPLAY (agent-keyed).** Distinct from the content-quality trust-badge (subject=content,
   migrated separately 2026-06-20). A person's earned credentials surface — what roles they've unlocked.

## Design forks for the /plan (genuine human calls)
- Does the prereq gate move to the public `attestation:mastery` credential, or stay on private progress +
  project the public proof? (slice 2)
- Recall = which `mastery_level` value / which Bloom's threshold mints the credential? (slice 1)
- What mints `attestation:content-quality` (peer-review? curator? authoring) — needed for the trust-badge to
  show data (separate from mastery; the badge is plumbed-but-empty as of the 2026-06-20 read-migration).
- DNA-migration safety: minting paths touch the elohim DNA coordinator — operator-gated per the consolidation
  design + root CLAUDE.md DNA gotchas.

**This is a multi-DNA + storage + frontend epic — NOT a quick deliverable.** Take it through `/brainstorm` →
`p2p-design-gate` → `/plan`, composing from the attestation-consolidation design (it already anticipated
`attestation:mastery` as "the public proof" of private ContentMastery). It is the substrate behind the
operator's learning→participation→governance progression.
