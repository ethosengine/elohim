# @elohim/rea-runtime

REA runtime primitives for the Elohim Protocol. This library owns:

- **Economic Event services** — `EventService` for hREA EconomicEvent creation and querying via the elohim-storage SQLite backend
- **Attention tracking** — `AttentionTrackerService` for dwell-time-qualified content-view events (protocol-native attention tracking, replacing analytics)
- **Resource exploration** — `ResourceExplorerService` + `CONTENT_TYPE_FOLDERS` for the stewardship resource browser (Google Drive-like lens over stewarded resources)
- **Commitment action types** — `REAAction` and `LamadEventType` type definitions (split from `@app/elohim/models` per manifest operator-input #3)
- **REA action constants** — `REAActions` (use/produce/transfer/cite/appreciate), `LamadEventTypes` (content-view, assessment-complete, etc.)
- **Resource explorer models** — `LensProvider`, `ExplorerNode`, `ExplorerBreadcrumb`, `ResourceCategory`, `FolderLevel`, `LensDefinition`
- **Shefa DI tokens** — `ECONOMIC_EVENT_FACTORY`, `STEWARDED_RESOURCES`, `EXCHANGE`, `COMPUTE_EVENT`, `CUSTODIAN_METRICS`, `DATA_PROTECTION`, `COMPUTE_DASHBOARD`, `FLOW_PLANNING` (REA action-type stewardship interfaces)

## Architecture

This library is Category C (operational) under the P2P Design Gate taxonomy. It holds no source-of-truth state; all primitives exposed here are projections of substrate entities notarized on the Holochain DHT (elohim and mishpat zomes).

The Z.D Phase 1 substrate (delegates-compute Commitment schema + REA wire formats) already landed on dev (commits `b2380b899`, `7f66391b6`, `bf2efd191`), so this SDK surface absorbs the compute-commitment primitive from day one.

See: `genesis/docs/architecture/rea-compute-commitment-primitive.md` for the canonical primitive spec.

## Usage

```typescript
import { EventService, REAActions, LamadEventTypes } from '@elohim/rea-runtime';
import type { REAAction, LamadEventType } from '@elohim/rea-runtime';
import { ECONOMIC_EVENT_FACTORY } from '@elohim/rea-runtime';
```
