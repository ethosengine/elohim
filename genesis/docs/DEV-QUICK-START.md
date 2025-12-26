# Elohim Mutual - Developer Quick Start

## Getting Oriented

### What Was Built
Phase 1 delivers **domain models + service stubs** for autonomous mutual insurance.

**Files to know:**
- `elohim-app/src/app/shefa/models/insurance-mutual.model.ts` - Data models
- `elohim-app/src/app/shefa/services/insurance-mutual.service.ts` - Service stubs
- `docs/integration/insurance-mutual-integration-guide.md` - How it integrates
- `docs/analysis/shefa-models-for-mutual-review.md` - Why this design

### The Big Picture
```
Insurance Mutual (new models)
    ↓
Shefa REA/ValueFlows (existing - immutable event ledger)
    ↓
Observer Protocol + Qahal Governance (existing services)
```

All state changes create immutable `EconomicEvent` entries.

---

## Models Overview

### 5 Core Domain Models

**1. MemberRiskProfile**
- Actual behavioral risk (not credit scores)
- Care maintenance, community connectedness, claims history
- Observer attestation trail
- Risk trending

**2. CoveragePolicy + CoveredRisk**
- What member is covered for
- Cost-sharing: deductible, coinsurance, OOP max
- Constitutional basis (links to governance documents)
- Prevention incentives

**3. InsuranceClaim**
- Full claim lifecycle
- Observer evidence
- REA event integration

**4. AdjustmentReasoning**
- Full constitutional reasoning for every decision
- Plain language explanation
- Audit trail for governance
- Generosity principle tracking

**5. 14 New Event Types**
- premium-payment, claim-filed, claim-adjusted, claim-settled, etc.
- Integrated into LamadEventType
- Mapped to REA actions (transfer, deliver-service, work, etc.)

---

## Implementation Priorities

### Priority 1: Get Members → Claims → Paid (Required for MVP)

```typescript
// 1. Member joins
enrollMember(memberId, qahalId, riskFactors)
  → Creates MemberRiskProfile + CoveragePolicy
  → Records 'coverage-decision' event

// 2. Member files claim
fileClaim(memberId, lossDetails)
  → Creates InsuranceClaim (status: 'reported')
  → Records 'claim-filed' event

// 3. Adjuster determines coverage
adjustClaim(claimId, adjusterId, reasoning)
  → Creates AdjustmentReasoning (full constitutional basis)
  → Records 'claim-adjusted' event
  → May flag for governance review

// 4. Settle claim (pay member)
settleClaim(claimId, settledAmount)
  → Creates AttributionClaim (member's entitlement)
  → Records 'claim-settled' event
  → Deducts from CommonsPool

// 5. Member pays premium
recordPremiumPayment(memberId, amount, period)
  → Creates 'premium-payment' event
  → Three-way split: adjuster, commons, network
  → Updates CommonsPool reserves
```

### Priority 2: Governance & Oversight

```typescript
// Adjuster decisions are reviewable
flagClaimForGovernanceReview(claimId, reason)
getFlaggedClaims(qahalId)
resolveAppeal(claimId, reviewingAdjusterId, decision)

// Track adjuster performance
getAdjusterMetrics(adjusterId)
  → Return: claimsProcessed, denialRate, appealRate,
  → constitutionalCompliance, generosityIndex
```

### Priority 3: Prevention & Analytics

```typescript
// Reward risk mitigation
recordRiskMitigation(memberId, observerAttestationId)
  → Updates MemberRiskProfile (score improves)
  → Records 'prevention-incentive-awarded' event
  → Premium adjusts next period

// Reserve/loss analytics
getReserveStatus(poolId)
analyzeClaimsTrends(poolId, period)
getMemberStatement(memberId)
getQahalAnalytics(qahalId)
```

---

## Critical Implementation Rules

### Rule 1: Immutability Through Events

**❌ WRONG:**
```typescript
claim.status = 'settled';
```

**✅ RIGHT:**
```typescript
// Create event first
const event = await this.economicService.createEvent({
  action: 'transfer',
  provider: 'health-claims-reserve',
  receiver: memberId,
  resourceClassifiedAs: ['currency'],
  hasPointInTime: new Date().toISOString(),
  metadata: {
    lamadEventType: 'claim-settled',
    claimId: claimId,
  },
});

// Then update model with event reference
claim.status = 'settled';
claim.settledEventId = event.id;
claim.paidAmount = settledAmount;
claim.settledAt = event.hasPointInTime;
```

### Rule 2: Constitutional Basis Required

Every claim determination must cite coverage policy:

