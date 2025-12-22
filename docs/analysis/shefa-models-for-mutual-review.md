# Shefa Models Review for Elohim Mutual
## Assessing REA/ValueFlows Readiness for Autonomous Mutual Insurance

**Date:** 2025-12-22
**Status:** Analysis Complete
**Reviewer:** Claude Code

---

## Executive Summary

The existing Shefa economic infrastructure provides a **solid foundation** for building Elohim Mutual. The core REA/ValueFlows models are well-designed and can support most insurance mutual requirements. However, **9 new domain-specific models** are needed to fully operationalize the autonomous mutual vision.

**Key Finding:** We don't need to rebuild the economic foundation—we need to extend it with insurance-specific abstractions that sit *on top of* the existing REA layer.

---

## Part 1: What Shefa Already Provides ✓

### 1. Immutable Economic Event Ledger
**File:** `economic-event.model.ts`

✓ **Perfect for insurance mutual:**
- Events are immutable records with full audit trail
- `EventState` enum tracks processing: pending → validated → countersigned → disputed → corrected
- Full context: provider, receiver, resource type, quantities, timestamps
- Metadata field for arbitrary data (perfect for insurance-specific context)
- `EventSignature` support for cryptographic proof (critical for claims)
- Event queries with filtering by agent, action, resource type, time range

✓ **Existing Lamad event types** include:
- `credit-transfer` - perfect for premium payments
- `credit-retire` - perfect for claims settlements
- `stewardship-begin` - perfect for adjuster assignments
- `affinity-mark` / `endorsement` - perfect for prevention incentives

### 2. Multi-Dimensional Agent Types
**File:** `rea-bridge.model.ts` → `REAAgent`

✓ **Insurance mutual agent roles already defined:**
- `'elohim'` - Can represent Elohim Adjuster instances
- `'organization'` - Risk pools, reinsurance partners, provider networks
- `'community'` - Qahal communities pooling risk together
- `'human'` - Individual members and claimants
- `'family'` - Household economic units

✓ **AgentRelationship types already support:**
- `'member-of'` - Member of a Qahal or risk pool
- `'steward-of'` - Adjuster stewarding a claim process
- `'created-by'` - Track content that established a member relationship
- `'delegate-of'` - Agent acting on behalf of another

### 3. Commitment & Agreement Patterns
**File:** `rea-bridge.model.ts`

✓ **For policy commitments:**
- `Commitment` model tracks binding promises (perfect for coverage commitments)
- `CommitmentState`: proposed → accepted → in-progress → fulfilled/cancelled/breached
- `Agreement` model for constitutional governance documents
- `AgreementClassification` includes:
  - `'constitution'` - Coverage constitution at each layer
  - `'community-covenant'` - Community risk pool agreements
  - `'learning-commitment'` - Can be adapted to stewardship commitments

### 4. Premium Gate System (Revenue Model)
**File:** `steward-economy.model.ts`

✓ **Directly applicable to insurance:**
- `PremiumGate` with flexible pricing models: one-time, subscription, pay-what-you-can
- **Three-way revenue split** (steward%, commons%, contributor%) - exactly what Elohim Mutual needs for:
  - Mutual reserves (commons share)
  - Adjuster compensation (steward share)
  - Network sustainability (contributor share)
- `AccessGrant` for membership/coverage periods
- `StewardRevenue` tracks settlement splits and creates immutable EconomicEvents
- `StewardCredential` with tier system (caretaker, curator, expert, pioneer) - perfect for adjuster qualification levels

### 5. Commons Pool & Value Attribution
**File:** `rea-bridge.model.ts`

✓ **Perfect for risk pool reserves:**
- `CommonsPool` - holds pooled risk reserves in mutual trust
- `ValueAttribution` - tracks attributed value (claims waiting to be paid)
- `AttributionClaim` with graduated claiming:
  - `requiredAttestationLevel`: basic, relationship, biometric, full
  - `identityVerified`, `responsibilityVerified` - prevents fraud
- `ClaimState` lifecycle: pending_identity → pending_responsibility → pending_review → approved/rejected

