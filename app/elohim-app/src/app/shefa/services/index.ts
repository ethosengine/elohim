/**
 * Shefa Services - Economy Services
 *
 * Shefa is the domain-agnostic economic substrate of the Elohim Protocol.
 * These services provide hREA (Resource-Event-Agent) primitives that
 * domain-specific layers (like Lamad) compose for their use cases.
 *
 * Services:
 * - IEconomicEventFactory (via ECONOMIC_EVENT_FACTORY token): hREA EconomicEvent + appreciation operations
 *
 * Domain-specific services (Lamad):
 * - ContributorService: Contributor dashboards and impact tracking
 * - StewardService: Credentials, gates, access control, revenue
 */

// =============================================================================
// SHEFA SERVICES (Domain-Agnostic hREA Primitives)
// =============================================================================

// Event service (elohim-storage backend) — protocol-level content interaction
export { EventService } from './event.service';
// Re-export protocol event types for convenience (canonical source: @app/elohim/models)
export { ProtocolEventTypes } from '@app/elohim/models/protocol-event-types.model';
export type { ProtocolEventType } from '@app/elohim/models/protocol-event-types.model';
// Legacy re-exports (domain types — will move to lamad barrel)
export { LamadEventTypes } from './event.service';
export type { LamadEventType } from './event.service';

// Attention tracking (dwell-qualified view recording, session dedup)
export { AttentionTrackerService } from './attention-tracker.service';

// Device stewardship (unified device view)
export { DeviceStewardshipService } from './device-stewardship.service';

// Resource explorer (Drive-like resource browsing)
export { ResourceExplorerService } from './resource-explorer.service';

// Custodian metrics API (thin HTTP client)
export { CustodianMetricsApiService } from './custodian-metrics-api.service';

// Data protection API (protection monitoring + derived accessors)
export { DataProtectionApiService } from './data-protection-api.service';

// Compute dashboard API (thin HTTP client for unified dashboard state)
export { ComputeDashboardApiService } from './compute-dashboard-api.service';

// Flow planning API (thin HTTP client for planning, budgets, goals, scenarios)
export { FlowPlanningApiService } from './flow-planning-api.service';

// Resilience API (P2P data protection profile)
export { ResilienceApiService } from './resilience-api.service';

// Node topology API (stewarded node CRUD + availability aggregation)
export { NodeTopologyApiService } from './node-topology-api.service';

// Household devices API (P2P dataplane visibility — /api/v1/households/{id}/devices)
export { HouseholdDevicesService } from './household-devices.service';

// Economic events API (thin HTTP client for hREA economic events)
export { EconomicEventsApiService } from './economic-events-api.service';

// Exchange API (thin HTTP client for request/offer coordination)
export { ExchangeApiService } from './exchange-api.service';

// Steward affinity API (thin HTTP client for affinity queries + curation events)
export { StewardAffinityApiService } from './steward-affinity-api.service';
export type { AffinityQuery } from './steward-affinity-api.service';

// EPR signal-emit (write-through path; EPR Phase 2B Task C.3)
export { SignalEmitService } from './signal-emit.service';
export type {
  SignalIntent,
  SignalEmitSuccessResponse,
  SignalEmitResult,
} from './signal-emit.service';
