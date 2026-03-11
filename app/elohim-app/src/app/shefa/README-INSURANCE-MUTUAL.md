# Shefa Insurance Mutual Module

## Overview

This module implements the **Elohim Mutual** - an autonomous mutual insurance entity built on the existing Shefa REA/ValueFlows economic infrastructure.

**Key Principle:** Insurance mutual is a specialized application of the REA/ValueFlows model. It uses:
- **EconomicEvent** as the immutable ledger
- **CommonsPool** for risk reserves
- **AttributionClaim** for member claims against pool
- **Commitment/Agreement** for coverage terms

## Module Structure

```
shefa/
├── models/
│   └── insurance-mutual.model.ts
│       ├── MemberRiskProfile          (behavioral risk assessment)
│       ├── CoveragePolicy + CoveredRisk (what's covered)
│       ├── InsuranceClaim             (claim processing)
│       └── AdjustmentReasoning        (decision transparency)
│
└── services/
    └── insurance-mutual.service.ts
        ├── Member enrollment & risk
        ├── Coverage management
        ├── Claims processing
        ├── Claims adjudication
        ├── Appeals & disputes
        ├── Prevention & risk mitigation
        ├── Governance & oversight
        ├── Reserves & actuarial
        ├── Premium management
        └── Reporting & transparency
```

## Core Models

### 1. MemberRiskProfile

Actual behavioral risk assessment (not proxy data like credit scores).

```typescript
import { MemberRiskProfile } from '@app/shefa/models/insurance-mutual.model';

const profile: MemberRiskProfile = {
  memberId: 'member-123',
  riskType: 'health',
  careMaintenanceScore: 85,        // Preventive behaviors observed via Observer
  communityConnectednessScore: 90, // Support network strength
  historicalClaimsRate: 20,        // Past claims frequency
  riskScore: 68,                   // Composite (0-100)
  riskTier: 'standard',            // low | standard | high | uninsurable
  evidenceEventIds: ['obs-12345'], // Observer attestation proofs
  riskTrendDirection: 'stable',    // improving | stable | declining
};
```

**Key Features:**
- Uses actual behavioral observation (Observer protocol)
- Not dependent on proxy data (credit scores, demographics)
- Trend tracking for prevention engagement
- Evidence trail for governance review

---

### 2. CoveragePolicy + CoveredRisk

Defines what is covered for each member.

```typescript
import { CoveragePolicy, CoveredRisk } from '@app/shefa/models/insurance-mutual.model';

const policy: CoveragePolicy = {
  id: 'policy-123',
  memberId: 'member-123',
  coverageLevel: 'community',        // individual | household | community | network | constitutional
  governedAt: 'qahal',               // Which governance layer decided this

  // Cost-sharing terms
  deductible: { hasNumericalValue: 500, hasUnit: 'unit-token' },
  coinsurance: 20,                    // Member pays 20%, mutual pays 80%
  outOfPocketMaximum: { hasNumericalValue: 5000, hasUnit: 'unit-token' },

  // Covered risks
  coveredRisks: [
    {
      riskType: 'emergency-health',
      isCovered: true,
      coverage: { coveragePercent: 100 },
    },
    {
      riskType: 'preventive-health',
      isCovered: true,
      preventionIncentive: {
        discountPercent: 15,
        requiredAttestations: ['completed-annual-checkup'],
      },
    },
  ],

  // Constitutional basis
  constitutionalBasis: 'community-health-coverage-2024.md',
  constitutionalSection: 'section 3.2',
};
```

**Key Features:**
- Graduated governance (decisions flow to appropriate level)
- Constitutional basis citation (must reference governance document)
- Prevention incentives built in
- Cost-sharing terms define member responsibility

---

### 3. InsuranceClaim

Complete claim lifecycle from reporting to settlement.

```typescript
import { InsuranceClaim, InsuranceClaimStatus } from '@app/shefa/models/insurance-mutual.model';

const claim: InsuranceClaim = {
  id: 'claim-123',
  claimNumber: 'INS-2025-001234',  // Human-readable for member
  memberId: 'member-123',
  policyId: 'policy-123',

  // What happened
  lossType: 'emergency-health',
  lossDate: '2025-12-15',
  reportedDate: '2025-12-16',
  description: 'Emergency room visit for chest pain',

  // What it cost
  memberEstimatedLossAmount: { hasNumericalValue: 8500, hasUnit: 'unit-token' },
  determinedLossAmount: { hasNumericalValue: 8500, hasUnit: 'unit-token' },

  // Evidence
  observerAttestationIds: ['obs-proof-loss'],    // Observer witnessed loss
  memberDocumentIds: ['receipt-123', 'bill-456'], // Supporting docs

  // Processing state
  status: 'reported' as InsuranceClaimStatus,
  assignedAdjusterId: 'adjuster-1',
  adjustmentReasoning: { /* see below */ },

  // Settlement
  approvedAmount: { hasNumericalValue: 8500, hasUnit: 'unit-token' },
  settledAt: '2025-12-20',

  // REA integration (immutable event trail)
  eventIds: {
    filedEventId: 'event-123',
    investigatedEventId: 'event-124',
    adjustedEventId: 'event-125',
    settledEventId: 'event-126',
  },
};
```

