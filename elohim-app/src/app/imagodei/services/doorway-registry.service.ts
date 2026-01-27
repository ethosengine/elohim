/**
 * Doorway Registry Service - Gateway Discovery & Selection
 *
 * Discovers and manages doorways (Elohim network gateways). Users select
 * a doorway at registration which serves as their identity provider and
 * Holochain gateway.
 *
 * Discovery hierarchy:
 * 1. DHT registry (on-chain, decentralized) - primary source
 * 2. Doorway fallback (fetch from any known doorway) - when DHT unavailable
 * 3. Bootstrap list (hardcoded) - last resort for first-time users
 *
 * Usage:
 * 1. Call loadDoorways() to fetch available doorways
 * 2. User selects a doorway from the list
 * 3. Call selectDoorway() to persist selection
 * 4. Selected doorway is used for all auth operations
 */

import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { Injectable, signal, computed, inject } from '@angular/core';

import { catchError, of, timeout, firstValueFrom } from 'rxjs';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

import {
  type DoorwayInfo,
  type DoorwayStatus,
  type DoorwayWithHealth,
  type DoorwaySelection,
  type DoorwayHealthResponse,
  DOORWAY_URL_KEY,
  DOORWAY_CACHE_KEY,
  BOOTSTRAP_DOORWAYS,
  sortDoorwaysByRelevance,
} from '../models/doorway.model';

// =============================================================================
// Constants
// =============================================================================

/** Health check timeout in milliseconds */
const HEALTH_CHECK_TIMEOUT_MS = 5000;

/** Cache TTL in milliseconds (1 hour) */
const CACHE_TTL_MS = 60 * 60 * 1000;

/** Maximum concurrent health checks */
const MAX_CONCURRENT_HEALTH_CHECKS = 5;

// =============================================================================
// Che Environment Detection
// =============================================================================

/**
 * Detect if running in Eclipse Che environment
 */
function isEclipseChe(): boolean {
  if (typeof window === 'undefined') return false;
  const hostname = window.location.hostname;
  return hostname.includes('.code.ethosengine.com') || hostname.includes('.devspaces.');
}

/**
 * Get the Che hc-dev endpoint URL for doorway access
 */
function getCheHcDevUrl(): string {
  if (typeof window === 'undefined') return '';
  const hostname = window.location.hostname.replace(/-angular-dev\./, '-hc-dev.');
  return `https://${hostname}`;
}

/**
 * Create a doorway info for the Che local environment
 */
function createCheDoorway(): DoorwayInfo {
  return {
    id: 'che-local-hc-dev',
    name: 'Local Dev (Che)',
    url: getCheHcDevUrl(),
    description: 'Local development doorway via Eclipse Che hc-dev endpoint',
    region: 'global', // Use 'global' as catch-all for dev
    operator: 'Local Development',
    features: [], // No special features for local dev
    status: 'online',
    registrationOpen: true,
    vouchCount: 0,
  };
}

// =============================================================================
// Service
// =============================================================================

@Injectable({ providedIn: 'root' })
export class DoorwayRegistryService {
  // ===========================================================================
  // Dependencies
  // ===========================================================================

  private readonly http = inject(HttpClient);
  private readonly holochainClient = inject(HolochainClientService);

  // ===========================================================================
  // State
  // ===========================================================================

  /** All known doorways */
  private readonly doorwaysSignal = signal<DoorwayInfo[]>([]);

  /** Currently selected doorway */
  private readonly selectedSignal = signal<DoorwaySelection | null>(null);

  /** Loading state */
  private readonly loadingSignal = signal(false);

  /** Error state */
  private readonly errorSignal = signal<string | null>(null);

  /** Health check results */
  private readonly healthMapSignal = signal<Map<string, DoorwayWithHealth>>(new Map());

  // ===========================================================================
  // Public Signals (read-only)
  // ===========================================================================

  /** All known doorways, sorted by relevance */
  readonly doorways = computed(() => sortDoorwaysByRelevance(this.doorwaysSignal()));

  /** Currently selected doorway */
  readonly selected = this.selectedSignal.asReadonly();

  /** Selected doorway URL (for auth operations) */
  readonly selectedUrl = computed(() => this.selectedSignal()?.doorway.url ?? null);

  /** Whether doorways are loading */
  readonly isLoading = this.loadingSignal.asReadonly();

  /** Current error */
  readonly error = this.errorSignal.asReadonly();

  /** Doorways with health info attached */
  readonly doorwaysWithHealth = computed(() => {
    const healthMap = this.healthMapSignal();
    return this.doorways().map(
      d =>
        healthMap.get(d.id) ?? {
          ...d,
          latencyMs: null,
          lastHealthCheck: new Date().toISOString(),
          isReachable: false,
        }
    );
  });

  /** Whether a doorway has been selected */
  readonly hasSelection = computed(() => this.selectedSignal() !== null);

  // ===========================================================================
  // Constructor
  // ===========================================================================

  constructor() {
    // Restore selection from localStorage on init
    this.restoreSelection();
  }

  // ===========================================================================
  // Public Methods - Discovery
  // ===========================================================================