```typescript
// ❌ WRONG: Deny without citing why
await adjustClaim(claimId, adjusterId, {
  determinations: { coverageApplies: false }
});

// ✅ RIGHT: Deny with full constitutional reasoning
await adjustClaim(claimId, adjusterId, {
  constitutionalCitation: 'coverage-policy-123.md, section 4.2',
  plainLanguageExplanation: 'This loss is not covered because...',
  determinations: { coverageApplies: false },
  auditableDecisionLog: 'Full analysis here...',
});
```

### Rule 3: Observer Evidence for Risk & Claims

```typescript
// Risk assessment uses Observer attestations
const profile = {
  evidenceEventIds: ['obs-12345', 'obs-12346'], // Observer proofs
  careMaintenanceScore: 85, // From preventive behaviors observed
};

// Claims use Observer evidence too
const claim = {
  observerAttestationIds: ['obs-proof-loss'], // Loss was witnessed
  memberDocumentIds: ['doc-receipt'], // Supporting docs
};
```

### Rule 4: Governance Flagging

Flag claims that need governance review:

```typescript
// Flag if:
// - Large claims (above threshold)
// - Unusual interpretation (ambiguous coverage resolved for member)
// - Pattern concerns (adjuster's denial rate increasing)
// - Denial of seemingly covered claim

flagClaimForGovernanceReview(claimId, 'large-claim', 'Claim exceeds $10k');
flagClaimForGovernanceReview(claimId, 'unusual-interpretation', 'Applied generosity principle');
```

### Rule 5: Premium Three-Way Split

Using existing PremiumGate pattern:

```typescript
const premiumGate = {
  stewardSharePercent: 10,    // → Adjuster compensation pool
  commonsSharePercent: 85,    // → Claims reserve (CommonsPool)
  contributorSharePercent: 5, // → Network sustainability
};

// Creates three separate events
event1: transfer to adjusters-guild
event2: transfer to health-claims-reserve (CommonsPool)
event3: transfer to network-development-fund

// All together sum to 100%
```

---

## Common Workflows

### Filing a Claim
```typescript
// Member files claim
const { claim, filedEvent } = await insuranceMutual.fileClaim(
  memberId,
  policyId,
  {
    lossType: 'emergency-health',
    lossDate: '2025-12-15',
    description: 'Emergency room visit',
    estimatedLossAmount: { hasNumericalValue: 8500, hasUnit: 'unit-token' },
    observerAttestationIds: ['obs-proof'], // Evidence it happened
    memberDocumentIds: ['receipt-123'],     // Billing
  }
);

// Claim is now:
// - status: 'reported'
// - stored in database
// - immutable event created
// - ready for adjuster assignment
```

### Adjusting a Claim

```typescript
// Adjuster determines claim
const { claim, reasoning, adjustedEvent } = await insuranceMutual.adjustClaim(
  claimId,
  adjusterId,
  {
    constitutionalCitation: 'coverage-policy-123.md, section 3.2',
    plainLanguageExplanation: `You have full coverage for emergency room
      visits. Your $500 deductible was met on January 15. We reviewed your
      ER bill and verified $8,500 in covered costs. You are approved for
      full payment.`,
    materialFacts: [
      'Member had active coverage on loss date',
      'Loss date: 12/15/2025',
      'Deductible was already met',
      'Emergency room is covered 100%',
    ],
    generosityInterpretationApplied: false,
    determinations: {
      coverageApplies: true,
      policyCoverageAmount: { hasNumericalValue: 8500, hasUnit: 'unit-token' },
      deductibleApplied: { hasNumericalValue: 0, hasUnit: 'unit-token' },
      finalApprovedAmount: { hasNumericalValue: 8500, hasUnit: 'unit-token' },
    },
    auditableDecisionLog: 'Full reasoning trail...',
  }
);

// Claim is now:
// - status: 'adjustment-made'
// - approved for settlement
// - AdjustmentReasoning stored
// - Ready to pay member
```

### Settling a Claim

```typescript
// Pay the member
const { claim, settlementEvent, attribution } = await insuranceMutual.settleClaim(
  claimId,
  { hasNumericalValue: 8500, hasUnit: 'unit-token' },
  'mutual-credit'
);

// This creates:
// - 'claim-settled' event (transfer from pool to member)
// - AttributionClaim (member's entitlement)
// - CommonPool deduction (reserves go down)
// - Payment reference (transaction ID)
```

### Governance Reviews

```typescript
// Get claims waiting for governance review
const flagged = await insuranceMutual.getFlaggedClaims(qahalId);

// Each flagged claim includes:
// - Full reasoning from adjuster
// - Why it was flagged
// - Plain language explanation
// - Decision log for analysis

// Committee decides
await insuranceMutual.resolveAppeal(
  claimId,
  reviewingAdjusterId,
  'modified', // upheld | overturned | modified
  'Committee modified to 100% coverage. Loss was clearly within dignity-floor protection.'
);
```

---

## Service Method Stubs to Implement

### Essential (get these working first)

