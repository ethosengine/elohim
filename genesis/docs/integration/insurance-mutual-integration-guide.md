# Elohim Mutual Integration Guide
## How Phase 1 Models Work with Shefa REA/ValueFlows Infrastructure

**Status:** Phase 1 Implementation Reference
**Date:** 2025-12-22

---

## Table of Contents
1. Architecture Overview
2. Core Workflows with Code Examples
3. Event Trail for Governance
4. Integration Points with Existing Services
5. Implementation Priorities

---

## Architecture Overview

### The Stack

```
┌─────────────────────────────────────────────────────────────┐
│  Insurance Mutual (New Layer)                               │
│  - MemberRiskProfile                                        │
│  - CoveragePolicy / CoveredRisk                            │
│  - InsuranceClaim / AdjustmentReasoning                    │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│  Shefa Economic Infrastructure (Existing)                   │
│  - EconomicEvent (immutable ledger)                        │
│  - CommonsPool (reserves)                                  │
│  - ValueAttribution / AttributionClaim (member claims)     │
│  - Commitment/Agreement (coverage terms)                   │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│  Supporting Systems                                         │
│  - Observer Protocol (risk assessment)                     │
│  - Lamad Steward Economy (premium gates)                   │
│  - Qahal Governance (coverage decisions)                   │
└─────────────────────────────────────────────────────────────┘
```

### Key Design Pattern: Immutability Through Events

**Critical Rule:** All state changes create `EconomicEvent` entries. Never mutate models directly.

```typescript
// ❌ WRONG: Direct mutation
memberRiskProfile.riskScore = 75;

// ✅ RIGHT: Create event that represents the state change
const assessmentEvent: EconomicEvent = {
  action: 'deliver-service',
  provider: 'elohim-assessor',
  receiver: memberId,
  resourceClassifiedAs: ['stewardship'],
  hasPointInTime: new Date().toISOString(),
  metadata: {
    lamadEventType: 'risk-reduction-verified',
    newRiskScore: 75,
    previousRiskScore: 82,
    riskTierChange: 'high → standard',
  },
};
// Then update model with reference to event
memberRiskProfile.assessmentEventIds.push(assessmentEvent.id);
```

---

## Core Workflows with Code Examples

### Workflow 1: Member Enrollment

**Scenario:** New member (household) wants to join community health mutual.

**Steps:**
1. Observer Protocol gathers baseline risk data
2. System assesses initial risk profile
3. Determine coverage (community level via Qahal decision)
4. Record enrollment as immutable event

**Code Example:**

```typescript
// In InsuranceMutualService.enrollMember()

async enrollMember(
  memberId: string,
  qahalId: string,
  initialRiskFactors: Partial<MemberRiskProfile>
): Promise<{ riskProfile: MemberRiskProfile; policy: CoveragePolicy; enrollmentEvent: EconomicEvent }> {
  // Step 1: Assess initial risk (pulls Observer attestations)
  const riskProfile = await this.assessMemberRisk(
    memberId,
    'health'
  );

  // Step 2: Determine coverage (from Qahal's coverage constitution)
  const qahalCoverage = await this.getQahalCoverageTemplate(qahalId);

  const policy: CoveragePolicy = {
    id: generateId(),
    memberId,
    coverageLevel: 'community',
    governedAt: 'qahal',
    coveredRisks: qahalCoverage.defaultRisks,
    deductible: qahalCoverage.deductible,
    coinsurance: qahalCoverage.coinsurance,
    outOfPocketMaximum: qahalCoverage.outOfPocketMaximum,
    effectiveFrom: new Date().toISOString(),
    constitutionalBasis: qahalCoverage.constitutionCitation,
    createdAt: new Date().toISOString(),
    lastModifiedAt: new Date().toISOString(),
    modificationEventIds: [],
  };

  // Step 3: Create enrollment event (immutable record)
  const enrollmentEvent: EconomicEvent = {
    id: generateId(),
    action: 'deliver-service',
    provider: 'elohim-mutual',
    receiver: memberId,
    resourceClassifiedAs: ['stewardship', 'membership'],
    hasPointInTime: new Date().toISOString(),
    state: 'validated',
    note: `${memberId} enrolled in ${qahalId} health mutual. Risk tier: ${riskProfile.riskTier}`,
    metadata: {
      lamadEventType: 'coverage-decision',
      riskProfileId: riskProfile.id,
      policyId: policy.id,
      riskScore: riskProfile.riskScore,
    },
  };

  // Step 4: Record via EconomicService
  const recordedEvent = await this.economicService.createEvent(enrollmentEvent);

  // Step 5: Link policy to event
  policy.createdAt = recordedEvent.hasPointInTime;
  riskProfile.assessmentEventIds = [recordedEvent.id];

  return { riskProfile, policy, enrollmentEvent: recordedEvent };
}
```

