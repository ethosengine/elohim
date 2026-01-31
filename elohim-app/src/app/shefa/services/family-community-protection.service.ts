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
 *
 * TODO: [HOLOCHAIN-ZOME] Zome call payloads in this service use snake_case
 * (e.g., steward_id, agent_id) because Holochain zomes are Rust and expect
 * snake_case field names. This cannot be changed without updating the Rust
 * zomes and running a DNA migration.
 */

import { Injectable } from '@angular/core';

import { map, switchMap, catchError, shareReplay, startWith } from 'rxjs/operators';

import { BehaviorSubject, Observable, combineLatest, of, interval, from } from 'rxjs';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

import {
  FamilyCommunityProtectionStatus,
  CustodianNode,
  RegionalPresence,
  TrustRelationship,
} from '../models/shefa-dashboard.model';

/**
 * Raw custodian commitment data from Holochain
 */
interface RawCustodianCommitment {
  id: string;
  stewardId: string;
  custodianId: string;
  custodianName?: string;
  custodianType: 'family' | 'friend' | 'community' | 'professional' | 'institution';
  dataDescription?: string;
  totalGb: number;
  shardCount: number;
  shardThreshold?: number; // Minimum shards needed to recover
  redundancyLevel: number; // 1, 2, 3, etc.
  strategy: 'full_replica' | 'threshold_split' | 'erasure_coded';
  status: 'active' | 'pending' | 'breached' | 'expired';
  relationship?: string;
  trustLevel?: number;
  location?: {
    region: string;
    country: string;
    latitude?: number;
    longitude?: number;
  };
  startDate: string;
  expiryDate: string;
  renewalStatus: 'auto-renew' | 'manual' | 'expired';
  createdAt: string;
  updatedAt: string;
}

/**
 * Agent status from heartbeat service
 */
interface AgentStatus {
  agentId: string;
  uptimePercent: number;
  lastHeartbeat: string;
  responseTimeMs: number;
  consecutiveFailures?: number;
}

@Injectable({
  providedIn: 'root',
})
export class FamilyCommunityProtectionService {
  private readonly protectionStatus$ = new BehaviorSubject<FamilyCommunityProtectionStatus | null>(
    null
  );
  private readonly healthCheckInterval = 30000; // Check custodian health every 30 seconds

  constructor(private readonly holochain: HolochainClientService) {}

  /**
   * Initialize protection monitoring for an operator
   * Returns Observable<FamilyCommunityProtectionStatus> that updates in real-time
   */
  initializeProtectionMonitoring(
    operatorId: string,
    refreshInterval = 60000 // Default: 1 minute
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
    return from(
      this.holochain.callZome<RawCustodianCommitment[]>({
        zomeName: 'content_store',
        fnName: 'get_custodian_commitments',
        payload: { steward_id: operatorId },
      })
    ).pipe(
      switchMap(result => {
        const commitments = result.success ? (result.data ?? []) : [];
        if (commitments.length === 0) {
          return of(this.getEmptyProtectionStatus());
        }

        // Load health status for each custodian
        const healthObs = commitments.map(c =>
          from(
            this.holochain.callZome<AgentStatus>({
              zomeName: 'content_store',
              fnName: 'get_agent_status',
              payload: { agent_id: c.custodianId },
            })
          ).pipe(
            map(statusResult => ({
              commitment: c,
              status: statusResult.success ? statusResult.data : null,
            })),
            catchError(() => of({ commitment: c, status: null }))
          )
        );

        return combineLatest(healthObs);
      }),
      map((data: { commitment: RawCustodianCommitment; status: AgentStatus | null }[]) =>
        this.buildProtectionStatus(operatorId, data)
      ),
      catchError(error => {
        return of(this.getEmptyProtectionStatus());
      })
    );
  }

