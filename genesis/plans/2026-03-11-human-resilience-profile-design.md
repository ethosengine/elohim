# Human Resilience Profile — Shefa Projection for P2P Data Protection

**Date:** 2026-03-11
**Status:** Approved
**Scope:** ResilienceProfile type, a2o scenarios through genesis humans, icon design story skeleton

## Problem

A human in the Elohim Protocol has data distributed across peers through sharded storage, mutual aid commitments, and trust topology — but has no way to know at a glance whether they're protected. The primitives exist (shard manifests, Reed-Solomon encoding, mutual aid contexts, stewardship allocations, custodian nodes) but nothing composes them into a single answer: "Am I safe? And if not, what should I do?"

## Architecture Decision: Shefa Projection, Not New Protocol Primitive

The resilience profile is a **shefa projection** — computed from existing protocol primitives, not a new on-chain type. This follows the established pattern: protocol primitives (ShardManifest, MutualAidContext, Commitment, CustodianNode) are the source of truth; shefa computes views over them.

No new Holochain entries. No new wire protocols. The profile is assembled from what the protocol already knows.

## Core Concept

### Two States That Matter

- **Protected** — data survives the loss of any single peer. Stewardship is reciprocated, shards are distributed, relationships are alive (verified implicitly through access). The indicator is ambient — you don't think about it.

- **At Risk** — something degraded. A peer went offline, a commitment expired, shards are concentrated. The indicator tells you *what to do*: make connections, accept a mutual aid offer, diversify your network.

Between these is a continuous gradient. The human should understand where they are and what the next step is.

### Not All Data Needs the Same Protection

Protection is per-content, shaped by reach and sensitivity. Medical records need institutional-grade attestation and jurisdictional awareness. A shared movie just needs a friendly peer. The resilience profile reflects this — different adequacy thresholds for different content buckets, and elohim discernment about whether the mechanical score is *appropriate* for what the data actually is.

### Attestation Through Use, Not Ceremony

Relationships are reaffirmed by living in them. Every shard fetch is a heartbeat. Every sync is proof-of-life. The protocol notices what's already happening rather than requiring separate attestation rounds. This must scale to billions of humans without collapsing under verification noise — the natural frequency of access is the signal.

### The Right to Be Forgotten

Data has a lifecycle. Memories degrade, content becomes irrelevant, and the protocol should honor that gracefully. The resilience profile isn't only about protecting data — it's also about knowing when to let it go. Graceful degradation, intentional forgetting, and dignified release of data that no longer serves its purpose are resilience concerns, not just retention concerns. There is more thought needed here, but the model should carry this from the start.

## Type Design

### ResilienceProfile

The top-level projection. Lives in shefa models alongside `StewardedResource`.

```typescript
type ProtectionStatus = 'at-risk' | 'partial' | 'protected';

interface ResilienceProfile {
  humanId: string;
  overallScore: number;                    // 0-1 normalized
  protectionStatus: ProtectionStatus;

  // What feeds the score
  shardHealth: ShardHealthSummary;
  commitmentHealth: CommitmentHealthSummary;
  trustCircleDepth: TrustCircleDepth;

  // Per-content risk (not all data needs the same protection)
  contentRiskBreakdown: ContentRiskBucket[];

  // What to do about it
  nextAction?: ResilienceAction;

  // Elohim assessment (discernment layer)
  elohimAssessment?: ElohimResilienceAssessment;

  lastComputedAt: string;
}
```

### ShardHealthSummary

Mechanical distribution metrics derived from `ShardManifest` and peer topology.

```typescript
interface ShardHealthSummary {
  totalBlobs: number;
  totalShards: number;
  distinctPeers: number;                   // How many unique peers hold shards
  averageShardsPerBlob: number;
  encodingBreakdown: {
    single: number;                        // Blobs with no redundancy
    chunked: number;                       // Sequential chunks
    reedSolomon: number;                   // RS 4-of-7 (recoverable)
  };
  singlePointOfFailureCount: number;       // Blobs on only one peer
  lastAccessVerifiedAt: string;            // Most recent implicit heartbeat
}
```

### CommitmentHealthSummary

Derived from `MutualAidContext` and `Commitment` records.

```typescript
interface CommitmentHealthSummary {
  activeCommitments: number;               // Mutual aid agreements in force
  reciprocatedCommitments: number;         // Commitments where both sides contribute
  expiringSoon: number;                    // Commitments nearing expiry
  totalPeersCommitted: number;             // Distinct humans backing your data
  commitmentCoverage: number;              // 0-1, what fraction of data is commitment-backed
}
```

### TrustCircleDepth

