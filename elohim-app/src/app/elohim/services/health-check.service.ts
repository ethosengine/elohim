/**
 * Health Check Service - System health monitoring for Elohim app.
 *
 * Provides unified health status across:
 * - Holochain connection
 * - IndexedDB cache
 * - Blob cache
 * - Network connectivity
 *
 * Usage:
 * ```typescript
 * // Get current health status
 * const status = this.healthCheck.status();
 *
 * // Force refresh all checks
 * await this.healthCheck.refresh();
 *
 * // Subscribe to health changes
 * effect(() => {
 *   const health = this.healthCheck.status();
 *   if (health.status === 'unhealthy') {
 *     this.showAlert();
 *   }
 * });
 * ```
 */

import { Injectable, signal, computed, inject, OnDestroy, afterNextRender } from '@angular/core';

// @coverage: 80.4% (2026-02-05)

import { HolochainClientService } from './holochain-client.service';
import { IndexedDBCacheService } from './indexeddb-cache.service';
import { LoggerService } from './logger.service';

// =============================================================================
// Types
// =============================================================================

/** Overall health status */
export type HealthState = 'healthy' | 'degraded' | 'unhealthy' | 'unknown';

/** Individual check result */
export interface HealthCheck {
  /** Check name */
  name: string;
  /** Current status */
  status: HealthState;
  /** Human-readable message */
  message: string;
  /** Last check timestamp */
  lastChecked: string;
  /** Duration of check in ms */
  durationMs?: number;
  /** Additional metadata */
  metadata?: Record<string, unknown>;
}

/** Complete health status */
export interface HealthStatus {
  /** Overall status (worst of all checks) */
  status: HealthState;
  /** Human-readable summary */
  summary: string;
  /** Last full check time */
  lastChecked: string;
  /** Individual check results */
  checks: {
    holochain: HealthCheck;
    indexedDb: HealthCheck;
    blobCache: HealthCheck;
    network: HealthCheck;
  };
  /** True if any check is in progress */
  isChecking: boolean;
}

// =============================================================================
// Configuration
// =============================================================================

/** Interval between automatic health checks (ms) */
const AUTO_CHECK_INTERVAL = 30_000; // 30 seconds

/** Timeout for individual checks (ms) */
const _CHECK_TIMEOUT = 5_000;

// =============================================================================
// Service Implementation
// =============================================================================

const UNKNOWN_ERROR = UNKNOWN_ERROR;

@Injectable({ providedIn: 'root' })
export class HealthCheckService implements OnDestroy {
  private readonly holochainClient = inject(HolochainClientService);
  private readonly indexedDbCache = inject(IndexedDBCacheService);
  private readonly logger = inject(LoggerService).createChild('HealthCheck');

  /** Current health status signal */
  private readonly _status = signal<HealthStatus>(this.createInitialStatus());

  /** Whether a check is in progress */
  private readonly _isChecking = signal(false);

  /** Auto-check interval ID */
  private autoCheckInterval: ReturnType<typeof setInterval> | null = null;

  // ===========================================================================
  // Public API
  // ===========================================================================

  /** Current health status (read-only signal) */
  readonly status = computed(() => this._status());

  /** Whether system is healthy */
  readonly isHealthy = computed(() => this._status().status === 'healthy');

  /** Whether system is degraded */
  readonly isDegraded = computed(() => this._status().status === 'degraded');

  /** Whether system is unhealthy */
  readonly isUnhealthy = computed(() => this._status().status === 'unhealthy');

  /** Whether a check is in progress */
  readonly isChecking = computed(() => this._isChecking());

  constructor() {
    // Defer health monitoring until after first render to avoid async in constructor
    afterNextRender(() => {
      void this.refresh();
      this.autoCheckInterval = setInterval(() => {
        void this.refresh();
      }, AUTO_CHECK_INTERVAL);
    });
  }

  ngOnDestroy(): void {
    if (this.autoCheckInterval) {
      clearInterval(this.autoCheckInterval);
      this.autoCheckInterval = null;
    }
  }

