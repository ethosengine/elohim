# REA Generated Types & Shefa Service Landscape

## EconomicEventView

```typescript
interface EconomicEventView {
  id: string;
  appId: string;
  action: string;                    // REA action verb
  provider: string;                  // Who gave
  receiver: string;                  // Who received
  resourceConformsTo?: string;       // Resource type
  resourceInventoriedAs?: string;    // Specific resource
  resourceClassifiedAs?: JsonValue;  // Classification tags
  resourceQuantityValue?: number;
  resourceQuantityUnit?: string;
  effortQuantityValue?: number;      // Time/effort spent
  effortQuantityUnit?: string;
  hasPointInTime: string;            // When it happened
  hasDuration?: string;
  inputOf?: string;                  // Input to process
  outputOf?: string;                 // Output of process
  lamadEventType?: string;           // Elohim-specific type
  contentId?: string;                // Related content
  contributorPresenceId?: string;    // Related contributor
  pathId?: string;                   // Related learning path
  triggeredBy?: string;              // What triggered this
  state: string;                     // 'pending' | 'confirmed' | 'settled'
  note?: string;
  metadata?: JsonValue;
  createdAt: string;
}
```

## Lamad Event Types

```typescript
type LamadEventType =
  | 'content-view'           // Human viewed content
  | 'affinity-mark'          // Human marked affinity
  | 'presence-claim'         // Contributor claimed presence
  | 'recognition-transfer'   // Recognition transferred on claim
  | 'learning-engagement'    // Session completion, mastery change
  | 'content-contribution'   // Content creation or curation
  | 'compute-contribution'   // Node hosting resources
  | 'governance-participation'; // Voting, consent process
```

## REA Actions

```typescript
type REAAction =
  | 'use'        // Consume or use a resource
  | 'cite'       // Reference/attribute a resource
  | 'produce'    // Create a new resource
  | 'transfer'   // Move resource between agents
  | 'work'       // Contribute effort
  | 'accept'     // Accept a delivered resource
  | 'modify';    // Change a resource
```

## StewardshipAllocationView

```typescript
interface StewardshipAllocationView {
  id: string;
  contentId: string;
  stewardPresenceId: string;
  allocationRatio: number;           // 0.0 - 1.0
  allocationMethod: string;          // 'manual' | 'algorithmic' | 'governance'
  contributionType: string;          // 'author' | 'curator' | 'translator' | 'inherited'
  contributionEvidence?: JsonValue;
  governanceState: string;           // 'active' | 'disputed' | 'ratified'
  recognitionAccumulated: number;
  // ... dispute fields, effective dates, etc.
}
```

## ContributorPresenceView

```typescript
interface ContributorPresenceView {
  id: string;
  displayName: string;
  presenceState: string;  // 'unclaimed' | 'stewarded' | 'claimed'
  externalIdentifiers?: JsonValue;
  establishingContentIds: JsonValue;  // Content that establishes this presence
  affinityTotal: number;
  uniqueEngagers: number;
  citationCount: number;
  recognitionScore: number;
  stewardId?: string;                 // Who stewards this presence
  claimedAgentId?: string;            // After claim verification
  // ... claim fields
}
```

## Resource Classifications

```typescript
type ResourceClassification =
  | 'content'       // Learning content
  | 'attention'     // Human engagement
  | 'recognition'   // Value acknowledgment
  | 'credential'    // Earned attestations
  | 'curation'      // Curated paths
  | 'stewardship'   // Care of presences
  | 'currency';     // Mutual credit (Unyt)
```

---

## Shefa Service Landscape

Services in `elohim-app/src/app/shefa/services/`:

### Core Services

| Service | Status | Purpose |
|---------|--------|---------|
| `EconomicService` | Partial | REA event creation and querying |
| `EconomicEventFactoryService` | Partial | Typed event construction |
| `EventService` | Partial | Event listing and filtering |
| `ElohimStubService` | **Intentional stub** | Mock for development/testing |

### Contributor & Stewardship

| Service | Status | Purpose |
|---------|--------|---------|
| `StewardedResourceService` | Partial | Stewardship allocation management |
| `AppreciationService` | Partial | Recognition flows |
| `ComputeEventService` | Partial | Infrastructure contribution events |

### Marketplace

| Service | Status | Purpose |
|---------|--------|---------|
| `RequestsAndOffersService` | Implemented | P2P service marketplace |
| `FlowPlanningService` | Stub | Multi-step economic flow planning |

### Banking Bridge

| Service | Status | Purpose |
|---------|--------|---------|
| `PlaidIntegrationService` | Stub | External banking integration |
| `TransactionImportService` | Stub | Import external transactions |
| `BudgetReconciliationService` | Stub | Reconcile external vs internal |
| `AICategorizationService` | Stub | ML categorization of transactions |
| `DuplicateDetectionService` | Stub | Prevent duplicate imports |
| `EconomicEventBridgeService` | Stub | Bridge external -> REA events |

### Insurance

| Service | Status | Purpose |
|---------|--------|---------|
| `InsuranceMutualService` | Stub | Community mutual aid |
| `FamilyCommunityProtectionService` | Stub | Family/community protection pools |

### Compute

| Service | Status | Purpose |
|---------|--------|---------|
| `ShefaComputeService` | Stub | Compute resource contribution tracking |
