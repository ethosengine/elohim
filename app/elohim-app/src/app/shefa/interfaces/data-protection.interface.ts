/**
 * IDataProtection -- Abstract interface for family-community data protection
 * visibility within the Shefa layer.
 *
 * Decouples the protection monitoring lifecycle (trust graphs, geographic
 * distribution, redundancy analysis, alerting) from the concrete persistence
 * layer. Consumers inject the DATA_PROTECTION token; the default factory
 * resolves to DataProtectionApiService.
 *
 * Tests provide a mock IDataProtection via the same token -- no concrete
 * service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class ProtectionDashboard {
 *   private readonly protection = inject(DATA_PROTECTION);
 *
 *   loadStatus(operatorId: string) {
 *     return this.protection.initializeProtectionMonitoring(operatorId);
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import { Observable } from 'rxjs';

import { DataProtectionApiService } from '../services/data-protection-api.service';

// =============================================================================
// TYPES
// =============================================================================

/**
 * Custodian type classification.
 */
export type CustodianType = 'family' | 'friend' | 'community' | 'professional' | 'institution';

/**
 * Redundancy strategy type.
 */
export type RedundancyStrategy = 'full_replica' | 'threshold_split' | 'erasure_coded';

/**
 * A node protecting the operator's data.
 */
export interface CustodianNode {
  id: string;
  name: string;
  type: CustodianType;
  location?: {
    region: string;
    country: string;
  };
  dataStored: {
    totalGB: number;
    shardCount: number;
    redundancyLevel: number;
  };
  health: {
    upPercent: number;
    lastHeartbeat: string;
    responseTime: number;
  };
  commitment: {
    id: string;
    status: 'active' | 'pending' | 'breached' | 'expired';
    startDate: string;
    expiryDate: string;
    renewalStatus: 'auto-renew' | 'manual' | 'expired';
  };
  trustLevel: number;
  relationship: string;
}

/**
 * Geographic distribution of custodians in a region.
 */
export interface RegionalPresence {
  region: string;
  custodianCount: number;
  dataShards: number;
  redundancy: number;
  riskFactors: string[];
}

/**
 * Trust relationship in the trust graph.
 */
export interface TrustRelationship {
  from: string;
  to: string;
  type: 'family-member' | 'friend' | 'community-peer' | 'professional' | 'institution';
  trustScore: number;
  depth: number;
  strength: 'weak' | 'moderate' | 'strong';
}

/**
 * Complete family-community protection status.
 */
export interface FamilyCommunityProtectionStatus {
  redundancy: {
    strategy: RedundancyStrategy;
    redundancyFactor: number;
    recoveryThreshold: number;
  };
  custodians: CustodianNode[];
  totalCustodians: number;
  geographicDistribution: {
    regions: RegionalPresence[];
    riskProfile: 'centralized' | 'distributed' | 'geo-redundant';
  };
  trustGraph: TrustRelationship[];
  protectionLevel: 'vulnerable' | 'protected' | 'highly-protected';
  estimatedRecoveryTime: string;
  lastVerification: string;
  verificationStatus: 'verified' | 'pending' | 'failed';
}

// =============================================================================
// INTERFACE
// =============================================================================

/**
 * Abstract data protection manager -- monitors family-community protection
 * status including trust graphs, geographic distribution, redundancy, and
 * recovery estimates.
 *
 * Implementations handle DHT queries for custodian commitments, health
 * monitoring, geographic risk analysis, and protection alerting.
 */
export interface IDataProtection {
  /**
   * Initialize protection monitoring for an operator.
   *
   * Returns an Observable that emits updated protection status on the
   * configured refresh interval.
   *
   * @param operatorId - Agent public key of the data owner
   * @param refreshInterval - Milliseconds between refreshes (default 60000)
   * @returns Observable stream of protection status updates
   */
  initializeProtectionMonitoring(
    operatorId: string,
    refreshInterval?: number
  ): Observable<FamilyCommunityProtectionStatus>;

  /**
   * Get current protection status snapshot.
   *
   * @returns Current status or null if not yet loaded
   */
  getProtectionStatus(): FamilyCommunityProtectionStatus | null;

  /**
   * Get protection status as an Observable.
   *
   * @returns Observable of current protection status
   */
  getProtectionStatus$(): Observable<FamilyCommunityProtectionStatus | null>;

  /**
   * Get custodians filtered by type.
   *
   * @param type - Custodian type to filter by
   * @returns Custodian nodes matching the type
   */
  getCustodiansByType(type: CustodianType): CustodianNode[];

  /**
   * Get regions with elevated risk.
   *
   * @returns Regions with risk factors or single-custodian coverage
   */
  getHighRiskRegions(): RegionalPresence[];

  /**
   * Check if a specific custodian is healthy.
   *
   * @param custodianId - Agent ID of the custodian
   * @returns True if uptime >= 95%
   */
  isCustodianHealthy(custodianId: string): boolean;

  /**
   * Get average uptime across all custodians.
   *
   * @returns Average uptime percentage (0-100)
   */
  getAverageUptime(): number;

  /**
   * Get protection alerts.
   *
   * @returns Alert messages for protection issues
   */
  getProtectionAlerts(): string[];
}

// =============================================================================
// INJECTION TOKEN
// =============================================================================

/**
 * Injection token for data protection monitoring.
 *
 * Default factory resolves to DataProtectionApiService which provides:
 * - Real-time protection monitoring via Observable streams
 * - Trust graph construction from custodian commitments
 * - Geographic distribution analysis with risk profiling
 * - Redundancy and recovery estimation
 * - Protection-level alerting
 *
 * Override in tests:
 * ```typescript
 * { provide: DATA_PROTECTION, useValue: mockDataProtection }
 * ```
 */
export const DATA_PROTECTION = new InjectionToken<IDataProtection>('DataProtection', {
  providedIn: 'root',
  factory: () => inject(DataProtectionApiService),
});
