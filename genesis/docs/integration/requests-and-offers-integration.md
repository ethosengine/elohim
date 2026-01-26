# Requests and Offers Integration with Shefa
## Lift-and-Shift from Research to Production

**Status:** Phase 1 - Models and Service Stubs Complete
**Date:** December 22, 2025

---

## Overview

The `/research/requests-and-offers` project demonstrates a mature peer-to-peer bulletin board system. We've lifted its core domain concepts and integrated them with Shefa's REA/ValueFlows infrastructure.

**Key Architecture Decision:** Requests and Offers are **REA Intents** in the economic vocabulary.

```
ServiceRequest = Intent (receiver wants to take something)
ServiceOffer   = Intent (provider wants to give something)
ServiceMatch   = Proposal (matching request intent + offer intent)
```

---

## Core Domain Models Lifted

### From Research Project → Shefa

| Research Concept | Shefa Integration | Purpose |
|------------------|-------------------|---------|
| Request | ServiceRequest extends Intent | Someone needs a service |
| Offer | ServiceOffer extends Intent | Someone provides a service |
| Match | ServiceMatch | Request + Offer pair |
| ServiceType | ServiceType | Categorize requests/offers |
| MediumOfExchange | MediumOfExchange | What can be exchanged (€, $, time, care tokens) |
| UserPreferences | UserPreferences | When/how/where to work |
| Status (Admin) | ListingAdminStatus | Pending/Accepted/Rejected/Suspended |
| SavedRequests/Offers | SavedRequest / SavedOffer | Favorites system |

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│ Requests & Offers (New Layer)                               │
│                                                             │
│  ServiceRequest      ServiceOffer      ServiceMatch        │
│  + preferences       + preferences      (linking)          │
│  + service types     + service types                       │
│  + mediums          + mediums                              │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Shefa REA/ValueFlows                                        │
│                                                             │
│  Intent (request/offer as forward-looking economic flow)  │
│  Proposal (match becomes proposal linking intents)        │
│  Commitment (when work is agreed to)                       │
│  Agreement (the terms of work)                            │
│  EconomicEvent (immutable audit trail)                     │
│  CommonsPool (settlement of payments)                      │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Supporting Systems                                          │
│                                                             │
│  Observer Protocol (verify work completed)                │
│  Holochain DHT (distributed storage)                      │
│  EconomicService (event creation)                          │
└─────────────────────────────────────────────────────────────┘
```

---

## Workflow 1: Create Request

**Scenario:** Sarah needs logo design for her startup.

```typescript
// 1. Create request
const { request, intent, createdEvent } = await requestsAndOffers.createRequest(
  'sarah-id',
  {
    title: 'Need modern logo design for tech startup',
    description: 'Looking for a designer to create a clean, modern logo...',
    contactPreference: 'email',
    contactValue: 'sarah@startup.io',
    dateRange: {
      startDate: '2025-12-22',
      endDate: '2026-01-15',
      flexibleDates: true,
    },
    timeZone: 'EST',
    timePreference: 'afternoon',
    interactionType: 'virtual',
    estimatedHours: 20,
    serviceTypeIds: ['service-type-graphic-design'],
    requiredSkills: ['Figma', 'branding', 'vector design'],
    budget: {
      amount: { hasNumericalValue: 1500, hasUnit: 'unit-token' },
      mediumOfExchangeId: 'medium-eur',
    },
    mediumOfExchangeIds: ['medium-eur', 'medium-mutual-credit'],
    isPublic: true,
  }
);

