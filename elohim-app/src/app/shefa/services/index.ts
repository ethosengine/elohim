/**
 * Shefa Services - Economy Services
 *
 * Shefa is the domain-agnostic economic substrate of the Elohim Protocol.
 * These services provide hREA (Resource-Event-Agent) primitives that
 * domain-specific layers (like Lamad) compose for their use cases.
 *
 * Services:
 * - EconomicService: hREA EconomicEvent operations (value flows)
 * - AppreciationService: Recognition/appreciation flows
 *
 * Domain-specific services (Lamad):
 * - ContributorService: Contributor dashboards and impact tracking
 * - StewardService: Credentials, gates, access control, revenue
 */

// =============================================================================
// SHEFA SERVICES (Domain-Agnostic hREA Primitives)
// =============================================================================

// Economic events (immutable value flow records)
export { EconomicService } from './economic.service';
export type { CreateEconomicEventInput } from './economic.service';

// Appreciation (recognition flows)
export { AppreciationService } from './appreciation.service';
export type { AppreciationDisplay, CreateAppreciationInput } from './appreciation.service';