✓ **This is exactly what member indemnification needs:**
- Premium contributions flow to CommonsPool (mutual reserves)
- When claims occur, member gets AttributionClaim against the pool
- Graduated claiming prevents "walk away and claim" scenarios

### 6. Constitutional Value Flow Control
**File:** `rea-bridge.model.ts`

✓ **Foundation for Bob Parr's conscience:**
- `ValueFlowLayer` defines four layers:
  1. dignity_floor - Basic existential minimums (unextractable)
  2. attribution - Value flows to creators
  3. circulation - Demurrage on hoarding
  4. sustainability - Network development
- `ValueFlowConstraint` - Inviolable constraints with parameters
- `ConstitutionalConstraint` in protocol-core - Governance can't be overridden

✓ **Direct application to mutual:**
- dignity_floor layer protects members from extraction
- Coverage terms cannot be violated for profit
- Adjuster decisions subject to constitutional review

### 7. Observer Protocol Integration Ready
**File:** `rea-bridge.model.ts`

✓ **For attestation-based claims:**
- `eventSignatures` field - supports multiple signers (adjuster, Observer, member)
- `EventSignature` with roles: provider, receiver, witness, validator
- `inScopeOf` - can track which Observer stream verified the claim
- Metadata field - can store Observer attestation chain references

---

## Part 2: Critical Gaps - Models Needed

### Gap 1: Member Risk Profile Model

**Current State:** No member-specific risk assessment data structure

**Needed:**
```typescript
// Required new model
interface MemberRiskProfile {
  memberId: string;

  // Individual risk factors (observed via Observer protocol)
  careMaintenance: number;        // frequency of preventive care
  communityConnectedness: number; // social support availability
  historicalClaimsRate: number;   // past loss frequency

  // Derived risk score
  riskScore: number;    // 0-100, updated continuously
  riskTier: 'low' | 'standard' | 'high' | 'uninsurable';

  // Attestation sources
  evidenceEventIds: string[];     // Observer attestation chains
  lastAssessmentAt: string;
  nextAssessmentDue: string;

  // Trend data for prevention incentives
  riskTrendDirection: 'improving' | 'stable' | 'declining';
  lastRiskScore: number;
}
```

**Why:** Risk-based premium calculation requires actual risk data. The epic explicitly states: "Elohim flips the information asymmetry—the Observer protocol sees actual behavior."

**Integration Point:** Lives alongside existing EconomicResource/REAAgent, linked by memberId.

---

### Gap 2: Policy/Coverage Model

**Current State:** No coverage definition structure

**Needed:**
```typescript
// Required new models
interface CoveragePolicy {
  id: string;
  memberId: string;

  // Coverage structure (graduated by governance layer)
  coverageLevel: 'individual' | 'household' | 'community' | 'network' | 'constitutional';

  // What's covered
  coveredRisks: CoveredRisk[];

  // Terms
  deductible: Measure;
  coinsurance: number;        // 0-100%, member's % of cost
  outOfPocketMaximum: Measure;
  annualLimit: Measure;

  // Effective dates
  effectiveFrom: string;
  effectiveUntil: string;
  renewalTerms: 'annual' | 'monthly' | 'continuous';

  // Governance basis
  governedAt: GovernanceLayer;
  constitutionalBasis: string;    // Citation to coverage constitution

  // Settlement details
  premiumEventId: string;  // Reference to most recent premium payment event
}

interface CoveredRisk {
  riskType: 'health' | 'property' | 'casualty' | 'disaster' | 'care';
  coverage: {
    covered: boolean;
    limitAmount?: Measure;
    coveragePercent?: number;
  };
  conditions: string[];           // Specific conditions or exclusions
  preventionIncentive?: {
    discount: number;             // % premium reduction for risk mitigation
    requiredAttestations: string[]; // What Observer events trigger discount
  };
}
```

**Why:** Coverage is central to the mutual. The epic describes coverage as flowing to the lowest competent governance layer—this model captures that structure.

**Integration Point:** References MemberRiskProfile for risk-based determinations; creates Commitments in the REA system for coverage obligations.

---

### Gap 3: Claim Model (Insurance-Specific)