  /**
   * Build complete protection status from commitments and health data
   */
  private buildProtectionStatus(
    operatorId: string,
    commitmentHealthPairs: { commitment: RawCustodianCommitment; status: AgentStatus | null }[]
  ): FamilyCommunityProtectionStatus {
    // Convert to CustodianNode objects
    const custodians: CustodianNode[] = commitmentHealthPairs.map(pair => ({
      id: pair.commitment.custodianId,
      name: pair.commitment.custodianName ?? pair.commitment.custodianId,
      type: pair.commitment.custodianType,
      location: pair.commitment.location,
      dataStored: {
        totalGB: pair.commitment.totalGb,
        shardCount: pair.commitment.shardCount,
        redundancyLevel: pair.commitment.redundancyLevel,
      },
      health: pair.status
        ? {
            upPercent: pair.status.uptimePercent,
            lastHeartbeat: pair.status.lastHeartbeat,
            responseTime: pair.status.responseTimeMs,
          }
        : {
            upPercent: 0,
            lastHeartbeat: 'never',
            responseTime: 0,
          },
      commitment: {
        id: pair.commitment.id,
        status: pair.commitment.status,
        startDate: pair.commitment.startDate,
        expiryDate: pair.commitment.expiryDate,
        renewalStatus: pair.commitment.renewalStatus,
      },
      trustLevel: pair.commitment.trustLevel ?? 50,
      relationship: pair.commitment.relationship ?? pair.commitment.custodianType,
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

    // Calculate risk profile based on regional distribution
    const riskProfile = this.calculateGeographicRiskProfile(geographicDistribution);

    return {
      redundancy: redundancyMetrics,
      custodians,
      totalCustodians: custodians.length,
      geographicDistribution: {
        regions: geographicDistribution,
        riskProfile,
      },
      trustGraph,
      protectionLevel,
      estimatedRecoveryTime,
      lastVerification: new Date().toISOString(),
      verificationStatus: custodians.length > 0 ? 'verified' : 'pending',
    };
  }

  /**
   * Calculate geographic risk profile based on distribution
   */
  private calculateGeographicRiskProfile(
    regions: RegionalPresence[]
  ): 'centralized' | 'distributed' | 'geo-redundant' {
    if (regions.length === 0) return 'centralized';
    if (regions.length === 1) return 'centralized';
    if (regions.length >= 3) return 'geo-redundant';
    return 'distributed';
  }

  /**
   * Calculate redundancy metrics from commitments
   */
  private calculateRedundancyMetrics(
    commitmentHealthPairs: { commitment: RawCustodianCommitment; status: AgentStatus | null }[]
  ): {
    strategy: 'full_replica' | 'threshold_split' | 'erasure_coded';
    redundancyFactor: number;
    recoveryThreshold: number;
  } {
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
    else if (strategyCount.threshold_split > strategyCount.full_replica)
      strategy = 'threshold_split';

    // Calculate average redundancy factor
    const avgRedundancy =
      commitmentHealthPairs.reduce((sum, pair) => sum + pair.commitment.redundancyLevel, 0) /
      commitmentHealthPairs.length;

    // Calculate recovery threshold
    let recoveryThreshold: number;
    if (strategy === 'full_replica') {
      // Full replica: need at least 1 copy
      recoveryThreshold = 1;
    } else if (strategy === 'threshold_split') {
      // Threshold: need m-of-n shards
      const avgShardThreshold =
        commitmentHealthPairs.reduce(
          (sum, pair) =>
            sum + (pair.commitment.shardThreshold ?? Math.ceil(pair.commitment.shardCount / 2)),
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
      const region = custodian.location?.region ?? 'unknown';
      const key = `${region}-${custodian.location?.country ?? 'unknown'}`;

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
  getCustodiansByType(
    type: 'family' | 'friend' | 'community' | 'professional' | 'institution'
  ): CustodianNode[] {
    const status = this.protectionStatus$.value;
    return status ? status.custodians.filter(c => c.type === type) : [];
  }

  /**
   * Get high-risk regions (few custodians, geographic clustering)
   */
  getHighRiskRegions(): RegionalPresence[] {
    const status = this.protectionStatus$.value;
    if (!status) return [];

    return status.geographicDistribution.regions.filter(
      region => region.riskFactors.length > 0 || region.custodianCount === 1
    );
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
      geographicDistribution: {
        regions: [],
        riskProfile: 'centralized',
      },
      trustGraph: [],
      protectionLevel: 'vulnerable',
      estimatedRecoveryTime: 'unknown',
      lastVerification: new Date().toISOString(),
      verificationStatus: 'pending',
    };
  }
}