// Result:
// - request.id = 'req-12345'
// - request.requestNumber = 'REQ-2025-001234'
// - request.status = 'active'
// - request.adminStatus.statusType = 'pending' (awaiting review)
// - intent.id = 'intent-12345' (REA Intent)
// - createdEvent.id = 'event-12345' (immutable record)
```

**Behind the scenes:**

1. Create `ServiceRequest` entity
   - Store all details: title, description, preferences
   - Link service types
   - Link mediums of exchange
   - Set status to 'active'
   - Set admin status to 'pending'

2. Create REA `Intent`
   - action: 'take' (receiver wants to take design service)
   - resourceConformsTo: service-type-graphic-design
   - resourceQuantity: 20 hours
   - receiver: 'sarah-id'
   - provider: null (not yet matched)

3. Create `EconomicEvent`
   - action: 'deliver-service'
   - provider: 'sarah-id'
   - receiver: 'system'
   - lamadEventType: 'service-request-created'
   - metadata: { requestId, title, budget }

4. **Admin Review**
   - Admin sees request pending
   - Reviews for authenticity, completeness
   - Approves → statusType = 'accepted'
   - Now visible in search

---

## Workflow 2: Create Offer

**Scenario:** Marcus is a designer who offers logo design services.

```typescript
// 1. Create offer
const { offer, intent, createdEvent } = await requestsAndOffers.createOffer(
  'marcus-id',
  {
    title: 'Professional logo design and branding',
    description: 'I\'m a designer with 10 years experience in startup branding...',
    contactPreference: 'email',
    contactValue: 'marcus@design.studio',
    dateRange: {
      startDate: '2025-12-22',
      endDate: '2026-06-30',
      flexibleDates: false,
    },
    timeZone: 'EST',
    timePreference: 'morning',
    interactionType: 'virtual',
    hoursPerWeek: 30,
    serviceTypeIds: ['service-type-graphic-design', 'service-type-branding'],
    offeredSkills: ['Figma', 'branding', 'vector design', 'print design'],
    rate: {
      amount: { hasNumericalValue: 75, hasUnit: 'unit-token' },
      per: 'hour',
      mediumOfExchangeId: 'medium-eur',
    },
    mediumOfExchangeIds: ['medium-eur', 'medium-mutual-credit'],
    acceptsAlternativePayment: true,
    isPublic: true,
  }
);

// Result:
// - offer.id = 'off-54321'
// - offer.offerNumber = 'OFR-2025-005678'
// - offer.status = 'active'
// - offer.adminStatus.statusType = 'pending'
// - intent.id = 'intent-54321' (REA Intent)
// - createdEvent.id = 'event-54321'
```

**Behind the scenes:**

1. Create `ServiceOffer` entity
2. Create REA `Intent`
   - action: 'give' (provider wants to give design service)
   - resourceConformsTo: service-type-graphic-design
   - provider: 'marcus-id'
   - receiver: null (not yet matched)
3. Create `EconomicEvent` (service-offer-created)
4. Admin review and approval

---

## Workflow 3: Discover & Match

**Scenario:** Search for logo designers, find Marcus.

```typescript
// 1. Search for offers matching Sarah's request
const matches = await requestsAndOffers.findMatchesForRequest('req-12345', 10);
// Returns: [
//   {
//     id: 'match-123',
//     requestId: 'req-12345',
//     offerId: 'off-54321',
//     matchQuality: 95,
//     sharedServiceTypes: ['service-type-graphic-design'],
//     timeCompatible: true,
//     interactionCompatible: true,
//     exchangeCompatible: true,
//     status: 'suggested',
//   },
//   // ... more matches
// ]

// Matching algorithm checks:
// - Request: graphic-design, 20 hours, €1500, virtual, afternoon, EST
// - Offer: graphic-design + branding, 30 hrs/week available, €75/hr, virtual, morning, EST
// - Match quality: service overlap (100%), rate compatibility (95%), time zones (90%)
```

**The Matching System:**

```typescript
calculateMatchQuality(request: ServiceRequest, offer: ServiceOffer): number {
  let score = 0;

  // Service type overlap
  const sharedServices = intersection(request.serviceTypeIds, offer.serviceTypeIds);
  const serviceScore = (sharedServices.length / request.serviceTypeIds.length) * 100;

  // Rate compatibility
  if (offer.rate && request.budget) {
    const requiredHours = request.estimatedHours || 1;
    const offerTotal = offer.rate.amount.hasNumericalValue * requiredHours;
    const budgetTotal = request.budget.amount.hasNumericalValue;
    const rateScore = Math.min(offerTotal, budgetTotal) / Math.max(offerTotal, budgetTotal) * 100;
  }

  // Time compatibility
  const timeScore = getTimePreferenceCompatibility(request.timePreference, offer.timePreference);
  const zoneScore = getTimeZoneDistance(request.timeZone, offer.timeZone);
  const dateScore = getDateRangeOverlap(request.dateRange, offer.dateRange);

  // Interaction type
  const interactionScore = canInteract(request.interactionType, offer.interactionType) ? 100 : 0;

  // Exchange options
  const exchangeScore = hasCommonMedium(request.mediumOfExchangeIds, offer.mediumOfExchangeIds) ? 100 : 50;

  return (serviceScore * 0.4 + rateScore * 0.3 + timeScore * 0.2 + interactionScore * 0.1) * (exchangeScore / 100);
}
```

---

## Workflow 4: Propose & Negotiate

**Scenario:** Marcus proposes his offer to Sarah's request.

```typescript
// 1. Marcus sees Sarah's request in search
// 2. Clicks "Propose my offer"
const { match, proposal, contactEvent } = await requestsAndOffers.proposeOfferToRequest(
  'off-54321',      // Marcus's offer
  'req-12345',      // Sarah's request
  'Hi Sarah! I\'ve got experience with startup branding. I can create a modern logo in your budget. Let\'s chat!'
);

