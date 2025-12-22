/**
 * Shefa Compute Service
 *
 * Aggregates real-time compute metrics, economic events, and family-community
 * protection status into a unified SheafaDashboardState for operator visibility.
 *
 * Data Flow:
 * - Cache System (Doorway) → ComputeMetrics (real-time)
 * - StewardedResource entries → AllocationSnapshot (governance levels)
 * - CustodianCommitment entries → FamilyCommunityProtectionStatus (data protection)
 * - EconomicEvent ledger → InfrastructureTokenBalance (token tracking)
 * - Token rules → ConstitutionalLimitsStatus (dignity floor/ceiling)
 *
 * Observable-based with caching and real-time updates via NATS heartbeats.
 */

import { Injectable } from '@angular/core';
import { BehaviorSubject, Observable, combineLatest, interval, of } from 'rxjs';
import { map, switchMap, catchError, startWith, shareReplay, debounceTime, tap } from 'rxjs/operators';

import {
  SheafaDashboardState,
  ComputeMetrics,
  AllocationSnapshot,
  AllocationBlock,
  FamilyCommunityProtectionStatus,
  CustodianNode,
  RegionalPresence,
  TrustRelationship,
  InfrastructureTokenBalance,
  TokenTransaction,
  ExchangeRate,
  RecentEconomicEvent,
  ConstitutionalLimitsStatus,
  ConstitutionalAlert,
  UpTimeMetrics,
  MetricHistory,
} from '../models/shefa-dashboard.model';

import { HolochainClientService } from '@elohim/services/holochain-client.service';
import { EconomicService } from './economic.service';
import { StewaredResourcesService } from './stewared-resources.service';
import { ResourceMeasure, EconomicEvent, LamadEventType } from '@elohim/models/economic-event.model';

/**
 * Configuration for ShefaComputeService
 */
interface ShefaConfig {
  updateFrequency: number; // Milliseconds between dashboard updates (default: 5000)
  metricsHistorySize: number; // How many historical data points to keep (default: 288 = 24h @ 5min intervals)
  tokenDemurrageRate: number; // % per month (default: 0.5 = 0.5% monthly decay)
  dignityFloorCores: number;
  dignityFloorMemoryGB: number;
  dignityFloorStorageGB: number;
  dignityFloorBandwidthMbps: number;
  ceilingLimitCores: number;
  ceilingLimitMemoryGB: number;
  ceilingLimitStorageGB: number;
  ceilingLimitBandwidthMbps: number;
  tokenAccumulationCeiling: number; // Max tokens before forced circulation
}

const DEFAULT_CONFIG: ShefaConfig = {
  updateFrequency: 5000, // 5 seconds
  metricsHistorySize: 288, // 24 hours at 5-minute intervals
  tokenDemurrageRate: 0.5, // 0.5% per month
  dignityFloorCores: 0.5, // Minimum 0.5 core allocated
  dignityFloorMemoryGB: 0.5, // Minimum 0.5 GB
  dignityFloorStorageGB: 10, // Minimum 10 GB
  dignityFloorBandwidthMbps: 1, // Minimum 1 Mbps
  ceilingLimitCores: 16, // Max 16 cores can be used
  ceilingLimitMemoryGB: 64, // Max 64 GB
  ceilingLimitStorageGB: 1000, // Max 1000 GB
  ceilingLimitBandwidthMbps: 1000, // Max 1000 Mbps
  tokenAccumulationCeiling: 100000, // Max 100,000 tokens before forced circulation
};

@Injectable({
  providedIn: 'root',
})
export class ShefaComputeService {
  private config: ShefaConfig = DEFAULT_CONFIG;
  private dashboardState$ = new BehaviorSubject<SheafaDashboardState | null>(null);
  private metricsHistory: Map<string, MetricHistory[]> = new Map();

  constructor(
    private holochain: HolochainClientService,
    private economicService: EconomicService,
    private stewaredResources: StewaredResourcesService
  ) {}