  /**
   * Load all available doorways.
   * Tries DHT first, then doorway fallback, then bootstrap list.
   */
  async loadDoorways(): Promise<DoorwayInfo[]> {
    this.loadingSignal.set(true);
    this.errorSignal.set(null);

    try {
      // Try DHT first (if Holochain connected)
      if (this.holochainClient.isConnected()) {
        const dhtDoorways = await this.fetchFromDHT();
        if (dhtDoorways.length > 0) {
          this.doorwaysSignal.set(dhtDoorways);
          this.cacheResult(dhtDoorways);
          return dhtDoorways;
        }
      }

      // Try fetching from known doorway
      const fallbackDoorways = await this.fetchFromDoorway();
      if (fallbackDoorways.length > 0) {
        this.doorwaysSignal.set(fallbackDoorways);
        this.cacheResult(fallbackDoorways);
        return fallbackDoorways;
      }

      // Fall back to cached or bootstrap
      const cached = this.getCached();
      if (cached) {
        this.doorwaysSignal.set(cached);
        return cached;
      }

      // Last resort: bootstrap list
      this.doorwaysSignal.set(BOOTSTRAP_DOORWAYS);
      return BOOTSTRAP_DOORWAYS;
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to load doorways';
      this.errorSignal.set(message);

      // Return cached or bootstrap on error
      const cached = this.getCached();
      const fallback = cached ?? BOOTSTRAP_DOORWAYS;
      this.doorwaysSignal.set(fallback);
      return fallback;
    } finally {
      this.loadingSignal.set(false);
    }
  }

  /**
   * Refresh doorway statuses with health checks.
   */
  async refreshHealth(): Promise<void> {
    const doorways = this.doorwaysSignal();
    if (doorways.length === 0) return;

    const healthMap = new Map<string, DoorwayWithHealth>();

    // Check health in batches to avoid overwhelming
    for (let i = 0; i < doorways.length; i += MAX_CONCURRENT_HEALTH_CHECKS) {
      const batch = doorways.slice(i, i + MAX_CONCURRENT_HEALTH_CHECKS);
      const results = await Promise.all(batch.map(d => this.checkHealth(d)));
      results.forEach(r => healthMap.set(r.id, r));
    }

    this.healthMapSignal.set(healthMap);
  }

  // ===========================================================================
  // Public Methods - Selection
  // ===========================================================================

  /**
   * Select a doorway for use.
   */
  selectDoorway(doorway: DoorwayInfo, isExplicit = true): void {
    const selection: DoorwaySelection = {
      doorway,
      selectedAt: new Date().toISOString(),
      isExplicit,
    };

    this.selectedSignal.set(selection);
    this.persistSelection(selection);
  }

  /**
   * Clear current selection.
   */
  clearSelection(): void {
    this.selectedSignal.set(null);
    localStorage.removeItem(DOORWAY_URL_KEY);
  }

  /**
   * Get doorway by ID.
   */
  getDoorwayById(id: string): DoorwayInfo | undefined {
    return this.doorwaysSignal().find(d => d.id === id);
  }

  /**
   * Get doorway by URL.
   */
  getDoorwayByUrl(url: string): DoorwayInfo | undefined {
    const normalized = this.normalizeUrl(url);
    return this.doorwaysSignal().find(d => this.normalizeUrl(d.url) === normalized);
  }

  // ===========================================================================
  // Public Methods - Validation
  // ===========================================================================

  /**
   * Validate a custom doorway URL.
   */
  async validateDoorway(url: string): Promise<{
    isValid: boolean;
    doorway?: DoorwayInfo;
    error?: string;
  }> {
    try {
      const healthUrl = `${this.normalizeUrl(url)}/health`;

      const response = await firstValueFrom(
        this.http.get<DoorwayHealthResponse>(healthUrl).pipe(
          timeout(HEALTH_CHECK_TIMEOUT_MS),
          catchError((err: HttpErrorResponse) => {
            throw new Error(err.message || 'Failed to reach doorway');
          })
        )
      );

      // Build doorway info from health response
      const doorway: DoorwayInfo = {
        id: `custom-${Date.now()}`,
        name: 'Custom Doorway',
        url: this.normalizeUrl(url),
        description: 'User-provided custom doorway',
        region: 'global',
        operator: 'Unknown',
        features: [],
        status: response.status,
        userCount: response.userCount,
        registrationOpen: response.registrationOpen,
      };

      return { isValid: true, doorway };
    } catch (err) {
      return {
        isValid: false,
        error: err instanceof Error ? err.message : 'Invalid doorway URL',
      };
    }
  }

  // ===========================================================================
  // Private Methods - Data Fetching
  // ===========================================================================

