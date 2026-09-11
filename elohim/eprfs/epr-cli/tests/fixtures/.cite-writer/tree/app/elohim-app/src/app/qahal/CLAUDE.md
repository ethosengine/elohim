---
id: qahal-pillar-gospel
cites:
  - qahal-domain-gospel | the subject SOURCE OF TRUTH this pillar consumes — governance content types, metadata schemas, graduated-standing function, commons-elohim coupling (renders, never redefines) | sha256:002d11309d8d9620 | path: elohim/sdk/domains/qahal/CLAUDE.md
---

# Qahal Pillar - Community

Community relationships, consent, governance, and deliberation.

*Qahal (קהל) = Hebrew for assembly/congregation*

**Specification:** `QAHAL_API_SPECIFICATION_v1.0.md`
**Architecture:** `elohim/ELOHIM_PROTOCOL_ARCHITECTURE.md`

## Subject home & citation discipline (this pillar is a CONSUMER)

This Angular pillar does not OWN the qahal subject — it consumes it. The vocabulary, metadata schemas, and
three-leg coupling (the governance content types `collective`/`proposal`/`challenge`/`statement`, the
graduated-standing function, the commons-elohim co-steward coupling) are the source of truth at the cited
subject home `qahal-domain-gospel` (`elohim/sdk/domains/qahal/`). The cite is content-addressed: a change at
the subject home drifts this gospel STALE for re-verification.

**Where code citations to the subject belong:**
- `generated/` is DERIVED from the subject home — never hand-edit; regenerate with `pnpm run qahal:codegen`.
- When code wires the graduated-standing function, Bloom-tier capability surfaces, or rubric versioning, leave
  a `// subject: qahal-domain-gospel` breadcrumb at the assumption site.
- When a service gates visible content by reach or capability by standing, cite the three-layer
  graduated-capability surface and friction-gradient rules the subject home owns.
- When code instantiates the commons-elohim co-steward (reflection / mediation / witness), cite its
  dual-stewardship model rather than re-deriving it here.

## Models

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

## Routes

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

## Consent Model (Graduated Intimacy)

```typescript
type IntimacyLevel = 'recognition' | 'connection' | 'trusted' | 'intimate';
type ConsentState = 'not_required' | 'pending' | 'accepted' | 'declined' | 'revoked' | 'expired';
```

Types owned by the elohim pillar: `@app/elohim/models/protocol-core.model.ts`.

Relationships progress through levels with explicit consent at each transition.

## Governance Model

Constitutional moderation with challenge rights:

```typescript
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

Every decision can be challenged - constitutional right.

## Feedback Profiles

"Virality is a privilege, not an entitlement."

NO Facebook-style "likes" — they are fundamentally pernicious. Mechanism selection is computed
server-side (M-POLICY-2; `MechanismSelectionView` via `GovernanceApiService`) and rendered by
`FeedbackMechanismGatewayComponent`. The full mechanism/reaction vocabulary (12-mechanism
friction hierarchy, mediated emotional reactions) is typed in the lamad bundle
(`app/lamad/src/app/models/feedback-profile.model.ts`) and rendered by `ReactionBarComponent`,
but the persisted, evolving profile dimension is unimplemented — tracked at the backlog entry
`qahal-feedback-profile-vision-remainder`.

## Geographic Context

Content has parallel reach dimensions:
- **Social reach**: WHO can access (private → commons)
- **Geographic reach**: WHERE content is relevant (local → bioregional)