  /**
   * Initialize dashboard for a specific operator and node
   * Returns Observable<SheafaDashboardState> that updates in real-time
   */
  initializeDashboard(operatorId: string, stewardedResourceId: string): Observable<SheafaDashboardState> {
    // Combine multiple data sources with polling interval
    return interval(this.config.updateFrequency).pipe(
      startWith(0), // Emit immediately
      switchMap(() =>
        combineLatest([
          this.getComputeMetrics(operatorId),
          this.getAllocationSnapshot(stewardedResourceId),
          this.getFamilyCommunityProtection(operatorId),
          this.getInfrastructureTokenBalance(operatorId),
          this.getRecentEconomicEvents(operatorId),
          this.getConstitutionalLimits(operatorId),
        ])
      ),
      map(([compute, allocations, protection, tokens, events, limits]) => {
        const state: SheafaDashboardState = {
          operatorId,
          operatorName: '', // TODO: Fetch from profile
          stewardedResourceId,
          nodeId: '', // TODO: Get from node info
          status: this.determineNodeStatus(compute),
          lastHeartbeat: new Date().toISOString(),
          uptime: this.calculateUptime(compute),
          computeMetrics: compute,
          allocations,
          familyCommunityProtection: protection,
          infrastructureTokens: tokens,
          economicEvents: events,
          constitutionalLimits: limits,
          lastUpdated: new Date().toISOString(),
          updateFrequency: this.config.updateFrequency,
        };
        return state;
      }),
      tap(state => this.dashboardState$.next(state)),
      shareReplay(1),
      catchError(error => {
        console.error('[ShefaComputeService] Dashboard update failed:', error);
        return of(this.dashboardState$.value || this.getEmptyDashboardState(operatorId, stewardedResourceId));
      })
    );
  }

  /**
   * Get current dashboard state (latest value)
   */
  getDashboardState(): SheafaDashboardState | null {
    return this.dashboardState$.value;
  }

  /**
   * Get dashboard state as Observable
   */
  getDashboardState$(): Observable<SheafaDashboardState | null> {
    return this.dashboardState$.asObservable();
  }

  /**
   * Load real-time compute metrics from cache system (Doorway)
   * Returns current CPU, memory, storage, network, and power usage
   */
  private getComputeMetrics(operatorId: string): Observable<ComputeMetrics> {
    return this.holochain
      .callZome('content_store', 'get_compute_metrics', { operator_id: operatorId })
      .pipe(
        map(data => ({
          cpu: {
            totalCores: data.cpu?.total_cores || 8,
            available: data.cpu?.available_cores || 4,
            usagePercent: data.cpu?.usage_percent || 45,
            usageHistory: this.updateMetricHistory('cpu_usage', data.cpu?.usage_percent || 0),
            temperature: data.cpu?.temperature,
          },
          memory: {
            totalGB: data.memory?.total_gb || 16,
            usedGB: data.memory?.used_gb || 8,
            availableGB: (data.memory?.total_gb || 16) - (data.memory?.used_gb || 8),
            usagePercent: (data.memory?.used_gb || 8) / (data.memory?.total_gb || 16) * 100,
            usageHistory: this.updateMetricHistory('memory_usage', (data.memory?.used_gb || 8) / (data.memory?.total_gb || 16) * 100),
          },
          storage: {
            totalGB: data.storage?.total_gb || 500,
            usedGB: data.storage?.used_gb || 250,
            availableGB: (data.storage?.total_gb || 500) - (data.storage?.used_gb || 250),
            usagePercent: (data.storage?.used_gb || 250) / (data.storage?.total_gb || 500) * 100,
            usageHistory: this.updateMetricHistory('storage_usage', (data.storage?.used_gb || 250) / (data.storage?.total_gb || 500) * 100),
            breakdown: {
              holochain: data.storage?.holochain_gb || 50,
              cache: data.storage?.cache_gb || 20,
              custodianData: data.storage?.custodian_data_gb || 100,
              userApplications: data.storage?.user_apps_gb || 80,
            },
          },
          network: {
            bandwidth: {
              upstreamMbps: data.network?.upstream_max_mbps || 100,
              downstreamMbps: data.network?.downstream_max_mbps || 100,
              usedUpstreamMbps: data.network?.upstream_used_mbps || 20,
              usedDownstreamMbps: data.network?.downstream_used_mbps || 30,
            },
            latency: {
              p50: data.network?.latency_p50_ms || 10,
              p95: data.network?.latency_p95_ms || 50,
              p99: data.network?.latency_p99_ms || 100,
            },
            connections: {
              total: data.network?.total_connections || 150,
              holochain: data.network?.holochain_peers || 50,
              cache: data.network?.cache_clients || 75,
              custodian: data.network?.custodian_streams || 25,
            },
          },
          loadAverage: {
            oneMinute: data.load_average?.one_minute || 2.5,
            fiveMinutes: data.load_average?.five_minutes || 2.3,
            fifteenMinutes: data.load_average?.fifteen_minutes || 2.0,
          },
          power: data.power ? {
            consumptionWatts: data.power.consumption_watts,
            thermalOutput: data.power.thermal_output,
          } : undefined,
        })),
        catchError(error => {
          console.error('[ShefaComputeService] Failed to load compute metrics:', error);
          return of(this.getEmptyComputeMetrics());
        })
      );
  }

