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

**Current reality (mapped 2026-06-20, full map in the appendix below):**
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

---

## Current-state map (2026-06-20)

Produced by a 4-reader understand pass while scoping the "migrate the ContentAttestationView"
deliverable (folded here 2026-07-02 from the session's map). Reframes the work: the operator's
vision (mastery attestations earned via quizzes, gating roles) is largely **declared scaffolding,
not wired**. The trust-badge frontend we set out to "migrate" is a *different* thing
(content-quality), and neither it nor mastery has a wired minter.

### The subject mismatch (spine of the decision)
The lamad manifest is unambiguous (`elohim/sdk/domains/lamad/manifest/attestations.json`):
- `attestation:mastery` → `subject_kinds: ["agent"]` — a credential ON a person.
- `attestation:content-quality` → `subject_kinds: ["content"]` — a quality claim ON content.

The trust-badge is keyed by **content CID** → it is the **content-quality** read. **Mastery-on-an-agent
is a separate, unbuilt display.** "Reviving the trust-badge" ≠ reviving mastery.

### Mint → gate → display
- **Mint (quiz → `attestation:mastery`): NOT WIRED.** Passing a lamad quiz emits an EconomicEvent with
  `resourceConformsTo:"mastery-attestation"` (a label, `signal-harness.service.ts:49`) — never an
  attestation. The DNA primitive `issue_attestation` exists (`content_store/src/attestation.rs:44`) but
  the quiz/mastery path never calls it. Manifest says mastery is minted "when policy fires" — **that
  policy does not exist.**
- **Mint (`attestation:content-quality`): ALSO NOT WIRED.** No producer mints it; the legacy write route
  `POST /api/v1/attestations` was removed in Phase-2a. The `attestations` table currently only receives
  `attestation:gate-decision` + governance votes (via the projector). **So repointing the badge READ
  yields empty badges from a correct endpoint — necessary but not sufficient for visible badges.**
- **Gate (mastery → content access): WIRED on `content_mastery`, NOT on attestations.** The rebuilt
  prereq gate (`epr_service.rs:619 check_prerequisite_mastery`, all 3 transports) reads the private
  `content_mastery` table, denying when `mastery_level=="not_started"`. It does NOT read
  `attestation:mastery` (and can't — `content_mastery` is keyed `(human_id, content_id)`, an attestation
  has one `subject_cid`). Threshold is looser than "recall": any engagement clears it
  (`ATTESTATION_GATE_LEVEL=4` exists but only feeds stats).
- **Gate (mastery → governance/role): NOT WIRED (aspirational).** No qahal/governance path consults
  mastery or any attestation as a precondition; governance is open. `attestation:governance-role` is
  declared, created + consumed by nothing. `/api/v1/mastery/check-privilege` is report-only — gates
  nothing.
- **Display (trust-badge): WIRED but ZOMBIE.** Two byte-identical stacks; only the **lamad** copy
  renders (`content-viewer.component.ts:587`). Both chain to the REMOVED
  `GET /api/v1/attestations?contentId=` → 404 → `catchError(()=>of([]))` → silent "unverified" fallback.

### The correct read surface (already live, the migration target)
`GET /api/v1/attestations/unified?subjectCid=<cid>&kind=<k>` is wired + tested (`handle_unified` →
`db::attestations::list_by_subject(subject_cid, kind_filter)`). A correct unused client already exists:
`AttestationApiService.listBySubject(subjectCid, kind?)` typed to unified `AttestationView`. **No
backend change needed for the content-quality read.** Adapter caveat: legacy `ContentAttestationView`
fields (`contentId/attestationType/isRevoked`) → unified shape (`attestationType` ←
`evidenceJson.quality_dimension`); verify content-node `id` (a slug) vs `subjectCid` keying.

### The two scopes
1. **Read-display migration (small, safe, EMPTY until a minter exists):** repoint the lamad trust-badge
   onto `/unified?subjectCid=&kind=attestation:content-quality`, retire the dead elohim-app twin + the
   legacy `ContentAttestationView`/`ContentAttestationApiService`. Completes the consolidation's
   frontend side; correct plumbing; badges stay empty (no content-quality minter).
2. **The mastery-credential epic (this entry's slices above, mostly unbuilt):** the
   `attestation:mastery` minter, gate alignment, mastery→governance/role gating, and an agent-keyed
   display surface distinct from the content-quality badge.