  /**
   * Fetch doorways from DHT via Holochain.
   *
   * Note: Per LINK_ARCHITECTURE.md, "get all doorways" is a query candidate
   * that should prefer projection queries. This method uses infrastructure DNA
   * as a fallback when projection isn't available.
   */
  private async fetchFromDHT(): Promise<DoorwayInfo[]> {
    try {
      // Infrastructure DNA handles doorway federation
      // TODO: Add get_all_doorways or use projection query instead
      const result = await this.holochainClient.callZome<DoorwayInfo[]>({
        zomeName: 'infrastructure',
        fnName: 'get_doorways_by_region',
        payload: 'global', // Use 'global' region to get all doorways
        roleName: 'infrastructure',
      });

      if (result.success && result.data) {
        return result.data;
      }
      return [];
    } catch {
      // Falls back to fetchFromDoorway() REST API
      return [];
    }
  }

  /**
   * Fetch doorways from a known doorway's registry endpoint.
   */
  private async fetchFromDoorway(): Promise<DoorwayInfo[]> {
    // Try selected doorway first, then bootstrap doorways
    const tryUrls = [
      this.selectedSignal()?.doorway.url,
      ...BOOTSTRAP_DOORWAYS.map(d => d.url),
    ].filter((url): url is string => !!url);

    for (const baseUrl of tryUrls) {
      try {
        const registryUrl = `${baseUrl}/registry/doorways`;
        const result = await firstValueFrom(
          this.http.get<DoorwayInfo[]>(registryUrl).pipe(
            timeout(HEALTH_CHECK_TIMEOUT_MS),
            catchError(() => of(null))
          )
        );
        if (result && result.length > 0) {
          return result;
        }
      } catch {
        continue;
      }
    }

    return [];
  }

  /**
   * Check health of a single doorway.
   */
  private async checkHealth(doorway: DoorwayInfo): Promise<DoorwayWithHealth> {
    const start = performance.now();

    try {
      const healthUrl = `${doorway.url}/health`;
      const response = await firstValueFrom(
        this.http.get<DoorwayHealthResponse>(healthUrl).pipe(
          timeout(HEALTH_CHECK_TIMEOUT_MS),
          catchError(() => of(null))
        )
      );

      const latencyMs = Math.round(performance.now() - start);

      if (response) {
        return {
          ...doorway,
          status: response.status,
          registrationOpen: response.registrationOpen,
          latencyMs,
          lastHealthCheck: new Date().toISOString(),
          isReachable: true,
        };
      }
    } catch {
      // Fall through to offline status
    }

    return {
      ...doorway,
      status: 'offline' as DoorwayStatus,
      latencyMs: null,
      lastHealthCheck: new Date().toISOString(),
      isReachable: false,
    };
  }

  // ===========================================================================
  // Private Methods - Caching
  // ===========================================================================

  /**
   * Cache doorway list to localStorage.
   */
  private cacheResult(doorways: DoorwayInfo[]): void {
    const cache = {
      doorways,
      fetchedAt: new Date().toISOString(),
      expiresAt: new Date(Date.now() + CACHE_TTL_MS).toISOString(),
    };
    localStorage.setItem(DOORWAY_CACHE_KEY, JSON.stringify(cache));
  }

  /**
   * Get cached doorways if not expired.
   */
  private getCached(): DoorwayInfo[] | null {
    try {
      const raw = localStorage.getItem(DOORWAY_CACHE_KEY);
      if (!raw) return null;

      const cache = JSON.parse(raw);
      const expiresAt = new Date(cache.expiresAt);

      if (expiresAt > new Date()) {
        return cache.doorways;
      }

      // Expired, remove cache
      localStorage.removeItem(DOORWAY_CACHE_KEY);
      return null;
    } catch {
      return null;
    }
  }

  // ===========================================================================
  // Private Methods - Selection Persistence
  // ===========================================================================

  /**
   * Persist selection to localStorage.
   */
  private persistSelection(selection: DoorwaySelection): void {
    localStorage.setItem(DOORWAY_URL_KEY, JSON.stringify(selection));
  }

  /**
   * Restore selection from localStorage, or auto-select Che doorway in dev.
   */
  private restoreSelection(): void {
    // In Eclipse Che, always use the local hc-dev endpoint
    if (isEclipseChe()) {
      const cheDoorway = createCheDoorway();
      console.log(
        '[DoorwayRegistry] Eclipse Che detected, using local hc-dev endpoint:',
        cheDoorway.url
      );
      this.selectedSignal.set({
        doorway: cheDoorway,
        selectedAt: new Date().toISOString(),
        isExplicit: false,
      });
      return;
    }

    try {
      const raw = localStorage.getItem(DOORWAY_URL_KEY);
      if (!raw) return;

      const selection = JSON.parse(raw) as DoorwaySelection;
      this.selectedSignal.set(selection);
    } catch {
      // Invalid stored data, clear it
      localStorage.removeItem(DOORWAY_URL_KEY);
    }
  }

  // ===========================================================================
  // Private Helpers
  // ===========================================================================

  /**
   * Normalize URL (remove trailing slash, ensure https).
   */
  private normalizeUrl(url: string): string {
    let normalized = url.trim().toLowerCase();

    // Add protocol if missing
    if (!normalized.startsWith('http://') && !normalized.startsWith('https://')) {
      normalized = `https://${normalized}`;
    }

    // Remove trailing slash
    if (normalized.endsWith('/')) {
      normalized = normalized.slice(0, -1);
    }

    return normalized;
  }
}