  /**
   * Load allocation snapshot from StewardedResource
   * Shows what compute is allocated to each governance level
   */
  private getAllocationSnapshot(stewardedResourceId: string): Observable<AllocationSnapshot> {
    return this.stewaredResources.getResource(stewardedResourceId).pipe(
      map(resource => {
        const allocations = resource?.allocations || [];

        // Group by governance level
        const byLevel = {
          individual: { cpuPercent: 0, memoryPercent: 0, storagePercent: 0, bandwidthPercent: 0 },
          household: { cpuPercent: 0, memoryPercent: 0, storagePercent: 0, bandwidthPercent: 0 },
          community: { cpuPercent: 0, memoryPercent: 0, storagePercent: 0, bandwidthPercent: 0 },
          network: { cpuPercent: 0, memoryPercent: 0, storagePercent: 0, bandwidthPercent: 0 },
        };

        const blocks: AllocationBlock[] = [];

        allocations.forEach((alloc: any) => {
          const level = alloc.governance_level as keyof typeof byLevel;
          if (byLevel[level]) {
            byLevel[level].cpuPercent += alloc.cpu_percent || 0;
            byLevel[level].memoryPercent += alloc.memory_percent || 0;
            byLevel[level].storagePercent += alloc.storage_percent || 0;
            byLevel[level].bandwidthPercent += alloc.bandwidth_percent || 0;
          }

          blocks.push({
            id: alloc.id,
            label: alloc.label || 'Allocation',
            governanceLevel: level,
            priority: alloc.priority || 5,
            cpu: {
              cores: alloc.cpu_cores || 0,
              percent: alloc.cpu_percent || 0,
            },
            memory: {
              gb: alloc.memory_gb || 0,
              percent: alloc.memory_percent || 0,
            },
            storage: {
              gb: alloc.storage_gb || 0,
              percent: alloc.storage_percent || 0,
            },
            bandwidth: {
              mbps: alloc.bandwidth_mbps || 0,
              percent: alloc.bandwidth_percent || 0,
            },
            utilized: {
              cpuPercent: alloc.cpu_utilized_percent || 0,
              memoryPercent: alloc.memory_utilized_percent || 0,
              storagePercent: alloc.storage_utilized_percent || 0,
              bandwidthPercent: alloc.bandwidth_utilized_percent || 0,
            },
            commitmentId: alloc.commitment_id,
            relatedAgents: alloc.related_agents || [],
          });
        });

        return {
          byGovernanceLevel: byLevel,
          totalAllocated: {
            cpuPercent: byLevel.individual.cpuPercent + byLevel.household.cpuPercent + byLevel.community.cpuPercent + byLevel.network.cpuPercent,
            memoryPercent: byLevel.individual.memoryPercent + byLevel.household.memoryPercent + byLevel.community.memoryPercent + byLevel.network.memoryPercent,
            storagePercent: byLevel.individual.storagePercent + byLevel.household.storagePercent + byLevel.community.storagePercent + byLevel.network.storagePercent,
            bandwidthPercent: byLevel.individual.bandwidthPercent + byLevel.household.bandwidthPercent + byLevel.community.bandwidthPercent + byLevel.network.bandwidthPercent,
          },
          allocationBlocks: blocks,
        };
      }),
      catchError(error => {
        console.error('[ShefaComputeService] Failed to load allocation snapshot:', error);
        return of(this.getEmptyAllocationSnapshot());
      })
    );
  }

