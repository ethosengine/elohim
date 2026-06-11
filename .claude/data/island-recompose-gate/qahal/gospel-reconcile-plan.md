# Gospel Reconcile Plan — qahal island recompose (PROPOSED EDITS, not applied)

Disposition group: readme/gospel-reconcile. Operator applies serially through cite tooling
(cites in frontmatter are sealed by tooling — plain paths below, tooling seals).

Line numbers reference working-tree state at branch `feat/frontend-eyes-sprint` (2026-06-11).

---

## TARGET 1 — `app/elohim-app/src/app/qahal/CLAUDE.md` (pillar gospel, id: qahal-pillar-gospel)

### Edit 1.1 — Spec reference → legacy-reference note (spec retiring to git)

**Why:** Line 13 points at `QAHAL_API_SPECIFICATION_v1.0.md`, which is being retired to git by
this recompose (mirrors lamad: commit c8cb7ebe3 retired `app/lamad/docs/` and rewrote the
"Legacy reference notes" row in `app/lamad/src/app/claude.md:43` to point at the history record
`genesis/docs/content/elohim-protocol/history/2026-06-11-lamad-mvp-implementation-arc.md`).

**Why (line 14 too):** `elohim/ELOHIM_PROTOCOL_ARCHITECTURE.md` (resolves to
`app/elohim-app/src/app/elohim/ELOHIM_PROTOCOL_ARCHITECTURE.md`) is being retired by a parallel
session. Architecture truth for this pillar already lives at the cited subject home —
`qahal-domain-gospel` (`elohim/sdk/domains/qahal/CLAUDE.md`, cited in this gospel's frontmatter
line 4 and body line 21) — and the gospel-tier vision spec
`genesis/docs/superpowers/specs/2026-05-21-qahal-architecture-vision.md` (verified to exist;
it is the cite at the subject home's frontmatter line 4).

**old_string** (lines 13-14):
```
**Specification:** `QAHAL_API_SPECIFICATION_v1.0.md`
**Architecture:** `elohim/ELOHIM_PROTOCOL_ARCHITECTURE.md`
```

**new_string**:
```
**Legacy reference:** git history of `QAHAL_API_SPECIFICATION_v1.0.md` (spec retired 2026-06-11 — extracted into canon; see `genesis/docs/content/elohim-protocol/history/2026-06-11-qahal-api-spec-extraction-arc.md`)
**Architecture:** the subject home `qahal-domain-gospel` (`elohim/sdk/domains/qahal/CLAUDE.md`) + `genesis/docs/superpowers/specs/2026-05-21-qahal-architecture-vision.md`
```

PRECONDITION: the history record `2026-06-11-qahal-api-spec-extraction-arc.md` must exist at
placement time (authored by the history disposition group of this same recompose).

### Edit 1.2 — Models table (stale: human-consent is cross-pillar; 3 in-pillar models missing)

**Why:** `app/elohim-app/src/app/qahal/models/` contains: `collective.model.ts`,
`collective-research.model.ts`, `governance-deliberation.model.ts`, `governance-feedback.model.ts`,
`human-affinity.model.ts`, `mutual-aid.model.ts`, `place.model.ts` (+ `index.ts`). There is NO
`human-consent.model.ts` in the pillar — it lives at
`app/elohim-app/src/app/elohim/models/human-consent.model.ts` and is re-exported by
`qahal/models/index.ts` (line: `export * from '@app/elohim/models/human-consent.model';`).
The gospel table (lines 35-41) lists human-consent as in-pillar and omits collective,
collective-research, and mutual-aid. Purposes below are taken from each model's header comment.

**old_string** (lines 35-41):
```
| Model | Purpose |
|-------|---------|
| `human-affinity.model.ts` | Engagement depth tracking |
| `human-consent.model.ts` | Graduated intimacy levels |
| `governance-feedback.model.ts` | Challenges, appeals, precedent |
| `governance-deliberation.model.ts` | Loomio/Polis-style deliberation |
| `place.model.ts` | Bioregional geographic context |
```

**new_string**:
```
| Model | Purpose |
|-------|---------|
| `human-affinity.model.ts` | Engagement depth tracking |
| `governance-feedback.model.ts` | Challenges, appeals, precedent |
| `governance-deliberation.model.ts` | Loomio/Polis-style deliberation |
| `place.model.ts` | Bioregional geographic context |
| `collective.model.ts` | Collectives as governance contexts with graduated participation |
| `collective-research.model.ts` | Multi-participant research coordination |
| `mutual-aid.model.ts` | Mutual-aid contexts — governance primitive for resource solidarity |

Consent types (`human-consent.model.ts`, graduated intimacy) live in the elohim pillar
(`@app/elohim/models/`) and are re-exported via `qahal/models/index.ts`.
```

### Edit 1.3 — Services table (stale: the 3 listed services are cross-pillar; in-pillar + retirements missing)

**Why:** `app/elohim-app/src/app/qahal/services/` contains ONLY `collective.service.ts` and
`bracket-synthesis.service.ts` (+ `index.ts`). The three services the gospel claims (lines 45-49)
actually live in `app/elohim-app/src/app/elohim/services/` (`affinity-tracking.service.ts`,
`human-consent.service.ts`, `governance.service.ts`) and are re-exported by
`qahal/services/index.ts`. That same index.ts documents server-side retirements:
`MechanismSelectionService` (M-POLICY-2), `SignalAccumulationService` (M-POLICY-1),
`GovernanceRecognitionService` (M-REA-3) — all replaced by `GovernanceApiService` (defined at
`app/elohim-library/projects/elohim-service/src/angular/services/governance-api.service.ts`,
imported as `@elohim/service`).

**old_string** (lines 43-49):
```
## Services

| Service | Purpose |
|---------|---------|
| `AffinityTrackingService` | Content engagement tracking |
| `HumanConsentService` | Consent-based relationship management |
| `GovernanceService` | Constitutional moderation |
```

**new_string**:
```
## Services

In-pillar (`qahal/services/`):

| Service | Purpose |
|---------|---------|
| `CollectiveService` | Collective CRUD, membership management |
| `BracketSynthesisService` | Polis bracket synthesis for sensemaking |

Cross-pillar (live in `@app/elohim/services/`, re-exported via `qahal/services/index.ts`):

| Service | Purpose |
|---------|---------|
| `AffinityTrackingService` | Content engagement tracking |
| `HumanConsentService` | Consent-based relationship management |
| `GovernanceService` | Constitutional moderation |

Retired to server-side (see retirement comments in `qahal/services/index.ts`):
`MechanismSelectionService` (M-POLICY-2), `SignalAccumulationService` (M-POLICY-1),
`GovernanceRecognitionService` (M-REA-3) — use `GovernanceApiService` (`@elohim/service`)
projections (`getMechanismSelection()`, `getAccumulationStatus()`, `postParticipation()`).
```

### Edit 1.4 — Routes section (stale: 8 registered routes shown as "Future")

**Why:** `app/elohim-app/src/app/qahal/community.routes.ts` registers, under a lazy
`CommunityLayoutComponent` shell: `''` (CommunityHome), `directory`, `collective/:id`,
`governance/sensemaking`, `governance/challenges`, `governance/challenges/new` (identityGuard),
`governance/challenges/:id`, `governance/disposition`, `governance/proxy-votes`
(community.routes.ts:25-116). The gospel (lines 53-56) shows only CommunityHomeComponent plus a
"Future" comment.

**old_string** (lines 53-56):
```
```typescript
{ path: '', component: CommunityHomeComponent }
// Future: /community/governance, /community/places
```
```

**new_string**:
```
Registered in `community.routes.ts` (all lazy, under a `CommunityLayoutComponent` shell):

```
/community                              Community home
/community/directory                    Community directory
/community/collective/:id               Collective detail
/community/governance/sensemaking       Polis-style sensemaking
/community/governance/challenges        Challenge list (+ /new with identityGuard, /:id detail)
/community/governance/disposition       Governance disposition profile
/community/governance/proxy-votes       Proxy vote notifications
```

Future: `/community/human`, `/community/governance` (dashboard), `/community/places`.
```

### Edit 1.5 — Consent Model code block (minor drift: `ConsentState` missing `expired`)

**Why:** `IntimacyLevel` matches code exactly, but `ConsentState` at
`app/elohim-app/src/app/elohim/models/protocol-core.model.ts:197` has SIX values — the gospel
(line 62) omits `'expired'`. Also worth one line of provenance: both types are elohim-pillar-owned.

**old_string** (line 62):
```
type ConsentState = 'not_required' | 'pending' | 'accepted' | 'declined' | 'revoked';
```

**new_string**:
```
type ConsentState = 'not_required' | 'pending' | 'accepted' | 'declined' | 'revoked' | 'expired';
```

(Optional, same section, after the code block close-fence — append provenance line:)
```
Types owned by the elohim pillar: `@app/elohim/models/protocol-core.model.ts`.
```

### Edit 1.6 — Challenge interface block (drift: 3 of 4 shown fields wrong vs code)

**Why:** The real `Challenge` at
`app/elohim-app/src/app/qahal/models/governance-feedback.model.ts:303-340` differs from the
gospel's block (lines 72-77) on every line except `challengerId`:
- `grounds` is a structured `ChallengeGrounds` (`primary: ChallengeGroundType`, 10 values at
  :372-382 — includes `factual-error`/`bias`/`inconsistency` but NOT `harm`);
- `state: ChallengeState` (:393-400) is `filed | acknowledged | under-review |
  additional-info-needed | escalated | resolved | appealed` — `upheld`/`dismissed` are NOT states
  (the nearest is `ChallengeResponse.decision: 'upheld' | 'rejected' | 'modified'`);
- the SLA field is `responseDeadline`, not `slaDeadline`.

**old_string** (lines 72-77):
```
interface Challenge {
  challengerId: string;
  grounds: 'factual-error' | 'bias' | 'inconsistency' | 'harm';
  state: 'filed' | 'under-review' | 'upheld' | 'dismissed';
  slaDeadline: string;  // Must respond within SLA
}
```

**new_string**:
```
// Condensed from governance-feedback.model.ts (the full interface is the source of truth)
interface Challenge {
  challengerId: string;
  standing: ChallengeStanding;       // why the challenger has the right to challenge
  grounds: ChallengeGrounds;         // primary: ChallengeGroundType (factual-error, bias, ...)
  state: ChallengeState;             // filed → acknowledged → under-review → ... → resolved
  responseDeadline: string;          // SLA — system MUST respond by this time
  response?: ChallengeResponse;      // decision: 'upheld' | 'rejected' | 'modified'
}
```

### Edit 1.7 — Feedback Profiles section (FeedbackMechanism is NOT typed in pillar code → reframe as vision)

**Why:** No `FeedbackMechanism` type exists anywhere in pillar (or elohim-pillar) code. The only
occurrences: the retiring spec `QAHAL_API_SPECIFICATION_v1.0.md:503` (a 12-value union — the
gospel's 5-value block matches neither code nor spec) and a comment at
`governance-deliberation.model.ts:156-157` referencing a `feedback-profile.model.ts` that does
not exist. What IS implemented: `FeedbackMechanismGatewayComponent` (exported at
`qahal/index.ts:25`) with mechanism selection computed SERVER-side per M-POLICY-2
(`MechanismSelectionView` wire type, see `qahal/services/index.ts` retirement comment).

**old_string** (lines 86-95, the code block + trailing line):
```
```typescript
type FeedbackMechanism =
  | 'approval-vote'       // Up/down (replaces "likes")
  | 'emotional-reaction'  // "I feel ___ about this"
  | 'graduated-usefulness'// Loomio-style scales
  | 'discussion-only'     // No amplification
  | 'view-only';          // No engagement permitted
```

NO Facebook-style "likes" - they are fundamentally pernicious.
```

**new_string**:
```
NO Facebook-style "likes" — they are fundamentally pernicious. Mechanism selection is computed
server-side (M-POLICY-2; `MechanismSelectionView` via `GovernanceApiService`) and rendered by
`FeedbackMechanismGatewayComponent`. The full graduated mechanism vocabulary (approval-vote,
emotional-reaction, graduated-usefulness, discussion-only, view-only, ...) is vision, not yet a
typed client contract — tracked at the backlog entry `qahal-feedback-profile-vision-remainder`.
```

PRECONDITION: backlog entry slug `qahal-feedback-profile-vision-remainder` is authored by the
backlog disposition group of this recompose — verify it exists in
`genesis/data/timeline/backlog/` at placement time (it does NOT exist in the tree as of this
plan's writing; the only existing qahal backlog entries are
`qahal-collective-cid-formation-projection-gap.md` and `qahal-household-collective-first-class.md`).

---

## TARGET 2 — `elohim/sdk/domains/qahal/CLAUDE.md` (subject home gospel, id: qahal-domain-gospel)

### Edit 2.1 — "Key Services (Angular)" table (2 of 4 rows name retired services)

**Why:** `MechanismSelectionService` and `SignalAccumulationService` were retired to server-side
(M-POLICY-2 / M-POLICY-1) — see retirement comments in
`app/elohim-app/src/app/qahal/services/index.ts`; replacement is `GovernanceApiService`
(`getMechanismSelection()` / `getAccumulationStatus()`, wire types `MechanismSelectionView` /
`AccumulationStatusView`). Current table at lines 117-122.

**old_string** (lines 117-122):
```
| Service | Purpose |
|---------|---------|
| `CollectiveService` | Community CRUD, membership management |
| `MechanismSelectionService` | Voting mechanism selection based on proposal type |
| `SignalAccumulationService` | Graduated feedback → formal proposal escalation |
| `BracketSynthesisService` | Polis bracket synthesis for sensemaking |
```

**new_string**:
```
| Service | Purpose |
|---------|---------|
| `CollectiveService` | Community CRUD, membership management |
| `GovernanceApiService` (`@elohim/service`) | Server-side governance projections — mechanism selection (M-POLICY-2, `MechanismSelectionView`) and signal accumulation (M-POLICY-1, `AccumulationStatusView`) |
| `BracketSynthesisService` | Polis bracket synthesis for sensemaking |
```

### Edit 2.2 — "Related Files" table — NO change needed (verified)

**Why verified-current:** all seven files
`genesis/plans/2026-03-15-governance-gateway-sprint{3..9}-plan.md` exist on disk (ls-verified,
sprint3 through sprint9). `app/elohim-app/src/app/qahal/` exists; `elohim/sdk/schemas/v1/`
exists. The "Psephos design — See Sophia architecture notes" row is vague but not provably dead;
leave as-is (minimal-diff rule).

---

## Apply notes

- Apply order: Target 1 edits top-to-bottom (1.1 → 1.7), then Target 2 (2.1). All old_strings
  are unique within their files as quoted.
- Edits 1.1 and 1.7 carry PRECONDITIONS on sibling disposition groups (history record + backlog
  entry existing at placement time). If either artifact is renamed, update the path/slug here
  before applying.
- The pillar gospel's frontmatter cite to `qahal-domain-gospel` will drift STALE after Edit 2.1
  changes the subject home body — expected; re-seal via cite tooling after both targets land.
- No edits to `QAHAL_API_SPECIFICATION_v1.0.md` itself in this group — its deletion is the
  spec-extraction disposition group's move.
