---
id: "backlog-discovery-attestation-subscale-render-unbuilt"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Discovery assessment: subscale-score render testids + semantic-ID attestation localStorage write are unbuilt (a2o know-thyself-discovery 29/30/31)"
slug: "discovery-attestation-subscale-render-unbuilt"
written: "2026-06-27"
author: "overnight doorway-deploy + genesis fan-out shift (2026-06-27T03, wave-2)"
status: "backlog"
priority: "medium"
jobs: [elohim-genesis]
---

## The gap (spec-ahead-of-implementation, NOT a bug)

a2o `features/lms/know-thyself-discovery.feature` scenarios 29/30/31 fail because
the asserted contract does not exist in shipped code (wave-2 fixer verdict: UNBUILT —
no patch). Three independent pieces are missing; adding one testid would not make them
pass:

1. **Subscale-score render testids.** Steps look for `data-testid="discovery-subscale-score"`
   and `data-testid="discovery-profile"` (`genesis/a2o/.../selectors.ts:548-549`, asserted
   `discovery-assessment.steps.ts:220,260`). Neither exists. The render DOES exist under
   different names: `discovery-quiz.component.ts` uses `class="subscale-breakdown"`/
   `subscale-percent`; `assessment-completion-summary.component.ts:99` uses
   `data-testid="completion-subscales"`. So the UI shows subscales but isn't addressable
   by the asserted contract.
2. **Semantic-ID attestation localStorage write.** Steps read unprefixed key
   `discovery-attestations` expecting milestone IDs (`values-examined`, `attachment-aware`,
   `first-discovery`). Those IDs live only in seed data
   (`genesis/data/lamad/paths/know-thyself-path.json` `attestationGranted`) and are merely
   displayed as a chapter badge. Nothing writes them on completion. The shipped
   `DiscoveryAttestationService` writes a DIFFERENT key (`elohim:discovery-attestations`,
   object-shaped, `attest-discovery-…`); the `first-discovery` milestone concept doesn't
   exist.
3. **No completion wiring.** `discovery-quiz.component.ts` `completeQuiz()` (~line 787)
   only emits a `completed` event — never calls `DiscoveryAttestationService`, never reads
   the step's `attestationGranted`, never touches localStorage. No `discovery-attestation-badge`.

## Proposed build (owner: lamad pillar / angular-architect)

Angular-native (ephemeral display-scoped localStorage + render testids; no Rust/domain
truth): on discovery completion, resolve the active path step's `attestationGranted`, write
the semantic ID (+ `first-discovery` on first completion) to the unprefixed
`discovery-attestations` key; expose `data-testid="discovery-subscale-score"`/`discovery-profile`
on the results render; add `discovery-attestation-badge` to the profile discovery section.
**Caveat:** do NOT fork a second store — reconcile intentionally with the existing
`elohim:`-prefixed `DiscoveryAttestationService` (the scenario contract is a distinct
unprefixed semantic-ID store).

## Evidence / refs

- Wave-2 fixer (t4-discovery-subscale-render) STEP-0 verdict B, empty patch.
- `80c959d8c fix(a2o): repair test-drift vs shipped components` reconciled other steps but
  left these three unbuilt.
- Shift journal: `.claude/shifts/2026-06-27T03-overnight-doorway-deploy-genesis-fanout.journal.md` (iter-5).