**Current State:** Generic `Claim` model exists in REA, but insurance claims need domain-specific fields

**Needed:**
```typescript
// Required new model
interface InsuranceClaim {
  id: string;

  // Claim identity
  claimNumber: string;            // Human-readable claim ID
  memberId: string;
  policyId: string;

  // Loss event details
  lossType: 'property-damage' | 'health-incident' | 'casualty' | 'disaster' | 'care-interruption';
  lossDate: string;
  reportedDate: string;           // When member reported it

  // Loss amount
  estimatedLossAmount: Measure;

  // Evidence/attestation
  observerAttestationIds: string[];   // Observer protocol evidence chains
  memberSupportingDocs?: string[];    // Content references to submitted documentation

  // Claim status
  status: InsuranceClaimStatus;

  // Adjudication
  assignedAdjusterId?: string;    // Elohim adjuster agent ID
  adjustmentReasoning?: AdjustmentReasoning;
  settledAmount?: Measure;

  // REA integration
  claimEventId?: string;          // EconomicEvent when claim filed
  adjustmentEventId?: string;     // EconomicEvent when claim adjusted
  settlementEventId?: string;     // EconomicEvent when claim settled

  // Timestamps
  createdAt: string;
  approvedAt?: string;
  settledAt?: string;
}

type InsuranceClaimStatus =
  | 'reported'          // Claim submitted
  | 'under-investigation' // Adjuster gathering evidence
  | 'pending-adjustment' // Adjuster reviewing
  | 'adjustment-made'   // Adjuster decision made
  | 'approved'          // Approved for settlement
  | 'disputed'          // Member or others dispute
  | 'appealed'          // Under appeal
  | 'settled'           // Claim paid
  | 'denied';           // Claim denied

interface AdjustmentReasoning {
  adjusterId: string;
  adjustmentDate: string;

  // Constitutional basis for decision
  constitutionalCitation: string; // Reference to coverage policy

  // The reasoning (for transparency)
  reasoning: string;      // Plain language explanation

  // Determination
  determinations: {
    coverageMeets: boolean;
    policyCoverageAmount: Measure;
    deductibleApplied: Measure;
    coinsuranceApplied: Measure;
    finalApprovedAmount: Measure;
  };

  // Integrity
  generosityInterpretationApplied: boolean;  // "Love as flourishing" principle
  auditableDecisionLog: string;  // Full reasoning for governance review
}
```

**Why:** The epic emphasizes transparent, auditable claims processing. "The Elohim adjuster explains every decision in plain language."

**Integration Point:**
- Creates multiple EconomicEvents (claim-filed, claim-investigated, claim-adjusted, claim-settled)
- Member's AttributionClaim against CommonsPool
- Audit trail for governance review

---

### Gap 4: Adjuster Agent Model

**Current State:** Adjusters exist as generic `REAAgent` type `'elohim'`

**Needed:**
```typescript
// Required new model
interface EuihElohumAdjuster extends REAAgent {
  type: 'elohim';
  role: 'adjuster';

  // Qualification
  tier: 'apprentice' | 'journeyman' | 'master' | 'chief';
  adjustingCertificationIds: string[];
  peerEndorsementEventIds: string[];

  // Performance metrics
  claimsProcessed: number;
  averageAdjustmentTime: string;  // Duration
  denialRate: number;             // % of claims denied
  appealRate: number;             // % appealed by members

  // Integrity tracking
  qualityScore: number;          // Based on governance review of decisions
  constitutionalComplianceScore: number;  // % decisions follow constitution
  generosityIndex: number;       // % of "love as flourishing" interpretations

  // Governance oversight
  reviewEvents: string[];        // EconomicEvent IDs of governance reviews
  disciplinaryActions?: {
    date: string;
    reason: string;
    action: string;
  }[];

  // Catchment area (Qahal communities served)
  servingQahalIds: string[];
}
```

**Why:** Adjusters are the "Bob Parr" role. The system must track their integrity and ensure constitutional compliance.

**Integration Point:**
- Can be marked as steward in AgentRelationship (stewarding the claim process)
- Performance tracked through governance review events
- Constitutional constraints on decision-making

