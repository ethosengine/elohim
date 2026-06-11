---
id: qahal-pillar-gospel
cites:
  - qahal-domain-gospel | the subject SOURCE OF TRUTH this pillar consumes — governance content types, metadata schemas, graduated-standing function, commons-elohim coupling (renders, never redefines) | sha256:c1afc1a5a0746893 | path: elohim/sdk/domains/qahal/CLAUDE.md
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
| `human-consent.model.ts` | Graduated intimacy levels |
| `governance-feedback.model.ts` | Challenges, appeals, precedent |
| `governance-deliberation.model.ts` | Loomio/Polis-style deliberation |
| `place.model.ts` | Bioregional geographic context |

## Services

| Service | Purpose |
|---------|---------|
| `AffinityTrackingService` | Content engagement tracking |
| `HumanConsentService` | Consent-based relationship management |
| `GovernanceService` | Constitutional moderation |

## Routes

```typescript
{ path: '', component: CommunityHomeComponent }
// Future: /community/governance, /community/places
```

## Consent Model (Graduated Intimacy)

```typescript
type IntimacyLevel = 'recognition' | 'connection' | 'trusted' | 'intimate';
type ConsentState = 'not_required' | 'pending' | 'accepted' | 'declined' | 'revoked';
```

Relationships progress through levels with explicit consent at each transition.

## Governance Model

Constitutional moderation with challenge rights:

```typescript
interface Challenge {
  challengerId: string;
  grounds: 'factual-error' | 'bias' | 'inconsistency' | 'harm';
  state: 'filed' | 'under-review' | 'upheld' | 'dismissed';
  slaDeadline: string;  // Must respond within SLA
}
```

Every decision can be challenged - constitutional right.

## Feedback Profiles

"Virality is a privilege, not an entitlement."

```typescript
type FeedbackMechanism =
  | 'approval-vote'       // Up/down (replaces "likes")
  | 'emotional-reaction'  // "I feel ___ about this"
  | 'graduated-usefulness'// Loomio-style scales
  | 'discussion-only'     // No amplification
  | 'view-only';          // No engagement permitted
```

NO Facebook-style "likes" - they are fundamentally pernicious.

## Geographic Context

Content has parallel reach dimensions:
- **Social reach**: WHO can access (private → commons)
- **Geographic reach**: WHERE content is relevant (local → bioregional)
