/**
 * @elohim/rea-runtime — Public API
 *
 * REA runtime primitives for the Elohim Protocol.
 * Economic Event services, Commitment action types, signal-bearing primitives.
 *
 * Architecture: This library is Category C (operational) under the P2P Design
 * Gate taxonomy. It holds no source-of-truth state; all primitives it exposes
 * are projections of substrate entities notarized on the Holochain DHT
 * (elohim and mishpat zomes). See rea-compute-commitment-primitive.md.
 */

// REA action types (split from @app/elohim/models per manifest operator-input #3)
export type { REAAction, LamadEventType } from './lib/rea-action-types';
export { REAActions, LamadEventTypes } from './lib/rea-action-types';

// Protocol event types (inlined from @app/elohim/models/protocol-event-types.model)
export type { ProtocolEventType, EventREAMapping } from './lib/protocol-event-types';
export { ProtocolEventTypes, PROTOCOL_EVENT_MAPPINGS, PROTOCOL_UNITS } from './lib/protocol-event-types';

// EventService — hREA EconomicEvent service
export { EventService, EVENT_API } from './lib/event.service';
export type {
  IEconomicEventApi,
  CreateEconomicEventParams,
  EconomicEventQuery,
} from './lib/event.service';

// AttentionTrackerService — dwell-time attention tracking as economic events
export { AttentionTrackerService, AGENT_CONTEXT, DWELL_THRESHOLD_MS } from './lib/attention-tracker.service';
export type { IAgentContext } from './lib/attention-tracker.service';

// ResourceExplorerService — stewardship resource browsing
export {
  ResourceExplorerService,
  CONTENT_TYPE_FOLDERS,
  CONTENT_DATA_SOURCE,
  LENS_REGISTRY,
} from './lib/resource-explorer.service';
export type { IContentDataSource, ILensRegistry } from './lib/resource-explorer.service';

// Resource explorer models (LensProvider, ExplorerNode, etc.)
export type {
  ExplorerBreadcrumb,
  ExplorerNode,
  LensProvider,
  ResourceCategory,
  FolderLevel,
  LensDefinition,
} from './lib/resource-explorer.model';

// Signal-emit protocol types and service (EPR write-through, Task C.3)
export type {
  SignalIntent,
  SignalEmitSuccessResponse,
  SignalEmitResult,
} from './lib/signal-emit-types';
export { SignalEmitService } from './lib/signal-emit.service';

// Shefa DI tokens — REA action-type stewardship interfaces
export {
  ECONOMIC_EVENT_FACTORY,
  STEWARDED_RESOURCES,
  EXCHANGE,
  COMPUTE_EVENT,
  CUSTODIAN_METRICS,
  DATA_PROTECTION,
  COMPUTE_DASHBOARD,
  FLOW_PLANNING,
} from './lib/shefa-di-tokens';

export type {
  IEconomicEventFactory,
  AppreciationDisplay,
  CreateAppreciationInput,
  CreateEconomicEventInput,
  EconomicEventRecord,
  IStewardedResources,
  IExchange,
  IComputeEvent,
  ComputeEventConfig,
  ComputeUsageSnapshot,
  ComputeEventPayload,
  ICustodianMetrics,
  CustodianMetrics,
  IDataProtection,
  IComputeDashboard,
  IFlowPlanning,
} from './lib/shefa-di-tokens';
