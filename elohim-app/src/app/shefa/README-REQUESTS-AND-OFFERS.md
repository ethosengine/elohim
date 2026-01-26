# Shefa Requests and Offers Module

## Overview

This module implements a peer-to-peer marketplace/bulletin board for coordinating work, services, and resource exchange.

**Lifted from:** `/research/requests-and-offers` - A production-grade Holochain application
**Integrated with:** Shefa REA/ValueFlows economic infrastructure

**Core Principle:** ServiceRequest and ServiceOffer are **REA Intents** - forward-looking economic flows.

---

## Module Structure

```
shefa/
├── models/
│   └── requests-and-offers.model.ts
│       ├── ServiceRequest (extends Intent)
│       ├── ServiceOffer (extends Intent)
│       ├── ServiceMatch (request + offer pair)
│       ├── ServiceType (categorization)
│       ├── MediumOfExchange (payment method)
│       ├── UserPreferences (contact/schedule)
│       ├── ListingAdminStatus (moderation)
│       └── SavedRequest/SavedOffer (favorites)
│
└── services/
    └── requests-and-offers.service.ts
        ├── Request management
        ├── Offer management
        ├── Search & discovery
        ├── Matching (algorithmic)
        ├── Proposal & coordination
        ├── Completion & settlement
        ├── Preferences & settings
        ├── Service types & mediums
        └── Admin & moderation
```

---

## Core Models

### ServiceRequest

Someone is requesting a service or resource.

```typescript
const request: ServiceRequest = {
  id: 'req-12345',
  requestNumber: 'REQ-2025-001234',
  requesterId: 'user-123',
  title: 'Need logo design for startup',
  description: '...',

  // Contact & timing
  contactPreference: 'email',
  contactValue: 'user@example.com',
  timeZone: 'EST',
  timePreference: 'afternoon',
  interactionType: 'virtual',
  dateRange: { startDate: '2025-12-22', endDate: '2026-01-15' },

  // Resources & payment
  serviceTypeIds: ['graphic-design'],
  requiredSkills: ['Figma', 'branding'],
  budget: { amount: { value: 1500, unit: 'EUR' }, mediumId: '...' },
  mediumOfExchangeIds: ['EUR', 'mutual-credit'],

  // Metadata
  status: 'active',
  isPublic: true,
  links: ['portfolio.com', 'brand-guide.pdf'],
  createdAt: '2025-12-22T10:00:00Z',
};
```

### ServiceOffer

Someone is offering a service or resource.

```typescript
const offer: ServiceOffer = {
  id: 'off-54321',
  offerNumber: 'OFR-2025-005678',
  offerorId: 'user-456',
  title: 'Professional logo design and branding',
  description: '...',

  // Contact & availability
  contactPreference: 'email',
  contactValue: 'marcus@design.studio',
  timeZone: 'EST',
  timePreference: 'morning',
  interactionType: 'virtual',
  hoursPerWeek: 30,
  dateRange: { startDate: '2025-12-22', endDate: '2026-06-30' },

  // Services & pricing
  serviceTypeIds: ['graphic-design', 'branding'],
  offeredSkills: ['Figma', 'branding', 'vector design'],
  rate: {
    amount: { value: 75, unit: 'EUR' },
    per: 'hour',
    mediumId: '...',
  },
  mediumOfExchangeIds: ['EUR', 'mutual-credit'],
  acceptsAlternativePayment: true,

  // Metadata
  status: 'active',
  isPublic: true,
  links: ['portfolio.com', 'case-studies.pdf'],
  createdAt: '2025-12-22T10:00:00Z',
};
```

### ServiceMatch

A potential or actual match between request and offer.

```typescript
const match: ServiceMatch = {
  id: 'match-123',
  requestId: 'req-12345',
  offerId: 'off-54321',
  matchReason: 'Shared service types, compatible times, overlapping mediums',
  matchQuality: 95,  // 0-100 score
  sharedServiceTypes: ['graphic-design'],
  timeCompatible: true,
  interactionCompatible: true,
  exchangeCompatible: true,
  status: 'suggested',  // → contacted → negotiating → agreed → completed
  createdAt: '2025-12-22T10:15:00Z',
};
```

### ServiceType

Category of service (e.g., "Logo Design", "Cloud Consulting").

```typescript
const serviceType: ServiceType = {
  id: 'st-graphic-design',
  name: 'Graphic Design',
  description: 'Visual design services including logos, branding, illustrations',
  isTechnical: false,
  creatorId: 'admin-1',
  status: 'active',
};
```

### MediumOfExchange

What can be exchanged (EUR, USD, mutual credit hours, care tokens).

```typescript
const medium: MediumOfExchange = {
  id: 'medium-eur',
  code: 'EUR',
  name: 'Euro',
  exchangeType: 'currency',
  resourceSpecHreaId: 'resource-spec-eur',  // Links to hREA
  creatorId: 'system',
  status: 'active',
};
```

### UserPreferences

When/how/where a user prefers to work.

