# Qahal Pillar - Community

Community relationships, consent, governance, and deliberation.

*Qahal (קהל) = Hebrew for assembly/congregation*

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