**Claim Status Lifecycle:**
```
reported
  ↓
documented (evidence added)
  ↓
under-investigation (adjuster gathering info)
  ↓
pending-adjustment (adjuster reviewing)
  ↓
adjustment-made (adjuster determination)
  ↓
[approved OR denied]
  ↓
[IF disputed: appealed → appeal-resolved]
  ↓
ready-for-settlement
  ↓
settled (claim paid)
```

---

### 4. AdjustmentReasoning

**The Bob Parr safety net:** Full constitutional reasoning for every claim decision.

```typescript
import { AdjustmentReasoning } from '@app/shefa/models/insurance-mutual.model';

const reasoning: AdjustmentReasoning = {
  id: 'reason-123',
  claimId: 'claim-123',
  adjusterId: 'adjuster-1',
  adjustmentDate: '2025-12-19',

  // Constitutional basis (required)
  constitutionalCitation: 'coverage-policy-123.md, section 3.2',
  citedText: 'Emergency health: 100% coverage after deductible',

  // Plain language for member
  plainLanguageExplanation: `Your claim for $8,500 in emergency room costs
    is approved. You have full coverage for emergency health care under
    your policy. Your $500 deductible was already met this year on January 15.
    We verified the charges with the hospital. You are approved for payment
    of the full $8,500.`,

  // Analysis transparency
  materialFacts: [
    'Member had active coverage on loss date (12/15/2025)',
    'Emergency health is covered at 100% in policy',
    'Deductible ($500) was met 01/15/2025',
    'Charges verified with hospital',
    'Within policy limits',
  ],

  // Generosity principle (when ambiguous, resolve for member)
  generosityInterpretationApplied: false,
  generosityExplanation: undefined, // N/A - not ambiguous

  // The actual determination
  determinations: {
    coverageApplies: true,
    policyCoverageAmount: { hasNumericalValue: 8500, hasUnit: 'unit-token' },
    deductibleApplied: { hasNumericalValue: 0, hasUnit: 'unit-token' },
    coinsuranceApplied: { hasNumericalValue: 0, hasUnit: 'unit-token' },
    finalApprovedAmount: { hasNumericalValue: 8500, hasUnit: 'unit-token' },
  },

  // Auditability
  auditableDecisionLog: '... full step-by-step reasoning ...',
  flaggedForGovernanceReview: false,
};
```

**Key Features:**
- **Constitutional citation** - Must cite coverage policy
- **Plain language** - Member understands the decision
- **Generosity principle** - When ambiguous, resolve toward member welfare
- **Auditable** - Full reasoning for governance committee
- **Integrity check** - Track whether decision is logical, factual, consistent

---

## Service Usage

### Enrolling a Member

```typescript
import { InsuranceMutualService } from '@app/shefa/services/insurance-mutual.service';

constructor(private mutual: InsuranceMutualService) {}

async enrollNewMember(memberId: string, qahalId: string) {
  const { riskProfile, policy, enrollmentEvent } =
    await this.mutual.enrollMember(memberId, qahalId, {
      careMaintenanceScore: 85,
      communityConnectednessScore: 90,
    });

  console.log(`Member enrolled with risk tier: ${riskProfile.riskTier}`);
  console.log(`Coverage: ${policy.coverageLevel}`);
  console.log(`Event ID: ${enrollmentEvent.id}`);
}
```

### Filing a Claim

```typescript
async memberFilesClaim(memberId: string, lossDetails: any) {
  const { claim, filedEvent } = await this.mutual.fileClaim(
    memberId,
    policyId,
    {
      lossType: 'emergency-health',
      lossDate: '2025-12-15',
      description: 'Emergency room visit',
      estimatedLossAmount: { hasNumericalValue: 8500, hasUnit: 'unit-token' },
      observerAttestationIds: ['obs-proof'],
      memberDocumentIds: ['bill-123'],
    }
  );

  return claim; // status: 'reported'
}
```

### Adjuster Determines Claim

```typescript
async adjusterReviewsClaim(claimId: string, adjusterId: string) {
  const { claim, reasoning, adjustedEvent } = await this.mutual.adjustClaim(
    claimId,
    adjusterId,
    {
      constitutionalCitation: 'coverage-policy-123.md, section 3.2',
      plainLanguageExplanation: 'Approved. Full coverage applies.',
      determinations: {
        coverageApplies: true,
        finalApprovedAmount: { hasNumericalValue: 8500, hasUnit: 'unit-token' },
      },
      auditableDecisionLog: '... full reasoning ...',
    }
  );

  return claim; // status: 'adjustment-made'
}
```

### Settle Claim (Pay Member)

```typescript
async payMember(claimId: string, amount: Measure) {
  const { claim, settlementEvent, attribution } =
    await this.mutual.settleClaim(
      claimId,
      amount,
      'mutual-credit' // or 'fiat-transfer'
    );

  return {
    claimStatus: 'settled',
    paidAmount: amount,
    paymentReference: settlementEvent.id,
  };
}
```

## Integration with Shefa Infrastructure