```typescript
const prefs: UserPreferences = {
  id: 'pref-123',
  userId: 'user-123',
  contactPreference: 'email',
  contactValue: 'user@example.com',
  timeZone: 'EST',
  timePreference: 'afternoon',
  interactionType: 'virtual',
  availableHoursPerWeek: 20,
  languages: ['English', 'Spanish'],
  skillsToShare: ['Figma', 'branding'],
  skillsToLearn: ['3D design'],
};
```

---

## Core Workflows

### Workflow 1: Create & Discover

```typescript
// 1. Create request
const { request, intent, event } = await requestsAndOffers.createRequest(
  'user-123',
  {
    title: 'Logo design needed',
    description: '...',
    // ... other details
  }
);

// 2. Admin approves
await requestsAndOffers.approveRequest('req-12345', 'admin-1');

// 3. User discovers offers
const results = await requestsAndOffers.searchOffers({
  serviceTypeIds: ['graphic-design'],
  interactionType: 'virtual',
  rateMax: 100,
});

// 4. Find matches
const matches = await requestsAndOffers.findMatchesForRequest('req-12345');
// Returns: [
//   { matchQuality: 95, offerId: 'off-54321' },
//   { matchQuality: 87, offerId: 'off-54322' },
// ]
```

### Workflow 2: Match & Propose

```typescript
// 1. Offeror sees request, proposes
const { match, proposal, event } = await requestsAndOffers.proposeOfferToRequest(
  'off-54321',      // My offer
  'req-12345',      // Their request
  'I can help! I have 10 years experience...'
);

// Result:
// - match.status = 'contacted'
// - proposal = REA Proposal linking request intent + offer intent
// - Requester gets notified

// 2. Requester reviews & accepts
const { proposal, commitment, event } = await requestsAndOffers.acceptProposal(
  'prop-456',
  'user-123',
  {
    rate: { value: 75, unit: 'EUR' },
    schedule: 'Kickoff Jan 1, delivery Jan 15',
    deliverables: '3 concepts, 2 rounds revisions',
  }
);

// Result:
// - commitment = REA Commitment (formalizes work)
// - commitment.status = 'accepted'
// - Full agreement on terms
```

### Workflow 3: Complete & Settle

```typescript
// 1. Work is complete, offeror submits
const { commitment, event } = await requestsAndOffers.markWorkComplete(
  'commit-789',
  'user-456',
  {
    description: 'Delivered 3 logo concepts with presentation',
    links: ['figma.com/...', 'presentation.pdf'],
  }
);

// 2. Requester accepts & settles payment
const { settlement, reputation } = await requestsAndOffers.settlePayment(
  'match-123',
  {
    amount: { value: 1500, unit: 'EUR' },
    mediumOfExchangeId: 'medium-eur',
    paymentMethod: 'mutual-credit',
  }
);

// Result:
// - settlement event records payment
// - Offeror credited, requester debited
// - Match marked 'completed'
// - Reputation event created
```

---

## Service Methods

### Request Management
- `createRequest()` - Post a request
- `updateRequest()` - Modify (requester only)
- `archiveRequest()` - Soft delete
- `getRequest()` - Retrieve details
- `getUserRequests()` - Get user's requests

### Offer Management
- `createOffer()` - Post an offer
- `updateOffer()` - Modify (offeror only)
- `archiveOffer()` - Soft delete
- `getOffer()` - Retrieve details
- `getUserOffers()` - Get user's offers

### Search & Discovery
- `searchRequests()` - Multi-filter search
- `searchOffers()` - Multi-filter search
- `getTrendingRequests()` - Popular requests
- `getTrendingOffers()` - Popular offers

### Matching
- `findMatchesForRequest()` - Find compatible offers
- `findMatchesForOffer()` - Find compatible requests
- `createMatch()` - Manual matching
- `updateMatchStatus()` - Track match lifecycle

### Proposal & Coordination
- `proposeOfferToRequest()` - Offeror proposes to request
- `proposeRequestToOffer()` - Requester proposes to offer
- `acceptProposal()` - Both parties agree
- `rejectProposal()` - Either party rejects

### Completion & Settlement
- `markWorkComplete()` - Work is done
- `settlePayment()` - Pay offeror

### Preferences & Settings
- `setUserPreferences()` - Store contact/schedule preferences
- `getUserPreferences()` - Retrieve preferences
- `getRecommendedRequests()` - Personalized suggestions
- `getRecommendedOffers()` - Personalized suggestions

### Favorites
- `saveRequest()` - Bookmark request
- `saveOffer()` - Bookmark offer
- `getSavedRequests()` - Get bookmarks
- `getSavedOffers()` - Get bookmarks
- `unsaveRequest()` - Remove bookmark
- `unsaveOffer()` - Remove bookmark

### Administration
- `getPendingRequests()` / `getPendingOffers()` - Admin queue
- `approveRequest()` / `approveOffer()` - Publish listings
- `rejectRequest()` / `rejectOffer()` - Decline listings
- `suspendRequest()` / `suspendOffer()` - Temp/indefinite suspension