**Result:** Member is now in the mutual with:
- Risk assessment recorded in EconomicEvent
- Coverage policy linked to governance layer
- Full audit trail of enrollment

---

### Workflow 2: Premium Collection

**Scenario:** Monthly premium collection flows to mutual reserves.

**Uses:** `PremiumGate` (steward-economy.model.ts) + `CommonsPool` (rea-bridge.model.ts)

**Steps:**
1. Premium due (subscription based)
2. Member pays via mutual credit
3. Three-way split: adjuster compensation, commons pool, network sustainability
4. Creates economic events for each flow

**Code Example:**

```typescript
// In InsuranceMutualService.recordPremiumPayment()

async recordPremiumPayment(
  memberId: string,
  amount: Measure,
  paymentMethod: 'mutual-credit' | 'fiat-transfer',
  periodCovered: { from: string; to: string }
): Promise<{ paymentEvent: EconomicEvent; updatedPolicy: CoveragePolicy }> {
  const policy = await this.getCoveragePolicy(memberId);

  // Calculate premium split using PremiumGate system
  const premiumGate = await this.getPremiumGateForCoverage(policy.id);

  const split = {
    grossAmount: amount.hasNumericalValue,
    stewardShare: amount.hasNumericalValue * (premiumGate.stewardSharePercent / 100),
    commonsShare: amount.hasNumericalValue * (premiumGate.commonsSharePercent / 100),
    contributorShare: amount.hasNumericalValue * (premiumGate.contributorSharePercent / 100),
  };

  // Event 1: Premium payment (member → mutual)
  const premiumEvent: EconomicEvent = {
    id: generateId(),
    action: 'transfer',
    provider: memberId,
    receiver: 'elohim-mutual-pool',
    resourceClassifiedAs: ['currency'],
    resourceQuantity: amount,
    hasPointInTime: new Date().toISOString(),
    state: 'validated',
    realizationOf: policy.id,
    metadata: {
      lamadEventType: 'premium-payment',
      policyId: policy.id,
      paymentMethod,
      periodCovered,
    },
  };

  const recordedPremium = await this.economicService.createEvent(premiumEvent);

  // Event 2: Steward compensation (mutual → adjusters pool)
  const stewardEvent: EconomicEvent = {
    ...premiumEvent,
    id: generateId(),
    action: 'transfer',
    provider: 'elohim-mutual-pool',
    receiver: 'adjusters-guild', // REAAgent representing steward collective
    resourceQuantity: { hasNumericalValue: split.stewardShare, hasUnit: amount.hasUnit },
    metadata: {
      ...premiumEvent.metadata,
      source: 'premium-split',
      splitPercentage: premiumGate.stewardSharePercent,
    },
  };

  const recordedSteward = await this.economicService.createEvent(stewardEvent);

  // Event 3: Commons pool contribution (mutual → reserves)
  const commonsEvent: EconomicEvent = {
    ...premiumEvent,
    id: generateId(),
    action: 'transfer',
    provider: 'elohim-mutual-pool',
    receiver: 'health-claims-reserve', // CommonsPool ID
    resourceQuantity: { hasNumericalValue: split.commonsShare, hasUnit: amount.hasUnit },
    metadata: {
      ...premiumEvent.metadata,
      source: 'premium-split',
      commonsPoolId: 'health-claims-reserve',
    },
  };

  const recordedCommons = await this.economicService.createEvent(commonsEvent);

  // Event 4: Network sustainability (mutual → ecosystem)
  const networkEvent: EconomicEvent = {
    ...premiumEvent,
    id: generateId(),
    action: 'transfer',
    provider: 'elohim-mutual-pool',
    receiver: 'network-development-fund',
    resourceQuantity: { hasNumericalValue: split.contributorShare, hasUnit: amount.hasUnit },
    metadata: {
      ...premiumEvent.metadata,
      source: 'premium-split',
    },
  };

  const recordedNetwork = await this.economicService.createEvent(networkEvent);

  // Update CommonsPool (add to reserves)
  await this.updateCommonsPool('health-claims-reserve', {
    balance: {
      hasNumericalValue: commonsPool.balance.hasNumericalValue + split.commonsShare,
      hasUnit: amount.hasUnit,
    },
  });

  // Update policy
  policy.lastPremiumEventId = recordedPremium.id;
  policy.lastModifiedAt = new Date().toISOString();
  policy.modificationEventIds.push(recordedPremium.id);

  return {
    paymentEvent: recordedPremium,
    updatedPolicy: policy,
  };
}
```