Which relationship circles contribute to stewardship. Derived from trust topology and custodian node relationships.

```typescript
interface TrustCircleDepth {
  householdPeers: number;                  // Family/intimate trust
  friendPeers: number;                     // Personal trust
  communityPeers: number;                  // Community/congregation
  institutionalPeers: number;              // Professional/organizational
  totalCircles: number;                    // How many distinct trust levels contribute
}
```

### ContentRiskBucket

Groups content by appropriate protection level. Not all data needs the same distribution.

```typescript
interface ContentRiskBucket {
  reach: ReachLevel;                       // personal, trusted, community, commons
  contentCount: number;
  shardDistribution: number;              // Avg distinct peers per content in this bucket
  adequacy: number;                       // 0-1, is distribution appropriate FOR this reach?
  exemplar?: string;                      // Human-readable example: "medical records", "shared media"
}
```

### ElohimResilienceAssessment

The discernment layer. Elohim are LLMs — they need narrative memory alongside quantitative scores. The elohim doesn't just compute; it *understands* context, remembers history, and offers judgment about whether the mechanical score is adequate for what the data actually is.

Elohim can also help *set* the score at higher abstract levels requiring discernment — institutional/political risk indicators, boundary-based attestations, jurisdictional awareness that a formula can't capture.

```typescript
interface ElohimResilienceAssessment {
  assessedAt: string;
  assessedBy: AgentRef;
  overallAdequacy: number;                // Elohim's judgment of the mechanical score

  // Narrative memory — contextual understanding
  narrative: string;                       // Current assessment in natural language
  memories: ResilienceMemory[];            // Accumulated context across assessments

  concerns: ResilienceConcern[];
  attestations: string[];                  // EPR refs to boundary/institutional attestations
  constitutionalBasis?: string;
}

interface ResilienceMemory {
  id: string;
  recordedAt: string;
  updatedAt: string;
  content: string;                         // Free text — the elohim's observation
  relevance: 'active' | 'background' | 'resolved';
  relatedContentIds?: string[];
  relatedHumanIds?: string[];
  supersededBy?: string;                   // Memory ID if updated/corrected
}

interface ResilienceConcern {
  severity: 'informational' | 'concerning' | 'critical';
  description: string;                     // Natural language concern
  affectedContentIds?: string[];
  suggestedAction?: string;
}
```

### ResilienceAction

The single most impactful thing a human can do to improve their resilience.

```typescript
interface ResilienceAction {
  type: 'connect' | 'diversify' | 'renew' | 'review' | 'release';
  description: string;                     // Human-readable: "Connect with a community custodian"
  suggestedPeerIds?: string[];             // Potential mutual aid partners
  urgency: 'whenever' | 'soon' | 'now';
}
```

## A2O Scenarios — Graduated Through Genesis Humans

Each scenario exercises the resilience projection through a real persona situation, building on the trust topology established in the P2P scaling design.

### Matthew (Baseline — Single Conductor)
Matthew seeds all his stewardship content on a single conductor. His ResilienceProfile shows `at-risk` — everything lives on one node. The elohim narrative: "All your data is on a single device. If it fails, you lose everything." The next action: invite Susan's conductor as household backup.

### Matthew + Susan (Household Reciprocation)
Susan's conductor comes online, household content replicates via spouse relationship (intimate trust). Matthew's profile improves — family content is now on two nodes. But the elohim remembers: "Both conductors are in the same household. A single infrastructure failure affects both." Still `partial`. Content risk breakdown shows personal-reach content is dual-hosted but community-reach content still has no community peers.

### + Pastor Pete (Community Depth)
Pete's congregation node adds a third trust circle. Community-reach content replicates to Pete. Matthew's faith-related content now survives household failure. The elohim notes: personal-reach medical notes are still household-only, which is *appropriate* — they shouldn't replicate to the congregation. The adequacy score for personal-reach content is high despite low peer count, because the elohim's discernment says household-only is correct for that reach level.

### + Timothy + Frank (Network Diversity)
Multiple trust bridges. Content flows through relationship paths. The resilience projection shows different adequacy per content bucket: commons content is well-distributed (5 peers), community content has 3 peers across 2 clusters, personal content stays tight (2 peers, household). The elohim's narrative reflects that this distribution *matches* the content's nature. Protection status: `protected`.

### Maria (Cold Start)
Joins with zero peers. Profile is `at-risk` with zero shard distribution. The elohim's first action: help her connect to a trust circle. As she builds relationships, her resilience grows organically — not through admin bulk-loading, but through the protocol reflecting real human connection. The elohim tracks her progress: "Maria connected with Susan through the learning community. First mutual aid commitment established."