### Analytics
- `getActivityStats()` - Network-wide metrics
- `getUserActivitySummary()` - User's activity

---

## Integration with Shefa

### EconomicService

Every operation creates immutable events:

```typescript
// Request created
const event = await economicService.createEvent({
  action: 'deliver-service',
  provider: 'requester-id',
  metadata: { lamadEventType: 'service-request-created', requestId },
});

// Payment settled
const event = await economicService.createEvent({
  action: 'transfer',
  provider: 'requester-id',
  receiver: 'offeror-id',
  resourceQuantity: { value: 1500, unit: 'EUR' },
  metadata: { lamadEventType: 'service-payment-settled', matchId },
});
```

### CommonsPool

For mutual credit settlements:

```typescript
const pool = await getCommonsPool('mutual-credit');

// Update balance when payment settled
pool.balance -= paymentAmount;

// Create attribution for offeror
const attribution = await createAttribution({
  agentId: 'offeror-id',
  amount: paymentAmount,
  sourceEventIds: ['settlement-event-id'],
});
```

### Intent/Proposal Pattern

ServiceRequest = Intent (take action)
ServiceOffer = Intent (give action)
ServiceMatch = Proposal (linking intents)

```typescript
// Request intent
const requestIntent: Intent = {
  action: 'take',
  receiver: 'requester-id',
  resourceConformsTo: 'graphic-design',
  resourceQuantity: { value: 20, unit: 'hour' },
};

// Offer intent
const offerIntent: Intent = {
  action: 'give',
  provider: 'offeror-id',
  resourceConformsTo: 'graphic-design',
  resourceQuantity: { value: 30, unit: 'hour-per-week' },
};

// Match → Proposal
const proposal: Proposal = {
  publishes: [requestIntent.id, offerIntent.id],
};
```

---

## Event Types

New event types added to Lamad:

- `service-request-created` - Request posted
- `service-offer-created` - Offer posted
- `service-proposal-created` - Match proposed
- `service-agreed` - Terms agreed
- `service-work-completed` - Work finished
- `service-payment-settled` - Payment made
- `service-reputation` - Reputation given

---

## Key Features

✅ **Full Traceability** - Every action immutable
✅ **Reputation System** - Built on completed work
✅ **Algorithmic Matching** - Find compatible pairs
✅ **Flexible Payment** - Currency, mutual credit, barter
✅ **Privacy** - User-controlled contact preferences
✅ **Schedule Matching** - Time zone & availability aware
✅ **Admin Moderation** - Pending review, suspension
✅ **Search & Discovery** - Multi-filter search
✅ **Favorites** - Save interesting listings
✅ **Economic Integration** - Full Shefa integration

---

## Example: End-to-End Flow

```typescript
// 1. Sarah posts request
const request = await requestsAndOffers.createRequest('sarah-id', {
  title: 'Logo design needed',
  budget: { value: 1500, unit: 'EUR' },
  // ... other details
});

// 2. Admin approves
await requestsAndOffers.approveRequest(request.id, 'admin-1');

// 3. Marcus finds it
const results = await requestsAndOffers.searchRequests({
  serviceTypeIds: ['graphic-design'],
});

// 4. Marcus proposes
const { match } = await requestsAndOffers.proposeOfferToRequest(
  'marcus-offer-id',
  request.id,
  'I can help!'
);

// 5. Sarah accepts
const { commitment } = await requestsAndOffers.acceptProposal(
  match.proposalId,
  'sarah-id',
  { rate: { value: 75, unit: 'EUR' }, schedule: '3 weeks' }
);

// 6. Marcus delivers
const { event } = await requestsAndOffers.markWorkComplete(
  commitment.id,
  'marcus-id',
  { links: ['figma.com/...'] }
);

// 7. Sarah pays
const { settlement } = await requestsAndOffers.settlePayment(
  match.id,
  { amount: { value: 1500, unit: 'EUR' }, mediumId: 'EUR' }
);

// Result:
// - Full audit trail in EconomicService
// - Marcus credited
// - Sarah debited (or committed)
// - Reputation flows created
// - Everything immutable & transparent
```

---

## Integration with Elohim Mutual

This system can coordinate work for:
- **Adjusters** - Processing insurance claims
- **Prevention Specialists** - Risk mitigation coordination
- **Arbiters** - Dispute resolution
- **Governance Members** - Committee work

All with economic tracking and reputation building.

---

## Implementation Status

✅ Models defined and typed (1,400+ lines)
✅ Service stubs created (1,200+ lines)
✅ Documentation complete
⏳ Phase 1b - Service implementation (coming next)

---

## Next Steps

1. **Phase 1b** - Implement core methods:
   - Request/offer CRUD
   - Search and discovery
   - Matching algorithm

2. **Phase 2** - Implement coordination:
   - Proposal & agreement
   - Work completion
   - Payment settlement

3. **Phase 3** - Advanced features:
   - Reputation system
   - Analytics
   - Admin interface

---

**Version:** 1.0 (Phase 1)
**Status:** Models Complete - Ready for Implementation
**Last Updated:** December 22, 2025

