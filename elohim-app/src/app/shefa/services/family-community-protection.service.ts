/**
 * Family-Community Protection Service
 *
 * Visualizes data protection by family, friends, and community members.
 * Provides three complementary views:
 *
 * 1. Trust Graph: Shows relationships and trust levels
 *    - Who's protecting my data?
 *    - How deep is the relationship (direct vs. through intermediaries)?
 *    - What's the trust level (weak/moderate/strong)?
 *
 * 2. Geographic Distribution: Shows where replicas exist
 *    - Which regions have custodians?
 *    - How many shards in each region?
 *    - What's the redundancy level?
 *    - Are there geopolitical risks?
 *
 * 3. Redundancy & Recovery: Shows data protection strength
 *    - What's the redundancy strategy (full replica, threshold split, erasure coded)?
 *    - How many shards needed to recover (2-of-3, 3-of-5, etc.)?
 *    - Estimated recovery time if a custodian goes down
 *    - Current protection level (vulnerable/protected/highly-protected)
 *
 * Integrates with:
 * - CustodianCommitment entries (Holochain DHT)
 * - Agent status service (heartbeat health)
 * - Geographic location service
 */

import { Injectable } from '@angular/core';
import { BehaviorSubject, Observable, combineLatest, of, interval } from 'rxjs';
import { map, switchMap, catchError, shareReplay, startWith } from 'rxjs/operators';

import {
  FamilyCommunityProtectionStatus,
  CustodianNode,
  RegionalPresence,
  TrustRelationship,
} from '../models/shefa-dashboard.model';

import { HolochainClientService } from '@elohim/services/holochain-client.service';

/**
 * Raw custodian commitment data from Holochain
 */
interface RawCustodianCommitment {
  id: string;
  steward_id: string;
  custodian_id: string;
  custodian_name?: string;
  custodian_type: 'family' | 'friend' | 'community' | 'professional' | 'institution';
  data_description?: string;
  total_gb: number;
  shard_count: number;
  shard_threshold?: number; // Minimum shards needed to recover
  redundancy_level: number; // 1, 2, 3, etc.
  strategy: 'full_replica' | 'threshold_split' | 'erasure_coded';
  status: 'active' | 'pending' | 'breached' | 'expired';
  relationship?: string;
  trust_level?: number;
  location?: {
    region: string;
    country: string;
    latitude?: number;
    longitude?: number;
  };
  start_date: string;
  expiry_date: string;
  renewal_status: 'auto-renew' | 'manual' | 'expired';
  created_at: string;
  updated_at: string;
}

/**
 * Agent status from heartbeat service
 */
interface AgentStatus {
  agent_id: string;
  uptime_percent: number;
  last_heartbeat: string;
  response_time_ms: number;
  consecutive_failures?: number;
}

/**
 * Geographic risk factors
 */
interface GeographicRisk {
  region: string;
  factors: string[]; // e.g., "single-isp", "geographic-clustering", "geopolitical-risk"
  riskScore: number; // 0-100
}

@Injectable({
  providedIn: 'root',
})
export class FamilyCommunityProtectionService {
  private protectionStatus$ = new BehaviorSubject<FamilyCommunityProtectionStatus | null>(null);
  private healthCheckInterval = 30000; // Check custodian health every 30 seconds

  constructor(private holochain: HolochainClientService) {}

  /**
   * Initialize protection monitoring for an operator
   * Returns Observable<FamilyCommunityProtectionStatus> that updates in real-time
   */
  initializeProtectionMonitoring(
    operatorId: string,
    refreshInterval: number = 60000 // Default: 1 minute
  ): Observable<FamilyCommunityProtectionStatus> {
    return interval(refreshInterval).pipe(
      startWith(0),
      switchMap(() => this.loadProtectionStatus(operatorId)),
      shareReplay(1)
    );
  }

  /**
   * Get current protection status
   */
  getProtectionStatus(): FamilyCommunityProtectionStatus | null {
    return this.protectionStatus$.value;
  }

  /**
   * Get protection status as Observable
   */
  getProtectionStatus$(): Observable<FamilyCommunityProtectionStatus | null> {
    return this.protectionStatus$.asObservable();
  }