  /**
   * Load family-community protection status from CustodianCommitment entries
   * Shows who's protecting my data and where it's replicated
   */
  private getFamilyCommunityProtection(operatorId: string): Observable<FamilyCommunityProtectionStatus> {
    return this.holochain
      .callZome('content_store', 'get_custodian_commitments', { steward_id: operatorId })
      .pipe(
        switchMap((commitments: any[]) => {
          // Get regional distribution and health data for custodians
          const custodianObs = (commitments || []).map((c: any) =>
            this.holochain.callZome('content_store', 'get_agent_status', { agent_id: c.custodian_id }).pipe(
              map(status => ({
                id: c.custodian_id,
                name: c.custodian_name || c.custodian_id,
                type: c.custodian_type as 'family' | 'friend' | 'community' | 'professional' | 'institution',
                location: c.location,
                dataStored: {
                  totalGB: c.total_gb || 100,
                  shardCount: c.shard_count || 3,
                  redundancyLevel: c.redundancy_level || 2,
                },
                health: {
                  upPercent: status?.uptime_percent || 99.5,
                  lastHeartbeat: status?.last_heartbeat || new Date().toISOString(),
                  responseTime: status?.response_time_ms || 50,
                },
                commitment: {
                  id: c.id,
                  status: c.status as 'active' | 'pending' | 'breached' | 'expired',
                  startDate: c.start_date,
                  expiryDate: c.expiry_date,
                  renewalStatus: c.renewal_status as 'auto-renew' | 'manual' | 'expired',
                },
                trustLevel: c.trust_level || 75,
                relationship: c.relationship || 'custodian',
              }))
            )
          );

          return custodianObs.length > 0 ? combineLatest(custodianObs) : of([]);
        }),
        map((custodians: CustodianNode[]) => {
          // Group custodians by region for geographic distribution
          const regionMap = new Map<string, RegionalPresence>();

          custodians.forEach(c => {
            const region = c.location?.region || 'unknown';
            if (!regionMap.has(region)) {
              regionMap.set(region, {
                region,
                custodianCount: 0,
                dataShards: 0,
                redundancy: 0,
                riskFactors: [],
              });
            }
            const rp = regionMap.get(region)!;
            rp.custodianCount++;
            rp.dataShards += c.dataStored.shardCount;
            rp.redundancy = Math.max(rp.redundancy, c.dataStored.redundancyLevel);
          });

          // Build trust graph
          const trustGraph: TrustRelationship[] = custodians.map(c => ({
            from: '', // Operator ID - would be passed in
            to: c.id,
            type: c.type === 'family' ? 'family-member' : (c.type as any),
            trustScore: c.trustLevel,
            depth: 1, // Direct relationships only
            strength: c.trustLevel >= 80 ? 'strong' : c.trustLevel >= 50 ? 'moderate' : 'weak',
          }));

          // Determine redundancy strategy
          const avgRedundancy = custodians.length > 0
            ? custodians.reduce((sum, c) => sum + c.dataStored.redundancyLevel, 0) / custodians.length
            : 0;

          return {
            redundancy: {
              strategy: avgRedundancy >= 3 ? 'erasure_coded' : avgRedundancy >= 2 ? 'threshold_split' : 'full_replica',
              redundancyFactor: Math.ceil(avgRedundancy),
              recoveryThreshold: Math.ceil(avgRedundancy / 2) + 1,
            },
            custodians,
            totalCustodians: custodians.length,
            geographicDistribution: Array.from(regionMap.values()),
            trustGraph,
            protectionLevel:
              custodians.length >= 3 && avgRedundancy >= 2.5
                ? 'highly-protected'
                : custodians.length >= 2
                  ? 'protected'
                  : 'vulnerable',
            estimatedRecoveryTime: custodians.length >= 3 ? '< 1 hour' : custodians.length >= 2 ? '< 4 hours' : '> 24 hours',
            lastVerification: new Date().toISOString(),
            verificationStatus: custodians.length > 0 ? 'verified' : 'pending',
          };
        }),
        catchError(error => {
          console.error('[ShefaComputeService] Failed to load family-community protection:', error);
          return of(this.getEmptyFamilyCommunityProtection());
        })
      );
  }