### Degradation (Matthew Goes Offline)
Matthew's conductor goes offline unexpectedly. Susan's profile shifts from `protected` to `partial` for household content. The elohim triggers an emergency `MutualAidContext`, records a memory about the degradation, and helps Susan understand what happened. When Matthew comes back, the elohim captures after-action: "Single-household concentration was the vulnerability. Recommend diversifying personal-reach backup to a trusted friend."

### Graceful Goodbye (Content Lifecycle)
A scenario exploring intentional data release. Content that has served its purpose — old drafts, expired community announcements, superseded learning materials — should be releasable with dignity. The resilience profile tracks not just "is this protected?" but "does this still need protecting?" The elohim can surface: "You have 47 expired community announcements still replicated across 3 peers. Would you like to release them?"

## Icon Design Direction (Story-Only — For Later)

The resilience profile needs a visual representation that communicates at a glance:
- Where you are on the gradient from at-risk to protected
- What the next action is (if any)
- That the indicator fades to background when everything is healthy

### Design Inspiration
- **WiFi bars** — graduated signal strength, universally understood, tiny footprint
- **Military chevrons** — convey rank, branch, specialty, time-in-service in a few stripes. A small visual element can carry layered meaning: protection level, trust circle depth, content sensitivity, commitment health
- **Protocol identity logos** — Hylo, Collaborative Technology Alliance — icons that embody a protocol's values while functioning as status indicators

### What the Icon Must Convey
- A continuous gradient, not discrete states — partial protection is visible
- Degradation is noticeable but not alarming (color shift, not alert)
- At full protection, the icon is ambient — like a full WiFi signal, you stop noticing it
- The icon should work at very small sizes (16x16, favicon, mobile status bar)
- Different content has different protection needs — the icon might represent an aggregate or be contextual to what you're viewing

### Open Questions for Visual Design
- Does the icon represent your *overall* resilience or the resilience *of the content you're currently viewing*?
- Should degradation be shown as loss (bars disappearing) or change (color shift)?
- How does the icon transition from "you need to act" to "you're safe" to the Maslow graduation where community governance/stewardship reach becomes the foreground concern?
- How do we represent the right to forget / graceful release visually?

This is a design exploration to pick up in the UI playground — story-only, no implementation.

## Source Primitives (What Already Exists)

| Primitive | Location | Role in Resilience |
|-----------|----------|-------------------|
| `ShardManifest` | `elohim-storage/src/sharding.rs`, `storage-client-ts/src/types.ts` | Shard distribution, encoding level |
| `MutualAidContext` | `qahal/models/mutual-aid.model.ts` | Governance-backed mutual aid agreements |
| `Commitment` | `elohim/models/rea-bridge.model.ts` | REA commitments backing stewardship |
| `CustodianNode` | `shefa/models/shefa-dashboard.model.ts` | Peer identity, location, trust level |
| `FamilyCommunityProtectionStatus` | `shefa/models/shefa-dashboard.model.ts` | Redundancy, geographic distribution |
| `StewardedResource` (compute) | `shefa/models/stewarded-resources.model.ts` | Resource capacity and allocation |
| `ContentStewardship` | `storage-client-ts/src/generated/ContentStewardship.ts` | Per-content steward allocations |
| `HumanAffinity` | `qahal/models/human-affinity.model.ts` | Trust circle relationships |
| `CoordinationEnvelope` (sense/respond) | `elohim/models/coordination-envelope.model.ts` | Implicit attestation through use |
| `ContributorPresence` | `elohim/models/contributor-presence.model.ts` | Stewardship lifecycle |

## Dependencies

- Story-driven P2P scaling plan (2026-03-09) — graduated conductor topology
- Coordination verb layer (2026-03-09) — sense/respond for implicit attestation
- Existing shefa dashboard model — `FamilyCommunityProtectionStatus`, `CustodianNode`
- Existing stewarded resources — `ComputeResource`, `AllocationBlock`

## What This Enables

1. **"Am I safe?" at a glance** — every human knows their data protection status without understanding P2P infrastructure
2. **Actionable next steps** — the profile doesn't just report, it suggests what to do
3. **Elohim as care provider** — LLM agents with memory, not scoring functions, interpreting protection in context
4. **Mutual aid visibility** — the care work of backing up each other's data is economically visible through REA, constantly accounted for even when "free" through family/friend/community bonds
5. **Graceful lifecycle** — data that no longer serves its purpose can be released with dignity, not hoarded forever
6. **Foundation for the icon** — the model is buildable now; the visual design is an open exploration for the UI playground