  /**
   * Load complete protection status from DHT
   */
  private loadProtectionStatus(operatorId: string): Observable<FamilyCommunityProtectionStatus> {
    return this.holochain
      .callZome('content_store', 'get_custodian_commitments', { steward_id: operatorId })
      .pipe(
        switchMap((commitments: RawCustodianCommitment[]) => {
          if (!commitments || commitments.length === 0) {
            return of(this.getEmptyProtectionStatus());
          }

          // Load health status for each custodian
          const healthObs = commitments.map(c =>
            this.holochain
              .callZome('content_store', 'get_agent_status', { agent_id: c.custodian_id })
              .pipe(
                map(status => ({ commitment: c, status })),
                catchError(() =>
                  of({ commitment: c, status: null })
                )
              )
          );

          return combineLatest(healthObs);
        }),
        map((data: any[]) => this.buildProtectionStatus(operatorId, data)),
        catchError(error => {
          console.error('[FamilyCommunityProtectionService] Failed to load protection status:', error);
          return of(this.getEmptyProtectionStatus());
        })
      );
  }

  /**
   * Build complete protection status from commitments and health data
   */
  private buildProtectionStatus(
    operatorId: string,
    commitmentHealthPairs: Array<{ commitment: RawCustodianCommitment; status: AgentStatus | null }>
  ): FamilyCommunityProtectionStatus {
    // Convert to CustodianNode objects
    const custodians: CustodianNode[] = commitmentHealthPairs.map(pair => ({
      id: pair.commitment.custodian_id,
      name: pair.commitment.custodian_name || pair.commitment.custodian_id,
      type: pair.commitment.custodian_type,
      location: pair.commitment.location,
      dataStored: {
        totalGB: pair.commitment.total_gb,
        shardCount: pair.commitment.shard_count,
        redundancyLevel: pair.commitment.redundancy_level,
      },
      health: pair.status
        ? {
            upPercent: pair.status.uptime_percent,
            lastHeartbeat: pair.status.last_heartbeat,
            responseTime: pair.status.response_time_ms,
          }
        : {
            upPercent: 0,
            lastHeartbeat: 'never',
            responseTime: 0,
          },
      commitment: {
        id: pair.commitment.id,
        status: pair.commitment.status,
        startDate: pair.commitment.start_date,
        expiryDate: pair.commitment.expiry_date,
        renewalStatus: pair.commitment.renewal_status,
      },
      trustLevel: pair.commitment.trust_level || 50,
      relationship: pair.commitment.relationship || pair.commitment.custodian_type,
    }));

    // Calculate redundancy metrics
    const redundancyMetrics = this.calculateRedundancyMetrics(commitmentHealthPairs);

    // Build geographic distribution
    const geographicDistribution = this.buildGeographicDistribution(custodians);

    // Build trust graph
    const trustGraph = this.buildTrustGraph(operatorId, custodians);

    // Calculate overall protection level
    const protectionLevel = this.calculateProtectionLevel(custodians, redundancyMetrics);

    // Estimate recovery time
    const estimatedRecoveryTime = this.estimateRecoveryTime(custodians, redundancyMetrics);

    return {
      redundancy: redundancyMetrics,
      custodians,
      totalCustodians: custodians.length,
      geographicDistribution,
      trustGraph,
      protectionLevel,
      estimatedRecoveryTime,
      lastVerification: new Date().toISOString(),
      verificationStatus: custodians.length > 0 ? 'verified' : 'pending',
    };
  }

