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

// Event service (elohim-storage backend)
export { EventService, LamadEventTypes, REAActions } from './event.service';
export type { LamadEventType, REAAction } from './event.service';

// Device stewardship (unified device view)
export { DeviceStewardshipService } from './device-stewardship.service';

// Resource explorer (Drive-like resource browsing)
export { ResourceExplorerService } from './resource-explorer.service';

// Custodian metrics API (thin HTTP client)
export { CustodianMetricsApiService } from './custodian-metrics-api.service';

// Data protection API (protection monitoring + derived accessors)
export { DataProtectionApiService } from './data-protection-api.service';
