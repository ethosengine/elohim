# Mastery / Content-Attestation mechanism — current-state map + gap (2026-06-20)

Produced by a 4-reader understand pass while scoping the "migrate the ContentAttestationView" deliverable.
Reframes the work: the operator's vision (mastery attestations earned via quizzes, gating roles) is largely
**declared scaffolding, not wired**. The trust-badge frontend we set out to "migrate" is a *different* thing
(content-quality), and neither it nor mastery has a wired minter.

## The subject mismatch (spine of the decision)
The lamad manifest is unambiguous (`elohim/sdk/domains/lamad/manifest/attestations.json`):
- `attestation:mastery` → `subject_kinds: ["agent"]` — a credential ON a person.
- `attestation:content-quality` → `subject_kinds: ["content"]` — a quality claim ON content.

The trust-badge is keyed by **content CID** → it is the **content-quality** read. **Mastery-on-an-agent is a
separate, unbuilt display.** "Reviving the trust-badge" ≠ reviving mastery.

## Current state — mint → gate → display
- **Mint (quiz → `attestation:mastery`): NOT WIRED.** Passing a lamad quiz emits an EconomicEvent with
  `resourceConformsTo:"mastery-attestation"` (a label, `signal-harness.service.ts:49`) — never an attestation.
  The DNA primitive `issue_attestation` exists (`content_store/src/attestation.rs:44`) but the quiz/mastery
  path never calls it. Manifest says mastery is minted "when policy fires" — **that policy does not exist.**
- **Mint (`attestation:content-quality`): ALSO NOT WIRED.** No producer mints it; the legacy write route
  `POST /api/v1/attestations` was removed in Phase-2a. The `attestations` table currently only receives
  `attestation:gate-decision` + governance votes (via the projector). **So repointing the badge READ yields
  empty badges from a correct endpoint — necessary but not sufficient for visible badges.**
- **Gate (mastery → content access): WIRED on `content_mastery`, NOT on attestations.** The rebuilt prereq
  gate (`epr_service.rs:619 check_prerequisite_mastery`, all 3 transports) reads the private `content_mastery`
  table, denying when `mastery_level=="not_started"`. It does NOT read `attestation:mastery` (and can't —
  `content_mastery` is keyed `(human_id, content_id)`, an attestation has one `subject_cid`). Threshold is
  looser than "recall": any engagement clears it (`ATTESTATION_GATE_LEVEL=4` exists but only feeds stats).
- **Gate (mastery → governance/role): NOT WIRED (aspirational).** No qahal/governance path consults mastery
  or any attestation as a precondition; governance is open. `attestation:governance-role` is declared, created
  + consumed by nothing. `/api/v1/mastery/check-privilege` is report-only — gates nothing.
- **Display (trust-badge): WIRED but ZOMBIE.** Two byte-identical stacks; only the **lamad** copy renders
  (`content-viewer.component.ts:587`). Both chain to the REMOVED `GET /api/v1/attestations?contentId=` →
  404 → `catchError(()=>of([]))` → silent "unverified" fallback.

## The correct read surface (already live, the migration target)
`GET /api/v1/attestations/unified?subjectCid=<cid>&kind=<k>` is wired + tested (`handle_unified` →
`db::attestations::list_by_subject(subject_cid, kind_filter)`). A correct unused client already exists:
`AttestationApiService.listBySubject(subjectCid, kind?)` typed to unified `AttestationView`. **No backend
change needed for the content-quality read.** Adapter caveat: legacy `ContentAttestationView` fields
(`contentId/attestationType/isRevoked`) → unified shape (`attestationType` ← `evidenceJson.quality_dimension`);
verify content-node `id` (a slug) vs `subjectCid` keying.

## The two scopes
1. **Read-display migration (small, safe, EMPTY until a minter exists):** repoint the lamad trust-badge onto
   `/unified?subjectCid=&kind=attestation:content-quality`, retire the dead elohim-app twin + the legacy
   `ContentAttestationView`/`ContentAttestationApiService`. Completes the consolidation's frontend side;
   correct plumbing; badges stay empty (no content-quality minter).
2. **The mastery-credential epic (the operator's actual vision, mostly unbuilt):** the `attestation:mastery`
   minter (quiz/ContentMastery-policy → `issue_attestation`), aligning the prereq gate to the *public*
   `attestation:mastery` credential (or deciding it stays on private `content_mastery`), the
   mastery→governance/role gating, and a mastery (agent-keyed) display surface distinct from the content-quality
   badge. This is a multi-DNA + storage + frontend epic, not a dev-deliverable bite.