**Result:** Full transparency on where premium dollars flow:
- Premiums recorded as single transfer event (immutable)
- Three separate events show three-way split
- CommonsPool grows with reserves
- Steward and network funding is visible

---

### Workflow 3: Claims Processing - The Bob Parr Test

**Scenario:** Member has emergency room visit, files claim, adjuster makes determination.

**The Principle:** "Can this adjuster explain decision in plain language? Would governance approve it?"

**Steps:**
1. Member files claim with Observer evidence
2. Adjuster investigates
3. Adjuster makes determination with full reasoning
4. Create `AdjustmentReasoning` (full constitutional basis)
5. Flag for governance review if needed
6. Settle claim (create AttributionClaim against pool)

**Code Example:**

```typescript
// Step 1: File claim
async fileClaim(memberId, policyId, lossDetails) {
  const claim: InsuranceClaim = {
    id: generateId(),
    claimNumber: `INS-${new Date().getFullYear()}-${claimSequence++}`,
    memberId,
    policyId,
    coveredRiskId: lossDetails.coveredRiskId,
    lossType: lossDetails.lossType,
    lossDate: lossDetails.lossDate,
    reportedDate: new Date().toISOString(),
    description: lossDetails.description,
    memberEstimatedLossAmount: lossDetails.estimatedLossAmount,
    observerAttestationIds: lossDetails.observerAttestationIds,
    memberDocumentIds: lossDetails.memberDocumentIds,
    status: 'reported',
    eventIds: { filedEventId: '' },
    createdAt: new Date().toISOString(),
    lastUpdatedAt: new Date().toISOString(),
  };

  // Create immutable claim-filed event
  const claimEvent: EconomicEvent = {
    action: 'deliver-service',
    provider: memberId,
    receiver: 'elohim-mutual',
    resourceClassifiedAs: ['stewardship'],
    hasPointInTime: claim.reportedDate,
    metadata: {
      lamadEventType: 'claim-filed',
      claimId: claim.id,
      lossType: claim.lossType,
      estimatedAmount: claim.memberEstimatedLossAmount,
    },
  };

  claim.eventIds.filedEventId = (await this.economicService.createEvent(claimEvent)).id;
  return { claim, filedEvent: claimEvent };
}

// Step 2: Assign to adjuster
await assignClaimToAdjuster(claimId, adjusterId);

// Step 3: Adjuster determines claim (with full reasoning)
async adjustClaim(
  claimId: string,
  adjusterId: string,
  reasoning: Omit<AdjustmentReasoning, 'id' | 'claimId' | 'createdAt'>
) {
  const claim = await this.getClaim(claimId);
  const policy = await this.getCoveragePolicy(claim.memberId);

  // Key: adjuster MUST cite the coverage policy (constitutional basis)
  if (!reasoning.constitutionalCitation.includes(policy.id)) {
    throw new Error('Adjuster must cite coverage policy being applied');
  }

  // Create full reasoning object (auditable)
  const fullReasoning: AdjustmentReasoning = {
    id: generateId(),
    claimId,
    adjusterId,
    adjustmentDate: new Date().toISOString(),
    constitutionalCitation: reasoning.constitutionalCitation,
    citedText: getCitedPolicySectionText(reasoning.constitutionalCitation),
    plainLanguageExplanation: reasoning.plainLanguageExplanation,
    materialFacts: reasoning.materialFacts,
    alternativesConsidered: reasoning.alternativesConsidered,
    generosityInterpretationApplied: reasoning.generosityInterpretationApplied,
    generosityExplanation: reasoning.generosityExplanation,
    determinations: reasoning.determinations,
    auditableDecisionLog: buildDecisionLog(reasoning),
    createdAt: new Date().toISOString(),
  };

  // Flag for governance review if any red flags
  fullReasoning.flaggedForGovernanceReview = shouldFlagForReview(
    fullReasoning,
    adjusterId
  );

  // Create claim-adjusted event
  const adjustedEvent: EconomicEvent = {
    action: 'deliver-service',
    provider: adjusterId,
    receiver: claim.memberId,
    resourceClassifiedAs: ['stewardship'],
    hasPointInTime: fullReasoning.adjustmentDate,
    metadata: {
      lamadEventType: 'claim-adjusted',
      claimId,
      approvedAmount: fullReasoning.determinations.finalApprovedAmount,
      reasoning: {
        constitutionalCitation: fullReasoning.constitutionalCitation,
        plainLanguage: fullReasoning.plainLanguageExplanation,
        generosityApplied: fullReasoning.generosityInterpretationApplied,
      },
    },
  };

  const recordedEvent = await this.economicService.createEvent(adjustedEvent);

  // Update claim
  claim.adjustmentReasoning = fullReasoning;
  claim.approvedAmount = fullReasoning.determinations.finalApprovedAmount;
  claim.status = 'adjustment-made';
  claim.eventIds.adjustedEventId = recordedEvent.id;

  return { claim, reasoning: fullReasoning, adjustedEvent: recordedEvent };
}

// Step 4: Settle claim (if approved)
async settleClaim(
  claimId: string,
  settledAmount: Measure,
  paymentMethod: 'mutual-credit' | 'fiat-transfer'
): Promise<{ claim: InsuranceClaim; settlementEvent: EconomicEvent; attribution: AttributionClaim }> {
  const claim = await this.getClaim(claimId);

  // Create AttributionClaim against CommonsPool
  // This is member's entitlement to payment
  const attribution: AttributionClaim = {
    id: generateId(),
    attributionId: generateId(), // Links to ValueAttribution in pool
    claimantId: claim.memberId,
    amount: settledAmount,
    requiredAttestationLevel: 'basic',
    identityVerified: true,
    responsibilityVerified: true,
    state: 'approved',
    submittedAt: new Date().toISOString(),
    processedAt: new Date().toISOString(),
  };

  // Create claim-settled event
  // This is the actual transfer from pool to member
  const settlementEvent: EconomicEvent = {
    action: 'transfer',
    provider: 'health-claims-reserve', // CommonsPool
    receiver: claim.memberId,
    resourceClassifiedAs: ['currency'],
    resourceQuantity: settledAmount,
    hasPointInTime: new Date().toISOString(),
    fulfills: [claimId], // This event fulfills the claim
    state: 'validated',
    signatures: [
      { signerId: adjusterId, role: 'provider' },
      { signerId: observerNetworkId, role: 'witness' }, // Observer attested the loss
      { signerId: claim.memberId, role: 'receiver' },
    ],
    metadata: {
      lamadEventType: 'claim-settled',
      claimId,
      adjustmentReasoningId: claim.adjustmentReasoning?.id,
      paymentMethod,
    },
  };

  const recordedEvent = await this.economicService.createEvent(settlementEvent);

  // Deduct from CommonsPool
  const pool = await this.getCommonsPool('health-claims-reserve');
  pool.balance.hasNumericalValue -= settledAmount.hasNumericalValue;
  pool.lastActivityAt = recordedEvent.hasPointInTime;

  // Update claim
  claim.status = 'settled';
  claim.settledAt = recordedEvent.hasPointInTime;
  claim.paidAmount = settledAmount;
  claim.paymentReference = recordedEvent.id;
  claim.eventIds.settledEventId = recordedEvent.id;

  return { claim, settlementEvent: recordedEvent, attribution };
}
```