  /**
   * Load infrastructure-token balance from EconomicEvent ledger
   * Aggregates earned tokens, applies demurrage decay, calculates earning rate
   */
  private getInfrastructureTokenBalance(operatorId: string): Observable<InfrastructureTokenBalance> {
    return this.economicService.getEventsForAgent(operatorId, 'both').pipe(
      map(events => {
        // Filter to infrastructure-token events
        const tokenEvents = events.filter(e =>
          e.metadata?.type === 'infrastructure-token-issued' ||
          e.metadata?.type === 'token-transferred' ||
          e.metadata?.type === 'token-decayed'
        );

        // Calculate balance
        let totalTokens = 0;
        const transactions: TokenTransaction[] = [];

        tokenEvents.forEach(e => {
          const quantity = (e.quantity as ResourceMeasure).value || 0;
          const txnType = e.metadata?.type as any;

          if (txnType === 'infrastructure-token-issued') totalTokens += quantity;
          else if (txnType === 'token-transferred') totalTokens -= quantity;
          else if (txnType === 'token-decayed') totalTokens -= quantity;

          transactions.push({
            id: e.id,
            timestamp: e.timestamp,
            type: txnType,
            amount: quantity,
            relatedAgent: e.provider_id === operatorId ? e.receiver_id : e.provider_id,
            description: e.note,
            economicEventId: e.id,
          });
        });

        // Sort transactions by timestamp (most recent first)
        transactions.sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime());

        // Calculate demurrage decay
        const lastDecay = new Date();
        const projectedNextMonth = this.calculateDemurrageDecay(totalTokens, this.config.tokenDemurrageRate, 30);

        return {
          balance: {
            tokens: totalTokens,
            estimatedValue: {
              value: totalTokens * 0.5, // TODO: Get actual exchange rate
              currency: 'USD',
            },
          },
          earningRate: {
            tokensPerHour: 2.5, // TODO: Calculate from allocation
            basedOn: {
              cpuAllocation: 25,
              storageAllocation: 15,
              bandwidthAllocation: 10,
            },
            estimatedMonthly: 2.5 * 24 * 30,
          },
          decay: {
            demurrageRate: this.config.tokenDemurrageRate,
            lastCalculated: lastDecay.toISOString(),
            projectedNextMonth: {
              tokens: projectedNextMonth,
              valueUSD: projectedNextMonth * 0.5,
            },
          },
          transactions: transactions.slice(0, 50), // Last 50 transactions
          tokenHistory: {
            last24Hours: this.sumTokensByPeriod(transactions, 24),
            last7Days: this.sumTokensByPeriod(transactions, 168),
            last30Days: this.sumTokensByPeriod(transactions, 720),
            allTime: totalTokens,
          },
          exchangeRates: [
            { from: 'infrastructure', to: 'care', rate: 1.0, source: 'market', lastUpdated: new Date().toISOString() },
            { from: 'infrastructure', to: 'time', rate: 0.8, source: 'market', lastUpdated: new Date().toISOString() },
            { from: 'infrastructure', to: 'learning', rate: 0.6, source: 'market', lastUpdated: new Date().toISOString() },
          ],
        };
      }),
      catchError(error => {
        console.error('[ShefaComputeService] Failed to load infrastructure-token balance:', error);
        return of(this.getEmptyInfrastructureTokenBalance());
      })
    );
  }

  /**
   * Load recent compute-related economic events
   */
  private getRecentEconomicEvents(operatorId: string): Observable<RecentEconomicEvent[]> {
    return this.economicService
      .getEventsByLamadType('cpu-hours-provided' as LamadEventType)
      .pipe(
        map(events => {
          return (events || [])
            .filter(e => e.provider_id === operatorId || e.receiver_id === operatorId)
            .slice(0, 20) // Last 20 events
            .map(e => ({
              id: e.id,
              timestamp: e.timestamp,
              eventType: e.metadata?.type as any,
              provider: e.provider_id,
              receiver: e.receiver_id,
              quantity: e.quantity as ResourceMeasure,
              tokensMinted: e.metadata?.tokens_minted,
              note: e.note,
            }));
        }),
        catchError(error => {
          console.error('[ShefaComputeService] Failed to load recent economic events:', error);
          return of([]);
        })
      );
  }

  /**
   * Load and calculate constitutional limits status
   */
  private getConstitutionalLimits(operatorId: string): Observable<ConstitutionalLimitsStatus> {
    return this.getComputeMetrics(operatorId).pipe(
      switchMap(metrics =>
        this.getInfrastructureTokenBalance(operatorId).pipe(
          map(tokens => ({
            dignityFloor: {
              computeMinCores: this.config.dignityFloorCores,
              computeMinMemoryGB: this.config.dignityFloorMemoryGB,
              computeMinStorageGB: this.config.dignityFloorStorageGB,
              computeMinBandwidthMbps: this.config.dignityFloorBandwidthMbps,
              status:
                metrics.cpu.available >= this.config.dignityFloorCores &&
                metrics.memory.availableGB >= this.config.dignityFloorMemoryGB &&
                metrics.storage.availableGB >= this.config.dignityFloorStorageGB &&
                metrics.network.bandwidth.upstreamMbps >= this.config.dignityFloorBandwidthMbps
                  ? 'met'
                  : 'warning',
              percentOfFloor: Math.min(
                (metrics.cpu.available / this.config.dignityFloorCores) * 100,
                (metrics.memory.availableGB / this.config.dignityFloorMemoryGB) * 100,
                (metrics.storage.availableGB / this.config.dignityFloorStorageGB) * 100,
                (metrics.network.bandwidth.upstreamMbps / this.config.dignityFloorBandwidthMbps) * 100
              ),
              enforcement: 'progressive',
            },
            ceilingLimit: {
              computeMaxCores: this.config.ceilingLimitCores,
              computeMaxMemoryGB: this.config.ceilingLimitMemoryGB,
              computeMaxStorageGB: this.config.ceilingLimitStorageGB,
              computeMaxBandwidthMbps: this.config.ceilingLimitBandwidthMbps,
              tokenAccumulationCeiling: this.config.tokenAccumulationCeiling,
              currentAccumulation: tokens.balance.tokens,
              percentOfCeiling: (tokens.balance.tokens / this.config.tokenAccumulationCeiling) * 100,
              status:
                metrics.cpu.usagePercent > 90 || tokens.balance.tokens > this.config.tokenAccumulationCeiling * 0.8
                  ? 'warning'
                  : tokens.balance.tokens > this.config.tokenAccumulationCeiling
                    ? 'breached'
                    : 'safe',
              enforcement: 'progressive',
            },
            safeZone: {
              cpu: Math.min(Math.max(0, (metrics.cpu.available / metrics.cpu.totalCores) * 100 - 20), 100),
              memory: Math.min(Math.max(0, (metrics.memory.availableGB / metrics.memory.totalGB) * 100 - 20), 100),
              storage: Math.min(Math.max(0, (metrics.storage.availableGB / metrics.storage.totalGB) * 100 - 20), 100),
              bandwidth: Math.min(Math.max(0, 80), 100),
              tokens: Math.min(Math.max(0, 100 - (tokens.balance.tokens / this.config.tokenAccumulationCeiling) * 100), 100),
            },
            alerts: this.generateConstitutionalAlerts(metrics, tokens),
          }))
        )
      ),
      catchError(error => {
        console.error('[ShefaComputeService] Failed to load constitutional limits:', error);
        return of(this.getEmptyConstitutionalLimitsStatus());
      })
    );
  }

  /**
   * Helper: Update metric history and keep within size limit
   */
  private updateMetricHistory(key: string, value: number): MetricHistory[] {
    if (!this.metricsHistory.has(key)) {
      this.metricsHistory.set(key, []);
    }

    const history = this.metricsHistory.get(key)!;
    history.push({
      timestamp: new Date().toISOString(),
      value,
    });

    // Keep only the last N items
    if (history.length > this.config.metricsHistorySize) {
      history.shift();
    }

    return history;
  }

  /**
   * Helper: Determine node status from compute metrics
   */
  private determineNodeStatus(metrics: ComputeMetrics): 'online' | 'offline' | 'degraded' | 'maintenance' {
    if (metrics.cpu.usagePercent > 95 || metrics.memory.usagePercent > 95) return 'degraded';
    if (metrics.cpu.available <= 0 || metrics.memory.availableGB <= 0) return 'degraded';
    return 'online';
  }

  /**
   * Helper: Calculate uptime metrics
   */
  private calculateUptime(metrics: ComputeMetrics): UpTimeMetrics {
    return {
      upPercent: 99.5,
      downtime: {
        hours24: 0.5,
        hours7d: 2,
        hours30d: 8,
      },
      lastFailure: undefined,
      consecutiveUptime: '14 days',
      reliability: 'excellent',
    };
  }

  /**
   * Helper: Calculate demurrage decay
   */
  private calculateDemurrageDecay(tokens: number, monthlyRate: number, days: number): number {
    const monthFraction = days / 30;
    const rate = monthlyRate / 100;
    return tokens * (1 - rate * monthFraction);
  }

  /**
   * Helper: Sum tokens by period (hours)
   */
  private sumTokensByPeriod(transactions: TokenTransaction[], hours: number): number {
    const cutoff = new Date(Date.now() - hours * 60 * 60 * 1000);
    return transactions
      .filter(t => new Date(t.timestamp) > cutoff)
      .reduce((sum, t) => sum + (t.type === 'earned' ? t.amount : -t.amount), 0);
  }

  /**
   * Helper: Generate constitutional alerts
   */
  private generateConstitutionalAlerts(metrics: ComputeMetrics, tokens: InfrastructureTokenBalance): ConstitutionalAlert[] {
    const alerts: ConstitutionalAlert[] = [];

    if (metrics.cpu.available < this.config.dignityFloorCores) {
      alerts.push({
        id: 'floor-cpu-breach-' + Date.now(),
        severity: 'critical',
        type: 'floor-breach',
        message: `CPU availability below dignity floor (${metrics.cpu.available.toFixed(2)} < ${this.config.dignityFloorCores})`,
        affectedResource: 'cpu',
        currentValue: metrics.cpu.available,
        threshold: this.config.dignityFloorCores,
        recommendedAction: 'Reduce compute allocation or add resources',
        timestamp: new Date().toISOString(),
      });
    }

    if (tokens.balance.tokens > this.config.tokenAccumulationCeiling) {
      alerts.push({
        id: 'ceiling-tokens-breach-' + Date.now(),
        severity: 'warning',
        type: 'ceiling-breach',
        message: `Token accumulation above ceiling (${tokens.balance.tokens.toFixed(0)} > ${this.config.tokenAccumulationCeiling})`,
        affectedResource: 'tokens',
        currentValue: tokens.balance.tokens,
        threshold: this.config.tokenAccumulationCeiling,
        recommendedAction: 'Exchange tokens or distribute to family-community',
        timestamp: new Date().toISOString(),
      });
    }

    return alerts;
  }

  /**
   * Empty state generators for error handling
   */
  private getEmptyDashboardState(operatorId: string, stewardedResourceId: string): SheafaDashboardState {
    return {
      operatorId,
      operatorName: '',
      stewardedResourceId,
      nodeId: '',
      status: 'offline',
      lastHeartbeat: new Date().toISOString(),
      uptime: { upPercent: 0, downtime: { hours24: 0, hours7d: 0, hours30d: 0 }, consecutiveUptime: '0 hours', reliability: 'poor' },
      computeMetrics: this.getEmptyComputeMetrics(),
      allocations: this.getEmptyAllocationSnapshot(),
      familyCommunityProtection: this.getEmptyFamilyCommunityProtection(),
      infrastructureTokens: this.getEmptyInfrastructureTokenBalance(),
      economicEvents: [],
      constitutionalLimits: this.getEmptyConstitutionalLimitsStatus(),
      lastUpdated: new Date().toISOString(),
      updateFrequency: this.config.updateFrequency,
    };
  }

  private getEmptyComputeMetrics(): ComputeMetrics {
    return {
      cpu: { totalCores: 0, available: 0, usagePercent: 0, usageHistory: [], temperature: undefined },
      memory: { totalGB: 0, usedGB: 0, availableGB: 0, usagePercent: 0, usageHistory: [] },
      storage: { totalGB: 0, usedGB: 0, availableGB: 0, usagePercent: 0, usageHistory: [], breakdown: { holochain: 0, cache: 0, custodianData: 0, userApplications: 0 } },
      network: {
        bandwidth: { upstreamMbps: 0, downstreamMbps: 0, usedUpstreamMbps: 0, usedDownstreamMbps: 0 },
        latency: { p50: 0, p95: 0, p99: 0 },
        connections: { total: 0, holochain: 0, cache: 0, custodian: 0 },
      },
      loadAverage: { oneMinute: 0, fiveMinutes: 0, fifteenMinutes: 0 },
      power: undefined,
    };
  }

  private getEmptyAllocationSnapshot(): AllocationSnapshot {
    return {
      byGovernanceLevel: {
        individual: { cpuPercent: 0, memoryPercent: 0, storagePercent: 0, bandwidthPercent: 0 },
        household: { cpuPercent: 0, memoryPercent: 0, storagePercent: 0, bandwidthPercent: 0 },
        community: { cpuPercent: 0, memoryPercent: 0, storagePercent: 0, bandwidthPercent: 0 },
        network: { cpuPercent: 0, memoryPercent: 0, storagePercent: 0, bandwidthPercent: 0 },
      },
      totalAllocated: { cpuPercent: 0, memoryPercent: 0, storagePercent: 0, bandwidthPercent: 0 },
      allocationBlocks: [],
    };
  }

  private getEmptyFamilyCommunityProtection(): FamilyCommunityProtectionStatus {
    return {
      redundancy: { strategy: 'full_replica', redundancyFactor: 0, recoveryThreshold: 0 },
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

  private getEmptyInfrastructureTokenBalance(): InfrastructureTokenBalance {
    return {
      balance: { tokens: 0, estimatedValue: { value: 0, currency: 'USD' } },
      earningRate: { tokensPerHour: 0, basedOn: { cpuAllocation: 0, storageAllocation: 0, bandwidthAllocation: 0 }, estimatedMonthly: 0 },
      decay: { demurrageRate: 0.5, lastCalculated: new Date().toISOString(), projectedNextMonth: { tokens: 0, valueUSD: 0 } },
      transactions: [],
      tokenHistory: { last24Hours: 0, last7Days: 0, last30Days: 0, allTime: 0 },
      exchangeRates: [],
    };
  }

  private getEmptyConstitutionalLimitsStatus(): ConstitutionalLimitsStatus {
    return {
      dignityFloor: {
        computeMinCores: this.config.dignityFloorCores,
        computeMinMemoryGB: this.config.dignityFloorMemoryGB,
        computeMinStorageGB: this.config.dignityFloorStorageGB,
        computeMinBandwidthMbps: this.config.dignityFloorBandwidthMbps,
        status: 'breached',
        percentOfFloor: 0,
        enforcement: 'progressive',
      },
      ceilingLimit: {
        computeMaxCores: this.config.ceilingLimitCores,
        computeMaxMemoryGB: this.config.ceilingLimitMemoryGB,
        computeMaxStorageGB: this.config.ceilingLimitStorageGB,
        computeMaxBandwidthMbps: this.config.ceilingLimitBandwidthMbps,
        tokenAccumulationCeiling: this.config.tokenAccumulationCeiling,
        currentAccumulation: 0,
        percentOfCeiling: 0,
        status: 'safe',
        enforcement: 'progressive',
      },
      safeZone: { cpu: 0, memory: 0, storage: 0, bandwidth: 0, tokens: 0 },
      alerts: [],
    };
  }
}