  /**
   * Calculate redundancy metrics from commitments
   */
  private calculateRedundancyMetrics(
    commitmentHealthPairs: Array<{ commitment: RawCustodianCommitment; status: AgentStatus | null }>
  ): { strategy: 'full_replica' | 'threshold_split' | 'erasure_coded'; redundancyFactor: number; recoveryThreshold: number } {
    if (commitmentHealthPairs.length === 0) {
      return {
        strategy: 'full_replica',
        redundancyFactor: 0,
        recoveryThreshold: 0,
      };
    }

    // Detect strategy (most common among commitments)
    const strategyCount = {
      full_replica: 0,
      threshold_split: 0,
      erasure_coded: 0,
    };

    commitmentHealthPairs.forEach(pair => {
      strategyCount[pair.commitment.strategy]++;
    });

    let strategy: 'full_replica' | 'threshold_split' | 'erasure_coded' = 'full_replica';
    if (strategyCount.erasure_coded > strategyCount.full_replica) strategy = 'erasure_coded';
    else if (strategyCount.threshold_split > strategyCount.full_replica) strategy = 'threshold_split';

    // Calculate average redundancy factor
    const avgRedundancy =
      commitmentHealthPairs.reduce((sum, pair) => sum + pair.commitment.redundancy_level, 0) / commitmentHealthPairs.length;

    // Calculate recovery threshold
    let recoveryThreshold: number;
    if (strategy === 'full_replica') {
      // Full replica: need at least 1 copy
      recoveryThreshold = 1;
    } else if (strategy === 'threshold_split') {
      // Threshold: need m-of-n shards
      const avgShardThreshold = commitmentHealthPairs.reduce(
        (sum, pair) => sum + (pair.commitment.shard_threshold || Math.ceil(pair.commitment.shard_count / 2)),
        0
      ) / commitmentHealthPairs.length;
      recoveryThreshold = Math.ceil(avgShardThreshold);
    } else {
      // Erasure coded: ~k of n (lower than threshold)
      recoveryThreshold = Math.ceil(avgRedundancy);
    }

    return {
      strategy,
      redundancyFactor: Math.ceil(avgRedundancy),
      recoveryThreshold: Math.ceil(recoveryThreshold),
    };
  }

  /**
   * Build geographic distribution view
   */
  private buildGeographicDistribution(custodians: CustodianNode[]): RegionalPresence[] {
    const regionMap = new Map<string, RegionalPresence>();

    custodians.forEach(custodian => {
      const region = custodian.location?.region || 'unknown';
      const key = `${region}-${custodian.location?.country || 'unknown'}`;

      if (!regionMap.has(key)) {
        regionMap.set(key, {
          region,
          custodianCount: 0,
          dataShards: 0,
          redundancy: 0,
          riskFactors: this.assessRegionRisks(region, custodian.location?.country),
        });
      }

      const presence = regionMap.get(key)!;
      presence.custodianCount++;
      presence.dataShards += custodian.dataStored.shardCount;
      presence.redundancy = Math.max(presence.redundancy, custodian.dataStored.redundancyLevel);
    });

    return Array.from(regionMap.values()).sort((a, b) => b.custodianCount - a.custodianCount);
  }

  /**
   * Assess geopolitical and infrastructure risks for a region
   */
  private assessRegionRisks(region: string, country?: string): string[] {
    const risks: string[] = [];

    // Check for geographic clustering
    // (In production, this would query actual custodian locations)
    if (region === 'unknown') {
      risks.push('location-unknown');
    }

    // Check for single-provider risks
    // (Would check if all custodians use same ISP)
    if (!country || country.length === 0) {
      risks.push('geopolitical-unknown');
    }

    return risks;
  }

  /**
   * Build trust relationship graph
   */
  private buildTrustGraph(operatorId: string, custodians: CustodianNode[]): TrustRelationship[] {
    return custodians.map(custodian => {
      // Determine trust strength from trust level
      let strength: 'weak' | 'moderate' | 'strong';
      if (custodian.trustLevel >= 80) {
        strength = 'strong';
      } else if (custodian.trustLevel >= 50) {
        strength = 'moderate';
      } else {
        strength = 'weak';
      }

      // Map custodian type to relationship type
      let type: TrustRelationship['type'];
      switch (custodian.type) {
        case 'family':
          type = 'family-member';
          break;
        case 'friend':
          type = 'friend';
          break;
        case 'community':
          type = 'community-peer';
          break;
        case 'professional':
          type = 'professional';
          break;
        case 'institution':
          type = 'institution';
          break;
      }

      return {
        from: operatorId,
        to: custodian.id,
        type,
        trustScore: custodian.trustLevel,
        depth: 1, // Direct relationship
        strength,
      };
    });
  }

  /**
   * Calculate overall protection level
   */
  private calculateProtectionLevel(
    custodians: CustodianNode[],
    redundancy: { strategy: string; redundancyFactor: number; recoveryThreshold: number }
  ): 'vulnerable' | 'protected' | 'highly-protected' {
    if (custodians.length === 0) return 'vulnerable';

    // High protection: 3+ custodians with good health and high redundancy
    const healthyCount = custodians.filter(c => c.health.upPercent >= 95).length;
    const highRedundancy = redundancy.redundancyFactor >= 2.5;

    if (custodians.length >= 3 && healthyCount >= 2 && highRedundancy) {
      return 'highly-protected';
    }

    // Protected: 2+ custodians with reasonable redundancy
    if (custodians.length >= 2 && redundancy.redundancyFactor >= 1.5) {
      return 'protected';
    }

    return 'vulnerable';
  }