```typescript
// Member lifecycle
enrollMember() ← Start here
assessMemberRisk()
getCoveragePolicy()

// Claims lifecycle
fileClaim()
adjustClaim()
settleClaim()
getClaim()

// Premium collection
recordPremiumPayment()
calculatePremium()

// Governance oversight
flagClaimForGovernanceReview()
getFlaggedClaims()
resolveAppeal()

// Analytics
getReserveStatus()
getMemberStatement()
```

### Each method should:
1. Validate inputs
2. Query existing data (policy, risk profile, etc)
3. Create or update models
4. **Create immutable EconomicEvent entry**
5. Update CommonsPool if money involved
6. Return result with event reference

---

## Testing Strategy

### Scenario: Emergency Room Visit (End-to-End)

```typescript
// Setup
const memberId = 'test-member-1';
const qahalId = 'community-health-pool';

// 1. Enroll member
const { riskProfile, policy } = await enrollMember(memberId, qahalId, {
  careMaintenanceScore: 85,
  communityConnectednessScore: 90,
});
assert(policy.deductible.hasNumericalValue === 500); // Community default
assert(riskProfile.riskTier === 'standard');

// 2. Get premium
const premium = await calculatePremium(memberId, 'community');
assert(premium.finalPremium.hasNumericalValue > 0);

// 3. Pay premium
const { paymentEvent } = await recordPremiumPayment(
  memberId,
  premium.finalPremium,
  'mutual-credit',
  { from: '2025-01', to: '2025-02' }
);
assert(paymentEvent.action === 'transfer');

// 4. File emergency room claim
const { claim } = await fileClaim(memberId, policy.id, {
  lossType: 'emergency-health',
  lossDate: '2025-01-15',
  estimatedLossAmount: { hasNumericalValue: 8500, hasUnit: 'unit-token' },
  observerAttestationIds: ['obs-proof'],
});
assert(claim.status === 'reported');

// 5. Adjuster determines claim
const { reasoning, adjustedEvent } = await adjustClaim(
  claim.id,
  'adjuster-1',
  {
    constitutionalCitation: policy.constitutionalBasis,
    plainLanguageExplanation: 'Full coverage. Approved.',
    determinations: {
      coverageApplies: true,
      finalApprovedAmount: { hasNumericalValue: 8500, hasUnit: 'unit-token' },
    },
  }
);
assert(reasoning.determinations.finalApprovedAmount.hasNumericalValue === 8500);

// 6. Settle claim
const { settlementEvent } = await settleClaim(claim.id, reason.determinations.finalApprovedAmount);
assert(settlementEvent.action === 'transfer');
assert(settlementEvent.receiver === memberId);

// 7. Check reserve deduction
const reserve = await getReserveStatus('health-claims-reserve');
assert(reserve.balance.hasNumericalValue < initialBalance);
```

---

## Key Files to Reference

| File | Purpose |
|------|---------|
| `insurance-mutual.model.ts` | TypeScript type definitions |
| `economic-event.model.ts` | Event types and mappings |
| `economic.service.ts` | How to create events |
| `integration-guide.md` | Detailed workflow examples |
| `shefa-models-review.md` | Architecture rationale |

---

## Common Pitfalls

### ❌ Pitfall 1: Forgetting the Event
```typescript
// WRONG: Model updated without event
claim.status = 'settled';
```

### ❌ Pitfall 2: No Constitutional Basis
```typescript
// WRONG: Adjuster approves without citing policy
adjustClaim(claimId, adjusterId, { coverageApplies: true });
```

### ❌ Pitfall 3: Ignoring Observer Evidence
```typescript
// WRONG: Risk score calculated without actual observation
riskScore = 50; // Made up
```

### ❌ Pitfall 4: No Governance Flag
```typescript
// WRONG: Large denial approved without governance review
if (claim > $50k) {
  settleClaim(claimId, 0); // Should flag for review!
}
```

### ❌ Pitfall 5: Skipping Plain Language
```typescript
// WRONG: Member gets policy jargon
explanation = 'Claim denied per subsection 4.3(b)(iv)';

// RIGHT: Member understands decision
explanation = 'Your claim for cosmetic work is not covered. The policy covers medical emergency care, not elective procedures.';
```

---

## Success Criteria for Phase 1

✅ Models defined and typed
✅ Event types added and mapped
✅ Service stubs created
✅ Integration patterns documented
✅ Example workflows shown

**Ready to build:** Service method implementations

---

## Questions?

Refer to:
1. `docs/integration/insurance-mutual-integration-guide.md` - Detailed examples
2. `docs/analysis/shefa-models-for-mutual-review.md` - Why this design
3. Models: Type definitions with extensive JSDoc comments

---

**Version:** 1.0
**Last Updated:** December 22, 2025
**Status:** Ready for Implementation