---

### Gap 5: Household/Economic Unit Model

**Current State:** `REAAgent` type `'family'` exists but needs insurance structure

**Needed:**
```typescript
// Required new model
interface MemberHousehold {
  id: string;

  // Household identity
  name: string;
  primaryAccountableId: string;   // Lead member

  // Members
  memberIds: string[];  // All household members
  dependents: {
    memberId: string;
    relationship: 'spouse' | 'child' | 'parent' | 'other';
  }[];

  // Household-level risk pooling
  householdPolicyId: string;  // Collective coverage
  householdRiskScore: number; // Aggregate risk across household

  // Deductible tracking (household-level aggregation)
  aggregateDeductibleApplied: Measure;
  aggregateOutOfPocketMet: Measure;
  aggregateOutOfPocketRemaining: Measure;

  // Household-level events
  householdClaimIds: string[];

  // For Qahal coordination
  memberOfQahalId?: string;  // Which community pool they belong to
}
```

**Why:** The epic describes household-layer risk pooling and governance: "What does our family need covered?"

**Integration Point:** Can be represented as `'family'` agent in REA system with relationship links to individual members.

---

### Gap 6: Deductible & Cost-Sharing Ledger

**Current State:** No persistent tracking of deductible application

**Needed:**
```typescript
// Required new model
interface DeductibleTracker {
  id: string;
  memberId: string;

  // Policy period
  policyYear: number;
  policyPeriodStart: string;
  policyPeriodEnd: string;

  // Deductible accounting
  deductibleAmount: Measure;
  deductibleMet: Measure;           // Amount applied so far
  deductibleRemaining: Measure;     // Amount still needed

  // Cost-sharing tracking
  coinsurancePercent: number;       // Member's % of covered costs
  coinsuranceAmountPaid: Measure;   // What member has paid
  outOfPocketMaximum: Measure;
  outOfPocketAmountPaid: Measure;
  outOfPocketRemaining: Measure;

  // Application events
  deductibleApplicationEvents: Array<{
    claimId: string;
    amountApplied: Measure;
    eventId: string;                // EconomicEvent ID
    appliedDate: string;
  }>;

  // Reset/renewal
  resetDate: string;                // Annual or policy renewal date
}
```

**Why:** Deductibles are critical to insurance mutual operation—they prevent moral hazard and manage claims frequency.

**Integration Point:** Updated by claim settlement events; informs premium calculations for next period.

---

### Gap 7: Reserve Adequacy & Statutory Requirements

**Current State:** `CommonsPool` exists but needs insurance-specific reserve management

**Needed:**
```typescript
// Required new model
interface MutualReserveAccount {
  id: string;

  // Pool identity
  name: string;                     // e.g., "Health Claims Reserve"
  riskPoolId: string;               // Which risk pool this serves

  // Financial position
  balance: Measure;                 // Current reserve balance

  // Statutory requirements
  statutoryMinimum: Measure;        // Required by regulator
  adequacyRatio: number;            // balance / (annual-expected-claims)
  targetAdequacyRatio: number;      // e.g., 1.25x expected annual claims

  // Risk position
  expectedAnnualClaims: Measure;    // Actuarial projection
  expectedLossRatio: number;        // Claims paid / premiums collected

  // Loss development
  reportedButNotPaid: Measure;      // IBNR (Incurred But Not Reported)
  claimsOutstanding: Array<{
    claimId: string;
    estimatedAmount: Measure;
  }>;

  // Premium flows
  premiumsCollected: Measure;       // This period
  premiumsReserved: Measure;        // Applied to this reserve

  // Reserve events
  reserveAdjustmentEvents: string[]; // EconomicEvent IDs

  // Regulatory
  lastRegulatoryCertification?: {
    date: string;
    certifiedAdequate: boolean;
  };
}
```

**Why:** The epic mentions "statutory reserve requirements" and "reserve policies." Regulators require proof of adequate reserves.

**Integration Point:** Draws from CommonsPool; updated by premium and settlement events; subject to governance review.

---

