---
name: rea-economics
description: Reference for REA (Resource-Event-Agent) economics, hREA on Holochain, Unyt mutual credit patterns, ValueFlows ontology, and the shefa pillar service landscape. Use when someone asks "create economic event", "set up stewardship", "contributor presence flow", "mutual credit", or works on requests/offers and currency flows.
metadata:
  author: elohim-protocol
  version: 1.0.0
---

# REA Economics Reference

The Elohim Protocol uses REA (Resource-Event-Agent) accounting for its economic layer, with hREA on Holochain and Unyt mutual credit patterns.

## REA Model

REA is an accounting framework that tracks value flows without assuming money:

```
Agent (who)  -->  Event (what happened)  -->  Resource (what changed)
                       |
                       v
                  Process (context)
```

### Core Concepts

| Concept | Description |
|---------|-------------|
| **Agent** | Any entity that participates (human, organization, node) |
| **Resource** | Anything of value (content, attention, recognition, credit) |
| **Event** | An observed change in resource state (create, use, transfer) |
| **Process** | A context grouping related events (learning session, content creation) |
| **Intent** | A forward-looking desired event (request, offer) |
| **Commitment** | A promised future event |
| **Fulfillment** | Links commitments to events that fulfill them |

---

## Layered Economic Stack

```
┌────────────────────────────────────────────────────────┐
│ STEWARDSHIP LAYER (Constitutional AI)                   │
│ Dignity floor, accumulation ceiling, graduated care     │
├────────────────────────────────────────────────────────┤
│ SETTLEMENT LAYER (future blockchain)                    │
│ External settlement when needed                         │
├────────────────────────────────────────────────────────┤
│ LEDGER LAYER (hREA / Holochain)                        │
│ EconomicEvent entries, agent-centric accounting         │
├────────────────────────────────────────────────────────┤
│ WITNESS LAYER (Observer Protocol)                       │
│ Immutable event observation, attestation                │
└────────────────────────────────────────────────────────┘
```

---

## Unyt Mutual Credit

The Elohim Protocol uses Unyt-inspired mutual credit with multiple currency swimlanes.

### Currency Swimlanes

| Currency | Unit | Tracks |
|----------|------|--------|
| **Time** | hours | Time contributed to community |
| **Care** | care-units | Caregiving, mentoring, support |
| **Infrastructure** | compute-hours | Node hosting, bandwidth, storage |
| **Learning** | recognition-units | Knowledge creation, curation, teaching |
| **Creator** | creator-tokens | Original content creation |

### Mutual Credit Principles

- **Agent-centric creation**: Credit created at the point of value flow (not minted centrally)
- **Constitutional limits**: Dignity floor (minimum access) + accumulation ceiling (maximum balance)
- **No debt spiral**: Negative balances limited by community-set bounds
- **Graduated stewardship**: As contributors demonstrate care, they gain stewardship privileges

---

## Generated Types (from elohim-storage)

Key types: `EconomicEventView`, `StewardshipAllocationView`, `ContributorPresenceView`. REA actions: `use`, `cite`, `produce`, `transfer`, `work`, `accept`, `modify`. Lamad event types: `content-view`, `affinity-mark`, `presence-claim`, `recognition-transfer`, `learning-engagement`, `content-contribution`, `compute-contribution`, `governance-participation`.

See `references/generated-types.md` for full interface definitions and the complete shefa service landscape.

---

## Stewardship Flow

```
1. Content imported with contributor attribution
2. ContributorPresence created (unclaimed)
3. StewardshipAllocation links content -> contributor
4. Recognition accumulates via economic events (views, affinity marks)
5. Contributor discovers and claims presence
6. Recognition transfers to claimed identity
7. Steward can then participate in governance for their content
```

---

## Contributor Presence

Placeholder identity for external contributors not yet in the network. Key fields: `presenceState` (`unclaimed` | `stewarded` | `claimed`), `recognitionScore`, `stewardId`, `claimedAgentId`. See `references/generated-types.md` for full `ContributorPresenceView` interface.

### Presence Lifecycle

```
unclaimed -> stewarded -> claimed

1. Import creates presence (unclaimed)
2. Elohim steward assigned (stewarded)
3. Real contributor claims (claimed)
   - Verification: email, social proof, etc.
   - Recognition transfers to new agent
```