  /**
   * Force refresh all health checks.
   */
  async refresh(): Promise<HealthStatus> {
    if (this._isChecking()) {
      return this._status();
    }

    this._isChecking.set(true);
    const timer = this.logger.startTimer('health-check');

    try {
      const [holochain, indexedDb, blobCache, network] = await Promise.all([
        this.checkHolochain(),
        this.checkIndexedDb(),
        this.checkBlobCache(),
        this.checkNetwork(),
      ]);

      const checks = { holochain, indexedDb, blobCache, network };
      const overallStatus = this.calculateOverallStatus(checks);
      const summary = this.createSummary(checks);

      const status: HealthStatus = {
        status: overallStatus,
        summary,
        lastChecked: new Date().toISOString(),
        checks,
        isChecking: false,
      };

      this._status.set(status);
      timer.end({ status: overallStatus });

      return status;
    } catch (_error) {
      this.logger.error('Health check failed', _error);
      throw _error;
    } finally {
      this._isChecking.set(false);
    }
  }

  /**
   * Check only Holochain connection.
   */
  async checkHolochainOnly(): Promise<HealthCheck> {
    const result = await this.checkHolochain();
    this._status.update(s => ({
      ...s,
      checks: { ...s.checks, holochain: result },
      status: this.calculateOverallStatus({ ...s.checks, holochain: result }),
      lastChecked: new Date().toISOString(),
    }));
    return result;
  }

  /**
   * Get quick status for display.
   */
  getQuickStatus(): {
    icon: string;
    label: string;
    color: string;
  } {
    const status = this._status();

    switch (status.status) {
      case 'healthy':
        return { icon: '●', label: 'All systems operational', color: 'green' };
      case 'degraded':
        return { icon: '◐', label: 'Some services degraded', color: 'yellow' };
      case 'unhealthy':
        return { icon: '○', label: 'System issues detected', color: 'red' };
      default:
        return { icon: '?', label: 'Checking...', color: 'gray' };
    }
  }

  // ===========================================================================
  // Individual Health Checks
  // ===========================================================================

  private async checkHolochain(): Promise<HealthCheck> {
    const start = performance.now();
    const name = 'holochain';

    try {
      const isConnected = this.holochainClient.isConnected();
      const displayInfo = this.holochainClient.getDisplayInfo();

      // Use Promise.resolve to satisfy require-await rule
      await Promise.resolve();

      if (isConnected) {
        return {
          name,
          status: 'healthy',
          message: `Connected to ${displayInfo.mode} mode`,
          lastChecked: new Date().toISOString(),
          durationMs: Math.round(performance.now() - start),
          metadata: {
            mode: displayInfo.mode,
            appUrl: displayInfo.appUrl,
          },
        };
      } else {
        return {
          name,
          status: 'unhealthy',
          message: 'Not connected to Holochain',
          lastChecked: new Date().toISOString(),
          durationMs: Math.round(performance.now() - start),
        };
      }
    } catch (error) {
      return {
        name,
        status: 'unhealthy',
        message: `Check failed: ${error instanceof Error ? error.message : UNKNOWN_ERROR}`,
        lastChecked: new Date().toISOString(),
        durationMs: Math.round(performance.now() - start),
      };
    }
  }

  private async checkIndexedDb(): Promise<HealthCheck> {
    const start = performance.now();
    const name = 'indexedDb';

    try {
      // Initialize if not already done
      await this.indexedDbCache.init();

      if (!this.indexedDbCache.isAvailable()) {
        return {
          name,
          status: 'degraded',
          message: 'IndexedDB not available',
          lastChecked: new Date().toISOString(),
          durationMs: Math.round(performance.now() - start),
        };
      }

      const stats = await this.indexedDbCache.getStats();

      return {
        name,
        status: 'healthy',
        message: `Cache active: ${stats.contentCount} content, ${stats.pathCount} paths`,
        lastChecked: new Date().toISOString(),
        durationMs: Math.round(performance.now() - start),
        metadata: {
          contentCount: stats.contentCount,
          pathCount: stats.pathCount,
        },
      };
    } catch (error) {
      return {
        name,
        status: 'degraded',
        message: `Check failed: ${error instanceof Error ? error.message : UNKNOWN_ERROR}`,
        lastChecked: new Date().toISOString(),
        durationMs: Math.round(performance.now() - start),
      };
    }
  }

