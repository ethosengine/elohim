# Shefa Pillar - Economy

REA-based economic coordination, recognition flows, contributor presence.

*Shefa (שפע) = Hebrew for abundance/flow*

**Architecture:** `elohim/ELOHIM_PROTOCOL_ARCHITECTURE.md`

## Models

| Model | Purpose |
|-------|---------|
| `rea-bridge.model.ts` | ValueFlows ontology (Agent, Resource, Event, Process) |
| `contributor-presence.model.ts` | Stewardship lifecycle for absent contributors |
| `economic-event.model.ts` | Immutable value flow audit trail |

## REA Ontology

Resources, Events, Agents - accounting without money assumption.

```typescript
interface EconomicEvent {
  action: REAAction;      // 'use' | 'cite' | 'produce' | 'transfer'
  provider: string;       // Who gave
  receiver: string;       // Who received
  resourceQuantity?: Measure;
  hasPointInTime: string;
}

type LamadEventType =
  | 'content-view'        // Human viewed content
  | 'affinity-mark'       // Human marked affinity
  | 'presence-claim'      // Contributor claimed presence
  | 'recognition-transfer';// Recognition transferred on claim
```

## Contributor Presence

Placeholder identity for external contributors not yet in the network.

```typescript
type PresenceState = 'unclaimed' | 'stewarded' | 'claimed';

interface ContributorPresence {
  presenceState: PresenceState;
  externalIdentifiers: ExternalIdentifier[];  // For claim verification
  accumulatedRecognition: AccumulatedRecognition;
  stewardship?: PresenceStewardship;  // Elohim care while unclaimed
}
```

### Lifecycle

```
1. Content attributed to external contributor
2. ContributorPresence created (unclaimed)
3. Elohim steward the presence (accumulate recognition)
4. Contributor discovers and claims presence
5. Recognition transfers to claimed identity
```

## Resource Classifications

```typescript
type ResourceClassification =
  | 'content'      // Learning content
  | 'attention'    // Human engagement
  | 'recognition'  // Value acknowledgment
  | 'credential'   // Earned attestations
  | 'curation'     // Curated paths
  | 'stewardship'  // Care of presences
  | 'currency';    // Mutual credit (Unyt)
```

## hREA Integration

Models align with hREA GraphQL API for Holochain deployment:
- `EconomicEvent` maps to hREA zome entries
- `ContributorPresence` uses hREA Agent with extensions
- Recognition transfer on claim uses countersigned events

## Routes

```typescript
{ path: '', component: ShefaHomeComponent }
// Placeholder - economy features coming later
```