**Key Bob Parr Features:**
1. **Constitutional citation** - Every decision must cite coverage policy
2. **Plain language** - Member reads plain language explanation, not policy jargon
3. **Generosity principle** - When ambiguous, decision resolves toward member welfare
4. **Auditable** - Full decision log recorded for governance review
5. **Flagged if concerning** - Large claims, unusual patterns trigger review

---

### Workflow 4: Governance Review (The Safety Net)

**Scenario:** Governance committee reviews questionable claim decision.

**Ensures:** Adjusters can't extract value by denying valid claims.

**Code Example:**

```typescript
// Get flagged claims for review
const flaggedClaims = await insuranceMutual.getFlaggedClaims(qahalId);

// Committee members review reasoning
for (const flagged of flaggedClaims) {
  console.log(`Claim ${flagged.claim.claimNumber}`);
  console.log(`Adjuster: ${flagged.reasoning.adjusterId}`);
  console.log(`Reason flagged: ${flagged.flagReason}`);
  console.log(`Decision: ${flagged.reasoning.determinations.finalApprovedAmount}`);
  console.log(`Reasoning: ${flagged.reasoning.plainLanguageExplanation}`);
  console.log(`Generosity applied? ${flagged.reasoning.generosityInterpretationApplied}`);
}

// Committee decides: upheld, modified, or overturned
const updatedClaim = await insuranceMutual.resolveAppeal(
  claimId,
  reviewingAdjusterId, // Different adjuster
  'modified', // Changed decision
  'Committee modified to full coverage. Loss was clearly covered under dignity-floor principle.'
);

// This creates ANOTHER event in the chain
// Full history visible: original decision + review decision
```