// Result:
// - match.id = 'match-123'
// - match.status = 'contacted'
// - proposal.id = 'prop-456' (REA Proposal)
// - contactEvent.id = 'event-456'
```

**Behind the scenes:**

1. Create `ServiceMatch`
   - Link request + offer
   - Calculate match quality
   - Set status to 'contacted'

2. Create REA `Proposal`
   - publishes: [request intent, offer intent]
   - This is how REA handles "matching" two intents
   - Proposal becomes container for negotiation

3. Create `EconomicEvent` (service-proposal-created)
   - Records that contact was initiated
   - Includes message

4. **Notification**
   - Sarah gets notified of proposal
   - Can accept, reject, or counter-propose

---

## Workflow 5: Agreement

**Scenario:** Sarah and Marcus agree on terms.

```typescript
// 1. Sarah reviews Marcus's proposal
// 2. Agrees to terms
const { proposal, commitment, agreementEvent } = await requestsAndOffers.acceptProposal(
  'prop-456',
  'sarah-id',
  {
    rate: { hasNumericalValue: 75, hasUnit: 'unit-token' },
    schedule: 'Kickoff Jan 1, final delivery Jan 15',
    deliverables: '3 logo concepts, 2 rounds of revisions, final files in Figma + PNG/SVG',
  }
);

// Result:
// - proposal is now 'accepted'
// - commitment.id = 'commit-789' (REA Commitment)
// - commitment.status = 'accepted'
// - agreementEvent.id = 'event-789'
```

**Behind the scenes:**

1. **REA Commitment Created**
   - Formalizes the work to be done
   - action: 'deliver-service'
   - provider: 'marcus-id' (will deliver design)
   - receiver: 'sarah-id' (will receive design)
   - resourceQuantity: 20 hours
   - status: 'accepted'

2. **REA Agreement Created**
   - Documents terms
   - Schedule, deliverables, rate
   - Both parties are parties to agreement
   - Constitutional basis (if needed)

3. **EconomicEvent Created**
   - lamadEventType: 'service-agreed'
   - Records commitment to work
   - Full audit trail

4. **Match Updated**
   - match.status = 'agreed'
   - match.proposalId = proposal.id
   - match.commitmentIds = [commitment.id]

---

## Workflow 6: Work & Completion

**Scenario:** Marcus does the work, Sarah receives it.

```typescript
// 1. Marcus marks work as complete
const { commitment, completionEvent } = await requestsAndOffers.markWorkComplete(
  'commit-789',
  'marcus-id',
  {
    description: 'Delivered 3 logo concepts with presentation slides',
    links: ['https://figma.com/...', 'https://design-brief.pdf'],
  }
);