  /**
   * Estimate data recovery time if a custodian goes down
   */
  private estimateRecoveryTime(
    custodians: CustodianNode[],
    redundancy: { strategy: string; redundancyFactor: number; recoveryThreshold: number }
  ): string {
    if (custodians.length === 0) return 'unable-to-recover';

    // Calculate based on number of custodians and recovery threshold
    const minResponseTime = Math.min(...custodians.map(c => c.health.responseTime));

    if (custodians.length >= 3 && redundancy.recoveryThreshold <= 2) {
      // 3+ replicas, need only 2: < 1 hour (quick shard recovery)
      return `< 1 hour`;
    } else if (custodians.length >= 2 && redundancy.recoveryThreshold <= 2) {
      // 2 replicas, threshold 2: < 4 hours
      return `< 4 hours`;
    } else if (custodians.length >= 2) {
      // 2 custodians, higher threshold: < 24 hours
      return `< 24 hours`;
    } else {
      // Only 1 custodian: needs recovery from single point of failure
      return `> 24 hours (single custodian)`;
    }
  }

  /**
   * Get custodians by type (family, friends, community, etc.)
   */
  getCustodiansByType(type: 'family' | 'friend' | 'community' | 'professional' | 'institution'): CustodianNode[] {
    const status = this.protectionStatus$.value;
    return status ? status.custodians.filter(c => c.type === type) : [];
  }

  /**
   * Get high-risk regions (few custodians, geographic clustering)
   */
  getHighRiskRegions(): RegionalPresence[] {
    const status = this.protectionStatus$.value;
    if (!status) return [];

    return status.geographicDistribution.filter(region => region.riskFactors.length > 0 || region.custodianCount === 1);
  }

  /**
   * Check if a specific custodian is healthy
   */
  isCustodianHealthy(custodianId: string): boolean {
    const status = this.protectionStatus$.value;
    if (!status) return false;

    const custodian = status.custodians.find(c => c.id === custodianId);
    return custodian ? custodian.health.upPercent >= 95 : false;
  }

  /**
   * Get average uptime across all custodians
   */
  getAverageUptime(): number {
    const status = this.protectionStatus$.value;
    if (!status || status.custodians.length === 0) return 0;

    const totalUptime = status.custodians.reduce((sum, c) => sum + c.health.upPercent, 0);
    return totalUptime / status.custodians.length;
  }

  /**
   * Alert if protection level drops below threshold
   */
  getProtectionAlerts(): string[] {
    const status = this.protectionStatus$.value;
    const alerts: string[] = [];

    if (!status) return alerts;

    if (status.protectionLevel === 'vulnerable') {
      alerts.push('Data protection is vulnerable - add more custodians');
    }

    if (status.custodians.length > 0) {
      const unhealthyCustodians = status.custodians.filter(c => c.health.upPercent < 95);
      if (unhealthyCustodians.length > 0) {
        alerts.push(`${unhealthyCustodians.length} custodian(s) have uptime < 95%`);
      }
    }

    const expiredCommitments = status.custodians.filter(c => c.commitment.status === 'expired');
    if (expiredCommitments.length > 0) {
      alerts.push(`${expiredCommitments.length} commitment(s) have expired`);
    }

    const soonToExpire = status.custodians.filter(c => {
      const expiryDate = new Date(c.commitment.expiryDate);
      const daysUntilExpiry = (expiryDate.getTime() - Date.now()) / (1000 * 60 * 60 * 24);
      return daysUntilExpiry < 30 && daysUntilExpiry > 0;
    });
    if (soonToExpire.length > 0) {
      alerts.push(`${soonToExpire.length} commitment(s) expire within 30 days`);
    }

    return alerts;
  }

  /**
   * Empty protection status
   */
  private getEmptyProtectionStatus(): FamilyCommunityProtectionStatus {
    return {
      redundancy: {
        strategy: 'full_replica',
        redundancyFactor: 0,
        recoveryThreshold: 0,
      },
      custodians: [],
      totalCustodians: 0,
      geographicDistribution: [],
      trustGraph: [],
      protectionLevel: 'vulnerable',
      estimatedRecoveryTime: 'unknown',
      lastVerification: new Date().toISOString(),
      verificationStatus: 'pending',
    };
  }
}