### EconomicEvent Integration
All operations create immutable events:

```typescript
// When claim is filed
const claimEvent = await economicService.createEvent({
  action: 'deliver-service',
  provider: memberId,
  receiver: 'elohim-mutual',
  metadata: { lamadEventType: 'claim-filed' },
});

// When claim is adjusted
const adjustmentEvent = await economicService.createEvent({
  action: 'deliver-service',
  provider: adjusterId,
  receiver: memberId,
  metadata: { lamadEventType: 'claim-adjusted' },
});

// When claim is settled (payment)
const settlementEvent = await economicService.createEvent({
  action: 'transfer',
  provider: 'health-claims-reserve',
  receiver: memberId,
  metadata: { lamadEventType: 'claim-settled' },
});
```

### CommonsPool Integration
Premiums and claims use the CommonsPool:

```typescript
// Premium payment → CommonsPool grows
await updateCommonsPool('health-claims-reserve', {
  balance: pool.balance + premiumAmount,
});

// Claim settlement → CommonsPool shrinks
await updateCommonsPool('health-claims-reserve', {
  balance: pool.balance - settlementAmount,
});

// Check adequacy
const reserve = await mutual.getReserveStatus('health-claims-reserve');
console.log(`Adequacy ratio: ${reserve.adequacyRatio}`);
```

### Observer Protocol Integration
Risk assessment uses actual behavior:

```typescript
// Not proxy data
const profile = await mutual.assessMemberRisk(memberId, 'health');
// profile.careMaintenanceScore based on observed preventive visits
// profile.communityConnectednessScore based on observed support network
// profile.historicalClaimsRate based on actual past claims
```

## Event Types

14 new event types for insurance mutual:

| Event Type | Action | Resource | Use Case |
|-----------|--------|----------|----------|
| `premium-payment` | transfer | currency | Member pays premium |
| `claim-filed` | deliver-service | stewardship | Claim reported |
| `claim-evidence-submitted` | deliver-service | stewardship | Docs attached |
| `claim-investigated` | work | stewardship | Adjuster investigating |
| `claim-adjusted` | deliver-service | stewardship | Adjuster determined |
| `claim-settled` | transfer | currency | Claim paid |
| `claim-denied` | modify | stewardship | Claim rejected |
| `claim-appealed` | work | stewardship | Member appealing |
| `risk-reduction-verified` | raise | recognition | Prevention verified |
| `preventive-care-completed` | produce | stewardship | Prevention activity |
| `prevention-incentive-awarded` | raise | care-token | Discount earned |
| `coverage-decision` | work | membership | Governance decided |
| `claim-review-initiated` | work | membership | Governance reviewing |
| `reserve-adjustment` | modify | currency | Regulatory change |

---

## Governance & Transparency

### Flag Claims for Governance Review

```typescript
// Automatic flagging for:
// - Large claims (above threshold)
// - Unusual interpretations
// - Pattern concerns (adjuster metrics)
// - Denials of seemingly covered losses

await mutual.flagClaimForGovernanceReview(
  claimId,
  'large-claim',
  'Exceeds $10k threshold'
);
```

### Governance Committee Reviews

```typescript
const flagged = await mutual.getFlaggedClaims(qahalId);
for (const item of flagged) {
  console.log(`Claim: ${item.claim.claimNumber}`);
  console.log(`Adjuster: ${item.reasoning.adjusterId}`);
  console.log(`Reasoning: ${item.reasoning.plainLanguageExplanation}`);
}

// Committee decides
await mutual.resolveAppeal(
  claimId,
  reviewingAdjusterId,
  'modified', // upheld | overturned | modified
  'Committee approves. Loss clearly within coverage.'
);
```

### Adjuster Metrics (Performance)

```typescript
const metrics = await mutual.getAdjusterMetrics(adjusterId);
// claimsProcessed: 25
// denialRate: 15%
// appealRate: 5%
// constitutionalComplianceScore: 95%
// generosityIndex: 40%
// qualityTrend: improving
```

---

## Documentation

- **Integration Guide:** `docs/integration/insurance-mutual-integration-guide.md`
- **Design Review:** `docs/analysis/shefa-models-for-mutual-review.md`
- **Developer Quick Start:** `docs/DEV-QUICK-START.md`
- **Phase 1 Summary:** `docs/PHASE-1-BUILD-SUMMARY.md`

---

## Key Principles

1. **Immutability** - Events are source of truth, not mutable state
2. **Constitutional** - Every decision must cite governance basis
3. **Transparent** - Plain language explanations for members
4. **Preventive** - System rewards risk mitigation
5. **Anti-capture** - Governance oversight of adjuster decisions
6. **Information asymmetry flip** - Use actual behavior, not proxies

---

## Implementation Status

✅ Models defined and typed
✅ Event types added
✅ Service stubs created
⏳ Service method implementations (in progress)
⏳ Integration with EconomicService (next)
⏳ Observer protocol integration (next)
⏳ Governance interface (Phase 2)

---

**Version:** 1.0 (Phase 1)
**Status:** Models Complete, Ready for Implementation
**Last Updated:** December 22, 2025