// Result:
// - completionEvent records work was done
// - Triggers settlement workflow
```

**Behind the scenes:**

1. Create `EconomicEvent` (service-work-completed)
   - Marks commitment fulfilled
   - Stores deliverables and evidence

2. **Settlement Workflow Triggered**
   - Sarah has 3 days to review
   - Can accept or dispute
   - If accepted: payment settled

---

## Workflow 7: Settlement & Payment

**Scenario:** Sarah accepts work, Marcus gets paid.

```typescript
// 1. Sarah accepts deliverables
// 2. Settlement is processed
const { settlement, reputation } = await requestsAndOffers.settlePayment(
  'match-123',
  {
    amount: { hasNumericalValue: 1500, hasUnit: 'unit-token' },
    mediumOfExchangeId: 'medium-eur',
    paymentMethod: 'mutual-credit',
    note: 'Payment for logo design project',
  }
);

// Result:
// - settlement event records payment
// - Marcus's account credited
// - Sarah's account debited
// - Match marked as 'completed'
// - Reputation flow created (recognition)
```

**Behind the scenes:**

1. **Create Payment Event**
   - action: 'transfer'
   - provider: 'sarah-id'
   - receiver: 'marcus-id'
   - resourceQuantity: 1500 EUR
   - fulfills: commitment.id

2. **Update Accounts** (if mutual credit)
   - Debit Sarah's mutual-credit balance
   - Credit Marcus's mutual-credit balance
   - Via CommonsPool or direct transfer

3. **Create Reputation Event** (optional)
   - Sarah gives Marcus recognition
   - "Great designer, delivered on time"
   - Creates 'reputation' or 'endorsement' event
   - Builds Marcus's credibility in network

4. **Update Match**
   - match.status = 'completed'
   - match.completedAt = now
   - match.statusReason = "Payment settled"

5. **Immutable Audit Trail**
   - All events recorded in EconomicService
   - Full history: request → offer → match → agreement → completion → payment → reputation
   - Available for:
     - Dispute resolution
     - Reputation calculation
     - Economic analysis
     - Governance review

---

## Integration with Shefa Infrastructure

### 1. EconomicService Integration

Every operation creates events:

```typescript
// When request created
const event = await economicService.createEvent({
  action: 'deliver-service',
  provider: 'sarah-id',
  receiver: 'system',
  metadata: {
    lamadEventType: 'service-request-created',
    requestId: 'req-12345',
  },
});

// When payment settled
const event = await economicService.createEvent({
  action: 'transfer',
  provider: 'sarah-id',
  receiver: 'marcus-id',
  resourceQuantity: { hasNumericalValue: 1500, hasUnit: 'EUR' },
  fulfills: ['commitment-789'],
  metadata: {
    lamadEventType: 'service-payment-settled',
    matchId: 'match-123',
  },
});
```

**Benefits:**
- Immutable audit trail
- Full transparency
- Enables dispute resolution
- Supports governance review
- Provides reputation calculation basis

### 2. CommonsPool Integration

If using mutual credit for payment:

```typescript
// When payment settles via mutual credit
const pool = await getCommonsPool('mutual-credit-pool');

// Sarah's balance decreases
pool.balance -= 1500;

// Marcus gets claim against pool
const attribution: AttributionClaim = {
  agentId: 'marcus-id',
  amount: { hasNumericalValue: 1500, hasUnit: 'mutual-credit' },
  sourceEventIds: ['settlement-event-id'],
};

// Marcus can claim his credits
```

### 3. Intent/Proposal Pattern

ServiceRequest/Offer as REA Intents:

```typescript
// ServiceRequest → Intent
{
  action: 'take',
  provider: null,  // Not yet matched
  receiver: 'sarah-id',
  resourceConformsTo: 'service-type-graphic-design',
  resourceQuantity: { hasNumericalValue: 20, hasUnit: 'hour' },
}

// ServiceOffer → Intent
{
  action: 'give',
  provider: 'marcus-id',
  receiver: null,  // Not yet matched
  resourceConformsTo: 'service-type-graphic-design',
  resourceQuantity: { hasNumericalValue: 30, hasUnit: 'hour-per-week' },
}

// When matched → Proposal
{
  publishes: [request_intent_id, offer_intent_id],
  reciprocal: [rate_intent_id],  // Marcus wants €1500
}
```

### 4. Observer Protocol Integration

Track completion and reputation:

```typescript
// Marcus submits work with Observer evidence
{
  observerAttestationIds: [
    'obs-delivery-confirmed',  // Observer witnessed delivery
    'obs-quality-check',       // Observer verified quality
  ],
}