### Gap 8: Reinsurance Contract & Coordination

**Current State:** No reinsurance model

**Needed:**
```typescript
// Required new model
interface ReinsuranceContract {
  id: string;

  // Contract identity
  contractNumber: string;
  reinsurerId: string;              // External reinsurance partner agent

  // Coverage
  coverageTypes: Array<{
    riskType: 'health' | 'property' | 'casualty' | 'disaster';
    limit: Measure;
    attachment: Measure;            // Deductible (reinsurer pays above this)
  }>;

  // Financial terms
  premiumRate: number;              // % of primary premiums ceded
  commissionRate: number;           // Reinsurer's commission
  profitCommission?: boolean;       // Participate in underwriting profit

  // Period
  effectiveFrom: string;
  effectiveUntil: string;
  renewalTerms: string;

  // Claims coordination
  claimsNotificationThreshold: Measure;  // Reinsurer notified above this
  reservingApproach: 'first-loss' | 'pro-rata' | 'excess';

  // Reinsurance events
  cededPremiumEventIds: string[];   // Payments to reinsurer
  reinsuranceClaims: Array<{
    primaryClaimId: string;
    reinsuranceRecoveryEventId: string;
    recoveredAmount: Measure;
  }>;

  // Regulatory
  reinsuranceCreditAllowed: Measure;  // Can be used for reserve calculation
}

interface ReinsuranceClaim {
  id: string;
  contractId: string;
  primaryClaimId: string;

  // Amount
  claimedAmount: Measure;
  coveredByReinsurance: boolean;
  recoveryAmount: Measure;

  // Status
  status: 'submitted' | 'acknowledged' | 'paid' | 'disputed' | 'denied';

  // Events
  claimSubmissionEventId: string;   // EconomicEvent when submitted
  recoveryEventId?: string;         // EconomicEvent when recovered
}
```

**Why:** The epic notes "reinsurance coordination" as critical for catastrophic events and transitional phase.

**Integration Point:** External contract with legacy insurance partners; creates cash outflows (ceded premiums) and inflows (recoveries) as EconomicEvents.

---

### Gap 9: Risk Mitigation Events & Incentive Tracking

**Current State:** Lamad events don't include insurance-specific risk mitigation

**Needed:**

Add new `LamadEventType` values for insurance mutual:

```typescript
// Additions to economic-event.model.ts

type InsuranceMutualEventType =
  // Claims events
  | 'claim-filed'                // Member filed claim
  | 'claim-evidence-submitted'   // Supporting documents attached
  | 'claim-investigated'         // Adjuster gathering evidence
  | 'claim-adjusted'             // Adjuster made determination
  | 'claim-settled'              // Claim paid
  | 'claim-denied'               // Claim rejected
  | 'claim-appealed'             // Member appealed decision

  // Prevention events
  | 'risk-reduction-verified'    // Observer verified risk mitigation
  | 'preventive-care-completed'  // Member completed prevention activity
  | 'safety-improvement-installed' // Property safety improvement
  | 'community-resilience-activity' // Community disaster prep activity

  // Premium/reserve events
  | 'premium-payment'            // Premium paid (mapped to 'credit-transfer')
  | 'reserve-adjustment'         // Regulatory reserve change

  // Governance events
  | 'claim-review-initiated'     // Governance review of adjuster
  | 'coverage-decision'          // Community decided on coverage
  | 'prevention-incentive-awarded' // Reward for risk mitigation;

export const INSURANCE_EVENT_MAPPINGS: Record<InsuranceMutualEventType, {
  action: REAAction;
  resourceType: ResourceClassification;
  defaultUnit: string;
}> = {
  'claim-filed': { action: 'deliver-service', resourceType: 'adjustment', defaultUnit: 'unit-claim' },
  'claim-adjusted': { action: 'deliver-service', resourceType: 'adjustment', defaultUnit: 'unit-claim' },
  'claim-settled': { action: 'transfer', resourceType: 'currency', defaultUnit: 'unit-token' },

  'risk-reduction-verified': { action: 'raise', resourceType: 'recognition', defaultUnit: 'unit-affinity' },
  'preventive-care-completed': { action: 'produce', resourceType: 'stewardship', defaultUnit: 'unit-each' },
  'prevention-incentive-awarded': { action: 'raise', resourceType: 'care-token', defaultUnit: 'unit-token' },
  // ... etc
};
```