**Result:** Pattern analysis becomes possible:
- Identify adjusters who consistently deny legitimate claims
- Spot bias (geographic, demographic) in decisions
- Ensure constitutional constraints are enforced
- Improve coverage definitions based on governance review

---

## Event Trail for Governance

### Complete Claim Lifecycle in Events

Every claim creates a chain of immutable events:

```
1. claim-filed (member reports loss)
   ↓ [stored in ClaimEventIds.filedEventId]

2. claim-evidence-submitted (member uploads docs)
   ↓ [if member submits additional evidence]

3. claim-investigated (adjuster gathers info)
   ↓ [stored in ClaimEventIds.investigatedEventId]

4. claim-adjusted (adjuster determines coverage)
   ↓ [stored in ClaimEventIds.adjustedEventId]
   → Includes AdjustmentReasoning (full constitutional reasoning)

5. claim-review-initiated (IF governance flagged)
   ↓ [optional - only if flagged for review]

6. claim-appealed (IF member appeals)
   ↓ [optional - if member disputes]

7. claim-settled (claim is paid)
   ↓ [stored in ClaimEventIds.settledEventId]
   → Creates AttributionClaim against CommonsPool
   → Member receives settlement
```

**Governance uses this chain to answer:**
- "Is adjuster's decision constitutional?"
- "Are we denying legitimate claims?"
- "Is there pattern bias in decisions?"
- "Are cost-sharing terms being applied fairly?"

---

## Integration Points with Existing Services

### 1. EconomicService Integration

```typescript
// From InsuranceMutualService
import { EconomicService } from './economic.service';

constructor(private economicService: EconomicService) {}

// Every operation creates events via EconomicService
const event = await this.economicService.createEvent({
  action: 'transfer',
  provider: 'health-claims-reserve',
  receiver: memberId,
  resourceClassifiedAs: ['currency'],
  // ... etc
});
```

**What EconomicService Provides:**
- `createEvent()` - Record immutable event
- `queryEvents()` - Audit trail queries
- `getAgentLedger()` - Agent's perspective on their flows

---

### 2. CommonsPool Integration

```typescript
// Get pool status
const pool = await this.getCommonsPool('health-claims-reserve');
console.log(`Balance: ${pool.balance}`);
console.log(`Attributed but unclaimed: ${pool.attributedUnclaimed}`);

// Update pool when claim settles
pool.balance.hasNumericalValue -= settlement.hasNumericalValue;

// Track adequacy for regulators
const adequacyRatio = pool.balance / expectedAnnualClaims;
```

**What CommonsPool Provides:**
- Central holding account for reserves
- Tracks attribution (value sitting in pool for absent contributors)
- Enables graduated claiming for members

---

### 3. PremiumGate Integration (Steward Economy)