// Sarah can verify work was actually delivered
// Creates reputation evidence for future matches
```

---

## Data Model: How It All Fits Together

```
User (REAAgent)
  ├── UserPreferences
  ├── ServiceRequests []
  │   ├── ServiceType[] links
  │   ├── MediumOfExchange[] links
  │   └── REA Intent
  ├── ServiceOffers []
  │   ├── ServiceType[] links
  │   ├── MediumOfExchange[] links
  │   └── REA Intent
  └── SavedRequests/Offers []

ServiceRequest (extends Intent)
  ├── ServiceType[] (what category)
  ├── MediumOfExchange[] (what payment)
  ├── UserPreferences (metadata)
  ├── ListingAdminStatus
  └── ServiceMatch[] (when matched)

ServiceOffer (extends Intent)
  ├── ServiceType[] (what category)
  ├── MediumOfExchange[] (what payment)
  ├── UserPreferences (metadata)
  ├── ListingAdminStatus
  └── ServiceMatch[] (when matched)

ServiceMatch
  ├── ServiceRequest
  ├── ServiceOffer
  ├── REA Proposal (negotiation container)
  ├── REA Commitment (if agreed)
  ├── REA Agreement (terms)
  └── EconomicEvent[] (full audit trail)

EconomicEvent
  ├── All lifecycle events
  ├── All payment events
  ├── All reputation flows
  └── Full immutable audit trail
```

---

## Event Types Added to Lamad

When these happen, events are created:

| Event | REA Action | Resource | Purpose |
|-------|-----------|----------|---------|
| service-request-created | deliver-service | stewardship | Request posted |
| service-offer-created | deliver-service | stewardship | Offer posted |
| service-proposal-created | deliver-service | stewardship | Match proposed |
| service-agreed | accept | membership | Work agreement made |
| service-work-completed | deliver-service | stewardship | Work finished |
| service-payment-settled | transfer | currency | Payment made |
| service-reputation | raise | recognition | Reputation given |

---

## Key Features Enabled

1. **Full Traceability** - Every action creates immutable event
2. **Reputation System** - Based on completed work and payments
3. **Dispute Resolution** - Full audit trail supports appeals
4. **Economic Integration** - Integrates with mutual credit and commons pools
5. **Governance Participation** - Contracts and disputes reviewable by governance
6. **Privacy** - Contact preferences managed by user
7. **Flexibility** - Multiple payment options (fiat, mutual credit, barter)
8. **Accessibility** - Support for virtual and in-person work
9. **Time Zone Awareness** - Schedule compatibility checking
10. **Skill Matching** - Service type and skill tagging

---

## Implementation Roadmap

### Phase 1a (Done ✅)
- Models defined
- Service stubs created
- Documentation written

### Phase 1b (Next)
1. Implement core methods:
   - `createRequest()` + `createOffer()`
   - `searchRequests()` + `searchOffers()`
   - `findMatchesForRequest()` + `findMatchesForOffer()`

2. Integrate with EconomicService
3. Wire to Holochain DHT storage

### Phase 2
4. Implement proposal & agreement
5. Implement work completion
6. Implement payment settlement

### Phase 3
7. Analytics and reputation
8. Admin interface
9. Governance integration

---

## Success Criteria

After Phase 1b, we should be able to:
- ✅ Create requests/offers with full preferences
- ✅ Search and discover with multiple filters
- ✅ Match algorithmically or manually
- ✅ Create proposals linking request + offer
- ✅ Full immutable audit trail
- ✅ All events in EconomicService

After Phase 2:
- ✅ Agree to work terms (commitment)
- ✅ Mark work complete with evidence
- ✅ Settle payment
- ✅ Build reputation from work

---

## Connection to Elohim Mutual

This requests-and-offers system can coordinate work for:
- Adjusters helping members with insurance claims
- Prevention specialists coordinating risk mitigation
- Arbiters resolving disputes
- Governance committee members reviewing decisions

All tracked economically with full audit trail and reputation building.

---

**Status:** Phase 1 Complete - Ready for Phase 1b Implementation
**Next Step:** Implement core search and matching services