**Why:** The epic emphasizes prevention: "The network sees what reduces risk and helps coordinate getting it done." These events create the Observable trail for premium adjustments.

**Integration Point:** Observer protocol generates these events; trigger automated premium adjustments and incentive rewards.

---

## Part 3: Implementation Roadmap

### Phase 1: Core Insurance Models (Required First)
1. **MemberRiskProfile** - Enables risk-based premiums
2. **CoveragePolicy** - Defines what's covered
3. **InsuranceClaim** - Processes claims with full audit trail
4. **AdjustmentReasoning** - Constitutional integrity for Bob Parr

**Timeline:** These four models form the minimum viable mutual.

### Phase 2: Adjuster & Governance (Required for operation)
5. **ElohumAdjuster** - Tracks adjuster integrity and qualification
6. **DeductibleTracker** - Prevents moral hazard

**Timeline:** Needed before processing claims.

### Phase 3: Household & Community (Multi-layer pooling)
7. **MemberHousehold** - Household-level risk pooling
8. **Insurance-specific event types** - Audit trail for governance

**Timeline:** Enables community-layer risk pool features.

### Phase 4: Financial Controls (Regulatory compliance)
9. **MutualReserveAccount** - Meets statutory requirements
10. **ReinsuranceContract** - Legacy economy interface

**Timeline:** Critical before accepting regulatory oversight.

---

## Part 4: Design Principles for New Models

### 1. Immutability Through Events
All state changes must create EconomicEvent entries. Never modify existing claims/policies—create new events that reference and supersede the old.

Example:
```typescript
// WRONG: mutating the claim
claim.status = 'settled';

// RIGHT: creating an event
const settlementEvent = createEvent({
  eventType: 'claim-settled',
  providerId: 'elohim-mutual-pool',
  receiverId: memberId,
  resourceQuantity: { value: settledAmount, unit: 'mutual-credit' },
  fulfills: [claimId],
  note: `Settlement of claim ${claimId}`
});
```

### 2. Graduated Complexity
Members should interact with simple surfaces; complexity lives in the constitutional layers below.

- **Simple surface:** "File a claim" → One action
- **Below:** Full attestation chains, Observer evidence, constitutional review

### 3. Constitutional Constraints
Every decision point should reference:
- What governance layer made the decision
- What constitutional constraint governs it
- Why this choice fits the constitution

### 4. Auditability for Governance Review
The epic emphasizes governance oversight of adjuster decisions:

```typescript
// Every decision must be auditable
adjustmentReasoning: {
  constitutionalCitation: "coverage-policy-martinez-2024, section 3.2",
  reasoning: "Member has full coverage. Deductible of 500 already met this year. Full repair cost of 23,000 approved under dignity-floor principle.",
  generosityInterpretationApplied: true,
  auditableDecisionLog: "..." // Full state of mind
}
```

---

## Part 5: Integration with Existing Shefa Patterns

### How Premium Collection Works (Using Existing Models)

```typescript
// 1. Premium Gate defines the coverage tier
const premiumGate = {
  stewardCredentialId: 'adjusters-guild',
  pricingModel: 'subscription',
  stewardSharePercent: 10,        // Adjuster compensation
  commonsSharePercent: 85,        // Mutual reserve
  contributorSharePercent: 5,     // Network sustainability
};

// 2. Member gains AccessGrant (membership)
const accessGrant = {
  grantType: 'subscription',
  grantedTo: memberId,
  validFrom: '2024-01-01',
  validUntil: '2024-12-31',
  policyRef: coveragePolicyId,
};

// 3. Premium payment triggers StewardRevenue split
const stewardRevenue = {
  fromLearnerId: memberId,
  toStewardPresenceId: 'adjusters-guild',
  toCommonsPoolId: 'health-claims-reserve',
  grossAmount: 1200,
  stewardAmount: 120,        // To adjuster pool
  commonsAmount: 1020,       // To claims reserve
};

// 4. Creates three EconomicEvents (immutable trail)
- stewardEconomicEventId   → Adjuster compensation
- commonsEconomicEventId   → Reserve contribution
- Both flow to CommonsPool for settlement tracking
```