```typescript
// From InsuranceMutualService.recordPremiumPayment()

// Get the premium gate for coverage
const premiumGate = await this.getPremiumGateForCoverage(policy.id);

// Use three-way split
const split = {
  stewardShare: amount * premiumGate.stewardSharePercent,
  commonsShare: amount * premiumGate.commonsSharePercent,
  contributorShare: amount * premiumGate.contributorSharePercent,
};

// Creates StewardRevenue records (existing Lamad pattern)
// Which splits flows among: adjusters, commons pool, network
```

**What PremiumGate Provides:**
- Revenue model (one-time, subscription, pay-what-you-can)
- Three-way split configuration
- Scholarship support (if needed)
- Access grant tracking

---

### 4. Observer Protocol Integration

```typescript
// From InsuranceMutualService.assessMemberRisk()

// Query Observer attestations for member
const careMaintenance = await this.queryObserverAttestations({
  agentId: memberId,
  attestationType: 'preventive-care-completed',
  fromDate: oneYearAgo,
});

// Score based on actual behavior
const careScore = (careMaintenance.count / 12) * 100; // percentage of months active

// Not proxy data (credit scores) - actual observed behavior
const connectednessScore = await this.calculateCommunityConnectedness(memberId);
```

**What Observer Provides:**
- Cryptographic attestations of observed behavior
- Evidence for risk assessment
- Claims verification (loss actually occurred)
- Prevention incentive tracking

---

### 5. Qahal Governance Integration

```typescript
// Coverage decisions flow to governance layers

// Individual layer
async updateCoveragePolicy(memberId, newTerms, 'individual') {
  // Member decides their own deductible/coinsurance
}

// Community layer
async updateCoveragePolicy(householdId, newTerms, 'household') {
  // Household decides what risks to pool locally
}

// Community layer
async updateCoveragePolicy(qahalId, newTerms, 'community') {
  // Qahal decides what coverage for all members
  // Must reference Qahal's coverage constitution
}

// Constitutional layer (can't be opted out of)
async updateCoveragePolicy(memberId, dignityFloor, 'constitutional') {
  // Dignity floor coverage - mandatory for all members
}
```

---

## Implementation Priorities

### Phase 1a: Core Models (DONE ✓)
- [x] MemberRiskProfile
- [x] CoveragePolicy + CoveredRisk
- [x] InsuranceClaim
- [x] AdjustmentReasoning
- [x] Insurance event types

### Phase 1b: Service Implementation (TODO)
**Priority 1 (Required for MVP):**
1. `enrollMember()` - Get members into system
2. `assessMemberRisk()` - Risk-based premium
3. `fileClaim()` - Members can file claims
4. `adjustClaim()` - Adjuster determines coverage
5. `settleClaim()` - Pay members
6. `recordPremiumPayment()` - Collect premiums

**Priority 2 (Required for operation):**
7. `calculatePremium()` - Premium calculation
8. `getReserveStatus()` - Regulatory reporting
9. `flagClaimForGovernanceReview()` - Governance oversight

**Priority 3 (Enhancement):**
10. `recordRiskMitigation()` - Prevention incentives
11. `appealClaimDecision()` - Appeal process
12. `getAdjusterMetrics()` - Performance tracking

### Phase 1c: Integration (TODO)
1. Wire service to EconomicService
2. Connect to CommonsPool updates
3. Integrate with Observer protocol queries
4. Hook into Qahal governance decisions

### Phase 2: Adjuster Model Extension (TODO)
- Extend REAAgent with adjuster-specific fields
- Track performance metrics
- Governance review patterns
- Qualification tiers

### Phase 3: Household & Reserve Models (TODO)
- MemberHousehold for family pooling
- MutualReserveAccount for regulatory compliance
- ReinsuranceContract for legacy insurance partners

---

## Key Takeaways

1. **Models sit on top of REA foundation** - Don't rebuild economic layer; extend it
2. **Everything is an event** - Immutability through events, not state mutation
3. **Constitutional transparency** - Every claim decision must cite governance basis
4. **Audit trail for governance** - Committee can review adjuster decisions
5. **Information asymmetry flip** - Use Observer for actual risk, not proxies
6. **Prevention-oriented** - System rewards risk mitigation, not denial

---

## Next Steps

1. Review and approve Phase 1 models ✓ (DONE)
2. Implement core service methods (Priority 1)
3. Test with prototype claims (emergency room visit scenario)
4. Integrate with EconomicService and CommonsPool
5. Design Adjuster qualification model
6. Plan governance committee interface

