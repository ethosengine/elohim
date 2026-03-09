/**
 * Shefa Module - Economy Context App
 *
 * Economic events, REA value flows, and contributor presence.
 *
 * Routes:
 * - /shefa - Economy home (future)
 * - /shefa/human - Economy-specific profile (future)
 */

// Interfaces (abstract contracts for IoC)
// Note: EconomicEvent is NOT re-exported to avoid collision with canonical
// EconomicEvent in elohim/models. Import directly from interfaces if needed.
export {
  ECONOMIC_EVENT_FACTORY,
  STEWARDED_RESOURCES,
  EXCHANGE,
  COMPUTE_EVENT,
  CUSTODIAN_METRICS,
  DATA_PROTECTION,
  COMPUTE_DASHBOARD,
  FLOW_PLANNING,
} from './interfaces';
export type {
  IEconomicEventFactory,
  AppreciationDisplay,
  CreateAppreciationInput,
  CreateEconomicEventInput,
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
} from './interfaces';

// Models
export * from './models';

// Services
export * from './services';