---

## Shefa Service Landscape

Services in `elohim-app/src/app/shefa/services/`. Most are stubs defining the API surface. Only `RequestsAndOffersService` is fully implemented. Core services (`EconomicService`, `EconomicEventFactoryService`, `EventService`) are partial. `ElohimStubService` is an intentional mock.

Six service groups: Core, Contributor/Stewardship, Marketplace, Banking Bridge, Insurance, Compute. See `references/generated-types.md` for the full service listing with status and purpose.

Resource classifications: `content`, `attention`, `recognition`, `credential`, `curation`, `stewardship`, `currency`.

---

## Common Operations

### Create Economic Event

```typescript
const input: CreateEconomicEventInputView = {
  action: 'use',
  provider: 'system',
  receiver: humanId,
  lamadEventType: 'content-view',
  contentId: 'concept-governance',
  resourceQuantityValue: 1,
  resourceQuantityUnit: 'view',
  hasPointInTime: new Date().toISOString(),
};
await http.post('/db/economic-events', input);
```

### Create Stewardship Allocation

```typescript
const input: CreateAllocationInputView = {
  contentId: 'concept-governance',
  stewardPresenceId: 'presence-satoshi',
  allocationRatio: 0.8,
  allocationMethod: 'manual',
  contributionType: 'author',
};
await http.post('/db/stewardship-allocations', input);
```

### Initiate Contributor Claim

```typescript
const input: InitiateClaimInputView = {
  claimingAgentId: myAgentId,
  verificationMethod: 'email',
  evidence: { email: 'satoshi@example.com', verifiedAt: '...' },
};
await http.post(`/db/contributor-presences/${presenceId}/claim`, input);
```

---

## Integration Points with Earlier Milestones

| Milestone | Integration |
|-----------|-------------|
| M1 (Content) | Path completion -> `learning-engagement` event |
| M2-M3 (Assessment) | Mastery achievement -> `learning-engagement` event |
| M4 (Desktop) | Node metrics -> `compute-contribution` event |
| M5-M6 (Economics) | Full REA flows, mutual credit, marketplace |

---

## Gotchas

1. **Most services are stubs** - Many shefa services return `Promise.reject('Not implemented')`. This is expected. They define the API surface for future implementation.

2. **`ElohimStubService` is intentional** - It provides mock data for development. Don't remove it or try to "fix" it.

3. **Hardcoded exchange rates** - Current economic calculations use placeholder exchange rates. Real rates will come from the Unyt protocol.

4. **`lamadEventType` is Elohim-specific** - Standard REA doesn't have this field. It's an extension for tracking learning-specific events.

5. **Events are immutable** - Once created, economic events cannot be modified (append-only audit trail). Corrections are new events that reference the original.

6. **hREA alignment** - Models align with the hREA GraphQL API. The zome types in `holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` define the Holochain entry types.

---

## Key Files

| File | Purpose |
|------|---------|
| `elohim-app/src/app/shefa/claude.md` | Shefa pillar overview |
| `elohim-app/src/app/shefa/models/rea-bridge.model.ts` | ValueFlows ontology types |
| `elohim-app/src/app/shefa/models/contributor-presence.model.ts` | Presence model |
| `elohim-app/src/app/shefa/models/economic-event.model.ts` | Event model |
| `elohim-app/src/app/shefa/services/economic.service.ts` | Core economic service |
| `elohim-app/src/app/shefa/services/requests-and-offers.service.ts` | Marketplace |
| `elohim-app/src/app/shefa/README-REQUESTS-AND-OFFERS.md` | Marketplace design |
| `elohim-app/src/app/shefa/README-INSURANCE-MUTUAL.md` | Insurance mutual design |
| `genesis/docs/Shefa_Economic_Infrastructure_Whitepaper.md` | Economic whitepaper |
| `research/economic/README.md` | Unyt architecture exploration |
| `holochain/sdk/storage-client-ts/src/generated/EconomicEventView.ts` | Generated type |
| `holochain/elohim-storage/src/views.rs` | Rust view types (EconomicEventView, etc.) |

## External References

- ValueFlows spec: `https://valueflo.ws/`
- hREA on Holochain: `https://hrea.io/`
- Unyt mutual credit: `https://unyt.co/`