### How Claims Settlement Works (Using Existing Models)

```typescript
// 1. Member files InsuranceClaim with Observer evidence
const claim = {
  memberId,
  policyId,
  observerAttestationIds: ['obs-12345'],  // Observer witnessed the loss
  status: 'reported'
};

// 2. Adjuster creates AdjustmentReasoning (constitutional)
const reasoning = {
  constitutionalCitation: "health-coverage-qahal-2024.md",
  reasoning: "Member has full coverage for medical incidents...",
  determinations: {
    coverageMeets: true,
    finalApprovedAmount: { value: 8500, unit: 'mutual-credit' }
  }
};

// 3. Settlement creates AttributionClaim against CommonsPool
const attribution = {
  agentId: memberId,
  tokenType: 'mutual-credit',
  amount: 8500,
  commonsPoolId: 'health-claims-reserve',
  sourceEventIds: [settlementEventId],
  claimed: true,
  claimEventId: claimSettlementEventId
};

// 4. Immutable EconomicEvent records the settlement
const settlingEvent = {
  id: claimSettlementEventId,
  action: 'transfer',
  provider: 'health-claims-reserve',      // CommonsPool ID
  receiver: memberId,
  resourceClassifiedAs: ['currency'],
  resourceQuantity: { value: 8500, unit: 'mutual-credit' },
  fulfills: [claimId],
  signatures: [
    { signerId: adjusterId, role: 'validator' },
    { signerId: observerNetworkId, role: 'witness' },
  ]
};
```

---

## Part 6: Recommendations

### DO Add These Models (High Priority)
1. ✅ **MemberRiskProfile** - No risk-based pricing without it
2. ✅ **CoveragePolicy** - No coverage without terms
3. ✅ **InsuranceClaim** - No claims processing without structure
4. ✅ **ElohumAdjuster** - No integrity without tracking
5. ✅ **DeductibleTracker** - No cost-sharing without ledger

### CONSIDER Adding (Medium Priority)
6. ~ **MemberHousehold** - Can prototype with REAAgent relationships first
7. ~ **Insurance event types** - Can start with metadata on generic events
8. ~ **MutualReserveAccount** - Can use CommonsPool with annotations initially

### CAN BUILD USING EXISTING (Safe Approaches)
- **Reinsurance:** Represent external reinsurer as REAAgent 'organization'; ceded premiums are transfer events
- **Prevention incentives:** Use existing 'affinity-mark' and 'endorsement' events with 'insurance-prevention' metadata
- **Governance reviews:** Use existing Commitment/Agreement pattern with reviewer roles

### DO NOT DO
- ❌ Don't create a separate ledger system—events ARE the ledger
- ❌ Don't store premium amounts outside of EconomicEvent structure
- ❌ Don't make policy decisions outside of Agreement/Commitment framework
- ❌ Don't track claims without immutable event trail

---

## Part 7: Conclusion

**The verdict:** Shefa's REA/ValueFlows foundation is **genuinely good** for insurance mutual. We're not rebuilding economic coordination—we're adding insurance-specific language (models and event types) on top of a solid platform.

The 9 recommended models are **domain language**, not new paradigms. They translate insurance concepts into REA/ValueFlows vocabulary:

- Risk Profile = Resource Specification + Observer evidence
- Coverage Policy = Commitment + Agreement
- Claims = Events + AttributionClaim against CommonsPool
- Adjuster decisions = Work events + reasoning metadata
- Deductibles = Measure tracking on Event context

**Next Step:** Implement the Phase 1 models (4 core domain concepts) and integrate them with the premium gate system. This creates a working insurance mutual prototype that can process claims, track reserves, and maintain immutable audit trails.

---

**Author:** Claude Code Analysis
**Date:** 2025-12-22
**Status:** Ready for architecture review