  private async checkBlobCache(): Promise<HealthCheck> {
    const start = performance.now();
    const name = 'blobCache';

    try {
      // Check if Cache API is available (for service worker caching)
      if ('caches' in globalThis) {
        const cacheNames = await caches.keys();
        const blobCaches = cacheNames.filter(n => n.includes('blob'));

        return {
          name,
          status: 'healthy',
          message: `${blobCaches.length} blob cache(s) available`,
          lastChecked: new Date().toISOString(),
          durationMs: Math.round(performance.now() - start),
          metadata: {
            cacheCount: blobCaches.length,
            cacheNames: blobCaches,
          },
        };
      }

      // Fall back to memory cache check
      return {
        name,
        status: 'degraded',
        message: 'Cache API not available, using memory only',
        lastChecked: new Date().toISOString(),
        durationMs: Math.round(performance.now() - start),
      };
    } catch (error) {
      return {
        name,
        status: 'degraded',
        message: `Check failed: ${error instanceof Error ? error.message : UNKNOWN_ERROR}`,
        lastChecked: new Date().toISOString(),
        durationMs: Math.round(performance.now() - start),
      };
    }
  }

  private async checkNetwork(): Promise<HealthCheck> {
    const start = performance.now();
    const name = 'network';

    try {
      // Use Navigator.onLine as quick check
      const isOnline = navigator.onLine;

      // Use Promise.resolve to satisfy require-await rule
      await Promise.resolve();

      if (!isOnline) {
        return {
          name,
          status: 'unhealthy',
          message: 'Browser reports offline',
          lastChecked: new Date().toISOString(),
          durationMs: Math.round(performance.now() - start),
        };
      }

      // Check network connection info if available (Network Information API)
      const navWithConnection = navigator as Navigator & {
        connection?: { effectiveType?: string; downlink?: number };
      };
      const connection = navWithConnection.connection;
      if (connection) {
        const effectiveType = connection.effectiveType;
        const downlink = connection.downlink;

        // Slow connection is degraded
        if (effectiveType === 'slow-2g' || effectiveType === '2g') {
          return {
            name,
            status: 'degraded',
            message: `Slow connection: ${effectiveType}`,
            lastChecked: new Date().toISOString(),
            durationMs: Math.round(performance.now() - start),
            metadata: {
              effectiveType,
              downlink,
            },
          };
        }

        return {
          name,
          status: 'healthy',
          message: `Online: ${effectiveType}, ${downlink} Mbps`,
          lastChecked: new Date().toISOString(),
          durationMs: Math.round(performance.now() - start),
          metadata: {
            effectiveType,
            downlink,
          },
        };
      }

      return {
        name,
        status: 'healthy',
        message: 'Online',
        lastChecked: new Date().toISOString(),
        durationMs: Math.round(performance.now() - start),
      };
    } catch (error) {
      return {
        name,
        status: 'unknown',
        message: `Check failed: ${error instanceof Error ? error.message : UNKNOWN_ERROR}`,
        lastChecked: new Date().toISOString(),
        durationMs: Math.round(performance.now() - start),
      };
    }
  }

  // ===========================================================================
  // Helper Methods
  // ===========================================================================

  private calculateOverallStatus(checks: HealthStatus['checks']): HealthState {
    const statuses = Object.values(checks).map(c => c.status);

    // Any unhealthy → overall unhealthy
    if (statuses.some(s => s === 'unhealthy')) {
      return 'unhealthy';
    }

    // Any degraded → overall degraded
    if (statuses.some(s => s === 'degraded')) {
      return 'degraded';
    }

    // Any unknown → overall degraded
    if (statuses.some(s => s === 'unknown')) {
      return 'degraded';
    }

    return 'healthy';
  }

  private createSummary(checks: HealthStatus['checks']): string {
    const unhealthy = Object.entries(checks)
      .filter(([_, c]) => c.status === 'unhealthy')
      .map(([name]) => name);

    const degraded = Object.entries(checks)
      .filter(([_, c]) => c.status === 'degraded')
      .map(([name]) => name);

    if (unhealthy.length > 0) {
      return `Issues with: ${unhealthy.join(', ')}`;
    }

    if (degraded.length > 0) {
      return `Degraded: ${degraded.join(', ')}`;
    }

    return 'All systems operational';
  }

  private createInitialStatus(): HealthStatus {
    const now = new Date().toISOString();
    const initialCheck: HealthCheck = {
      name: '',
      status: 'unknown',
      message: 'Not yet checked',
      lastChecked: now,
    };

    return {
      status: 'unknown',
      summary: 'Checking system health...',
      lastChecked: now,
      checks: {
        holochain: { ...initialCheck, name: 'holochain' },
        indexedDb: { ...initialCheck, name: 'indexedDb' },
        blobCache: { ...initialCheck, name: 'blobCache' },
        network: { ...initialCheck, name: 'network' },
      },
      isChecking: true,
    };
  }
}
