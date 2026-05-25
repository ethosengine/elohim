/**
 * Shefa DI tokens — REA action-type stewardship interfaces.
 *
 * These Angular InjectionTokens and their associated interfaces represent
 * the REA-primitive IoC contracts for interacting with shefa's economic
 * event and stewardship infrastructure.
 *
 * Migrated to @elohim/rea-runtime (from @app/shefa barrel) as part of
 * Wave 2 Slice 2.4 of the cross-pillar import cleanup sprint.
 *
 * Design: Token declarations are defined here without embedded factory
 * functions. The concrete implementations (EconomicEventFactoryService,
 * StewardedResourcesApiService, etc.) remain in elohim-app's shefa pillar.
 * elohim-app's DI root registers them under these tokens.
 *
 * This allows libraries to declare injected dependencies on these tokens
 * without depending on the concrete elohim-app services.
 *
 * TODO(wave-3-cutover): When Wave 3 migrates the concrete implementations
 * to @elohim/service, elohim-app's DI root will forward from the new import
 * path. The tokens themselves (defined here) need no change.
 */

import { InjectionToken } from '@angular/core';

import type { REAAction } from './rea-action-types';
import type { LamadEventType } from './rea-action-types';

// =============================================================================
// IEconomicEventFactory
// =============================================================================

/**
 * Flattened appreciation record for UI consumption.
 */
export interface AppreciationDisplay {
  id: string;
  appreciationOf: string;
  appreciatedBy: string;
  appreciationTo: string;
  quantityValue: number;
  quantityUnit: string;
  note: string | null;
  createdAt: string;
}

/**
 * Input for creating an appreciation.
 */
export interface CreateAppreciationInput {
  appreciationOf: string;
  appreciationTo: string;
  quantityValue: number;
  quantityUnit: string;
  note?: string;
}

/**
 * Minimal EconomicEvent shape used by IEconomicEventFactory.
 * The full type lives in @app/elohim/models/economic-event.model.
 */
export interface EconomicEventRecord {
  id: string;
  action: REAAction;
  provider: string;
  receiver: string;
  lamadEventType?: LamadEventType;
  createdAt: string;
  [key: string]: unknown;
}

/**
 * Input for creating an economic event.
 */
export interface CreateEconomicEventInput {
  action: REAAction;
  providerId: string;
  receiverId: string;
  resourceConformsTo?: string;
  resourceInventoriedAs?: string;
  toResourceInventoriedAs?: string;
  resourceClassifiedAs?: string[];
  resourceQuantityValue?: number;
  resourceQuantityUnit?: string;
  effortQuantityValue?: number;
  effortQuantityUnit?: string;
  inputOf?: string;
  outputOf?: string;
  fulfills?: string[];
  realizationOf?: string;
  agreedIn?: string;
  satisfies?: string[];
  inScopeOf?: string[];
  note?: string;
  lamadEventType?: LamadEventType;
}

/**
 * Abstract economic event factory.
 */
export interface IEconomicEventFactory {
  createEconomicEvent(payload: CreateEconomicEventInput): Promise<EconomicEventRecord>;
  getEventsByProvider(agentId: string): Promise<EconomicEventRecord[]>;
  getEventsByReceiver(agentId: string): Promise<EconomicEventRecord[]>;
  getEventsByAction(action: string): Promise<EconomicEventRecord[]>;
  getEventsByLamadType(lamadType: string): Promise<EconomicEventRecord[]>;
  getAppreciationsFor(appreciatedId: string): Promise<AppreciationDisplay[]>;
  getAppreciationsBy(appreciatorId: string): Promise<AppreciationDisplay[]>;
  createAppreciation(payload: CreateAppreciationInput): Promise<AppreciationDisplay>;
}

/**
 * DI token for the economic event factory.
 * elohim-app provides the concrete EconomicEventFactoryService.
 */
export const ECONOMIC_EVENT_FACTORY = new InjectionToken<IEconomicEventFactory>(
  'EconomicEventFactory'
);

// =============================================================================
// IStewardedResources
// =============================================================================

export interface IStewardedResources {
  buildDashboard(stewardId: string): Promise<unknown>;
  [key: string]: unknown;
}

export const STEWARDED_RESOURCES = new InjectionToken<IStewardedResources>('StewardedResources');

// =============================================================================
// IExchange
// =============================================================================

export interface IExchange {
  [key: string]: unknown;
}

export const EXCHANGE = new InjectionToken<IExchange>('Exchange');

// =============================================================================
// IComputeEvent
// =============================================================================

export interface ComputeEventConfig {
  nodeId: string;
  [key: string]: unknown;
}

export interface ComputeUsageSnapshot {
  cpuPercent: number;
  memoryPercent: number;
  [key: string]: unknown;
}

export interface ComputeEventPayload {
  nodeId: string;
  snapshot: ComputeUsageSnapshot;
  [key: string]: unknown;
}

export interface IComputeEvent {
  recordUsage(payload: ComputeEventPayload): Promise<void>;
  [key: string]: unknown;
}

export const COMPUTE_EVENT = new InjectionToken<IComputeEvent>('ComputeEvent');

// =============================================================================
// ICustodianMetrics
// =============================================================================

export interface CustodianMetrics {
  nodeId: string;
  isHealthy: boolean;
  [key: string]: unknown;
}

export interface ICustodianMetrics {
  getMetrics(nodeId: string): Promise<CustodianMetrics>;
  [key: string]: unknown;
}

export const CUSTODIAN_METRICS = new InjectionToken<ICustodianMetrics>('CustodianMetrics');

// =============================================================================
// IDataProtection
// =============================================================================

export interface IDataProtection {
  getProtectionStatus(stewardId: string): Promise<unknown>;
  [key: string]: unknown;
}

export const DATA_PROTECTION = new InjectionToken<IDataProtection>('DataProtection');

// =============================================================================
// IComputeDashboard
// =============================================================================

export interface IComputeDashboard {
  getDashboardState(stewardId: string): Promise<unknown>;
  [key: string]: unknown;
}

export const COMPUTE_DASHBOARD = new InjectionToken<IComputeDashboard>('ComputeDashboard');

// =============================================================================
// IFlowPlanning
// =============================================================================

export interface IFlowPlanning {
  [key: string]: unknown;
}

export const FLOW_PLANNING = new InjectionToken<IFlowPlanning>('FlowPlanning');
