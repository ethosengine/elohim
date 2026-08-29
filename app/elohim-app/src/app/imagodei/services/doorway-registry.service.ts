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

// @coverage: 72.2% (2026-02-24)

import { catchError, of, timeout, firstValueFrom } from 'rxjs';

import { FederationRegistryAnchor } from '@app/elohim/integrity';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

import { isWorkspaceRuntime, workspaceDoorwayUrl } from '@workspace/runtime';

import { environment } from '../../../environments/environment';
import {
  type DoorwayInfo,
  type DoorwayStatus,
  type DoorwayWithHealth,
  type DoorwaySelection,
  type DoorwayHealthResponse,
  DOORWAY_URL_KEY,
  DOORWAY_CACHE_KEY,
  BOOTSTRAP_DOORWAYS,
  probeDoorway,
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

/**
 * This development workspace's OWN doorway, as a selectable identity provider.
 *
 * The workspace developer registers and logs in against it, and its chaperone
 * provisions their agent — so it is a real doorway in the registry, not a
 * special case. The vendor's hostname convention is resolved by the fence
 * (`app/workspace-runtime/`), never here.
 */
function createWorkspaceDoorway(): DoorwayInfo {
  return {
    id: 'workspace-local-doorway',
    name: 'Local Dev (workspace)',
    url: workspaceDoorwayUrl() ?? '',
    description: "This development workspace's own doorway",
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
  private readonly federationAnchor = inject(FederationRegistryAnchor);

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

  /**
   * Selected doorway URL — null until the selection carries PROOF.
   *
   * This is the accessor auth uses, so it is deliberately the strict one: a
   * selection whose `verified` flag is false reads as "no doorway selected"
   * and callers fall back to their configured origin. Use
   * `selectedUrlUnverified` for display/diagnostics, never as a request base.
   */
  readonly selectedUrl = computed(() => {
    const selection = this.selectedSignal();
    return selection?.verified ? selection.doorway.url : null;
  });

  /** Selected doorway URL regardless of proof — display/diagnostics only. */
  readonly selectedUrlUnverified = computed(() => this.selectedSignal()?.doorway.url ?? null);

  /** Whether doorways are loading */
  readonly isLoading = this.loadingSignal.asReadonly();

  /** Current error */
  readonly error = this.errorSignal.asReadonly();

  /** Doorways with health info attached (always includes selected doorway) */
  readonly doorwaysWithHealth = computed(() => {
    const healthMap = this.healthMapSignal();
    const doorways = this.doorways();
    const selected = this.selectedSignal()?.doorway;

    // Ensure the selected doorway is always in the list
    const hasSelected = selected && doorways.some(d => d.id === selected.id);
    const allDoorways = hasSelected || !selected ? doorways : [selected, ...doorways];

    return allDoorways.map(
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
          return this.mergeWithFederationPeers(dhtDoorways);
        }
      }

      // Try fetching from known doorway
      const fallbackDoorways = await this.fetchFromDoorway();
      if (fallbackDoorways.length > 0) {
        this.doorwaysSignal.set(fallbackDoorways);
        this.cacheResult(fallbackDoorways);
        return this.mergeWithFederationPeers(fallbackDoorways);
      }

      // Fall back to cached or bootstrap
      const cached = this.getCached();
      if (cached) {
        this.doorwaysSignal.set(cached);
        return this.mergeWithFederationPeers(cached);
      }

      // Last resort: bootstrap list
      this.doorwaysSignal.set(BOOTSTRAP_DOORWAYS);
      return this.mergeWithFederationPeers(BOOTSTRAP_DOORWAYS);
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
      const results = await Promise.all(batch.map(async d => this.checkHealth(d)));
      results.forEach(r => healthMap.set(r.id, r));
    }

    this.healthMapSignal.set(healthMap);
  }

  // ===========================================================================
  // Public Methods - Selection
  // ===========================================================================

  /**
   * Select a doorway the caller already holds — a registry entry, a bootstrap
   * entry, or one `validateDoorway()` just answered for. Such a doorway came
   * from somewhere trusted, so the selection is verified.
   */
  selectDoorway(doorway: DoorwayInfo, isExplicit = true): void {
    this.applySelection(doorway, isExplicit, true);
  }

  /**
   * Select a doorway by URL — REFUSING any URL we cannot vouch for.
   *
   * A URL reaches here from a resolved federated identifier, from
   * `environment.client.doorwayUrl`, or from a profile action, so it may carry
   * whatever a human typed. We select it only when its host is one we already
   * trust (a known/bootstrap doorway, a configured origin, this workspace's
   * doorway). An unrecognised host is a NO-OP: the current selection stands
   * and nothing is written to localStorage — because the next thing that
   * happens to a selected doorway is a plaintext password POSTed at it.
   *
   * To adopt a genuinely new doorway, prove it first with
   * {@link selectProbedDoorwayUrl}.
   */
  selectDoorwayByUrl(url: string): void {
    if (this.selectedUrl() === url) return;

    const known = this.findTrustedDoorway(url);
    if (!known) return;

    this.applySelection(known, false, true);
  }

  /**
   * Adopt a doorway the app has never seen, by asking that host to prove it is
   * one (`GET /.well-known/elohim-auth`). The probed origin is the host's OWN
   * origin — never a derivative of it — so what gets selected is exactly what
   * answered.
   *
   * @returns true when the host proved itself and is now selected.
   */
  async selectProbedDoorwayUrl(url: string, isExplicit = false): Promise<boolean> {
    const trusted = this.findTrustedDoorway(url);
    if (trusted) {
      this.applySelection(trusted, isExplicit, true);
      return true;
    }

    const origin = await probeDoorway(url);
    if (!origin) return false;

    this.applySelection(this.minimalDoorway(origin), isExplicit, true);
    return true;
  }

  /**
   * Clear current selection.
   */
  clearSelection(): void {
    this.selectedSignal.set(null);
    // eslint-disable-next-line no-restricted-syntax -- SSR-safe: browser-only tauri-auth logout surface, never SSR-rendered
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
    // Delegate to FederationRegistryAnchor — the zome call is already
    // a fallback behind HTTP; now its integrity role is named.
    return this.federationAnchor.verify('global');
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
    // eslint-disable-next-line no-restricted-syntax -- SSR-safe: only invoked from loadDoorways(), triggered by explicit profile-component action, never SSR bootstrap-reachable
    localStorage.setItem(DOORWAY_CACHE_KEY, JSON.stringify(cache));
  }

  /**
   * Get cached doorways if not expired.
   */
  private getCached(): DoorwayInfo[] | null {
    try {
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: only invoked from loadDoorways(), triggered by explicit profile-component action, never SSR bootstrap-reachable
      const raw = localStorage.getItem(DOORWAY_CACHE_KEY);
      if (!raw) return null;

      const cache = JSON.parse(raw) as { expiresAt: string; doorways: DoorwayInfo[] };
      const expiresAt = new Date(cache.expiresAt);

      if (expiresAt > new Date()) {
        return cache.doorways;
      }

      // Expired, remove cache
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: only invoked from loadDoorways(), triggered by explicit profile-component action, never SSR bootstrap-reachable
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
    // eslint-disable-next-line no-restricted-syntax -- SSR-safe: only invoked from selectDoorway()/selectDoorwayByUrl(), triggered by explicit user action in register/profile/login components, never SSR bootstrap-reachable
    localStorage.setItem(DOORWAY_URL_KEY, JSON.stringify(selection));
  }

  /**
   * Restore selection from localStorage, or auto-select this workspace's own
   * doorway in dev.
   */
  private restoreSelection(): void {
    // SSR-safe: no browser storage during server-side rendering
    if (typeof localStorage === 'undefined') return;

    // In a development workspace, always use this workspace's own doorway
    if (isWorkspaceRuntime()) {
      this.selectedSignal.set({
        doorway: createWorkspaceDoorway(),
        selectedAt: new Date().toISOString(),
        isExplicit: false,
        // Resolved by the vendor fence from build-time config, not from input.
        verified: true,
      });
      return;
    }

    try {
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: guarded by typeof localStorage check at top of restoreSelection()
      const raw = localStorage.getItem(DOORWAY_URL_KEY);
      if (!raw) return;

      const selection = JSON.parse(raw) as DoorwaySelection;

      // localStorage is attacker-writable and predates this rule, so a stored
      // `verified: true` proves nothing. Recompute it: a trusted host is
      // verified now, anything else is restored UNVERIFIED (so it can never be
      // an auth base) and re-proved in the background, or discarded.
      const trusted = this.findTrustedDoorway(selection.doorway.url) !== null;
      this.selectedSignal.set({ ...selection, verified: trusted });

      if (!trusted) {
        void this.reverifyRestoredSelection(selection.doorway.url);
      }
    } catch {
      // Invalid stored data, clear it
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: guarded by typeof localStorage check at top of restoreSelection()
      localStorage.removeItem(DOORWAY_URL_KEY);
    }
  }

  // ===========================================================================
  // Private Methods - Federation
  // ===========================================================================

  /**
   * Merge existing doorways with federation peers from the selected doorway.
   * Deduplicates by ID, tagging federation-sourced entries.
   */
  private async mergeWithFederationPeers(doorways: DoorwayInfo[]): Promise<DoorwayInfo[]> {
    const selectedUrl = this.selectedUrl();
    if (!selectedUrl) return doorways;

    const peers = await this.fetchFederationPeers(selectedUrl);
    if (peers.length === 0) return doorways;

    // Deduplicate by ID — existing entries take priority
    const existingIds = new Set(doorways.map(d => d.id));
    const newPeers = peers.filter(p => !existingIds.has(p.id));

    if (newPeers.length > 0) {
      const merged = [...doorways, ...newPeers];
      this.doorwaysSignal.set(merged);
      return merged;
    }

    return doorways;
  }

  /**
   * Fetch federation peers from the selected doorway's federation endpoint.
   */
  private async fetchFederationPeers(baseUrl: string): Promise<DoorwayInfo[]> {
    try {
      const resp = await fetch(`${baseUrl}/api/v1/federation/doorways`, {
        signal: AbortSignal.timeout(5000),
      });
      if (!resp.ok) return [];
      const data = (await resp.json()) as {
        doorways?: { id: string; url: string; region?: string }[];
      };
      return (data.doorways ?? []).map(d => ({
        id: d.id,
        name: d.id,
        url: d.url,
        description: 'Discovered via federation',
        region: (d.region ?? 'global') as DoorwayInfo['region'],
        operator: 'Federation',
        features: [] as DoorwayInfo['features'],
        status: 'online' as DoorwayInfo['status'],
        registrationOpen: false,
      }));
    } catch {
      return [];
    }
  }

  // ===========================================================================
  // Private Methods - Trust
  // ===========================================================================

  /**
   * Origins this app trusts WITHOUT a probe, because they were not typed by a
   * human: build-time environment config and this workspace's own doorway.
   */
  private configuredOrigins(): string[] {
    return [
      environment.client?.doorwayUrl,
      environment.holochain?.authUrl,
      workspaceDoorwayUrl(),
    ].filter((url): url is string => !!url);
  }

  /**
   * Find a doorway we can vouch for at `url`'s host — a loaded registry entry,
   * a bootstrap entry, or a configured origin. Host EQUALITY, never substring:
   * `alpha.elohim.host.evil.tld` contains `alpha.elohim.host` and is not it.
   *
   * @returns the doorway to select, or null when the host is unvouched.
   */
  private findTrustedDoorway(url: string): DoorwayInfo | null {
    const host = this.hostOf(url);
    if (!host) return null;

    const known = [...this.doorwaysSignal(), ...BOOTSTRAP_DOORWAYS].find(
      d => this.hostOf(d.url) === host
    );
    if (known) return known;

    const configured = this.configuredOrigins().find(origin => this.hostOf(origin) === host);
    return configured ? this.minimalDoorway(configured) : null;
  }

  /** Lowercased host (with port) of a URL, or null when it does not parse. */
  private hostOf(url: string): string | null {
    try {
      return new URL(this.normalizeUrl(url)).host.toLowerCase();
    } catch {
      return null;
    }
  }

  /** A placeholder registry entry for a doorway known only by its URL. */
  private minimalDoorway(url: string): DoorwayInfo {
    return {
      id: url.replace(/https?:\/\//, '').replace(/[^a-z0-9]/g, '-'),
      name: new URL(url).hostname,
      url,
      description: '',
      region: 'global',
      operator: '',
      features: [],
      status: 'unknown',
      registrationOpen: true,
    };
  }

  /** Set + persist a selection, recording whether it carries proof. */
  private applySelection(doorway: DoorwayInfo, isExplicit: boolean, verified: boolean): void {
    const selection: DoorwaySelection = {
      doorway,
      selectedAt: new Date().toISOString(),
      isExplicit,
      verified,
    };

    this.selectedSignal.set(selection);
    this.persistSelection(selection);
  }

  /**
   * Re-prove a selection restored from localStorage. The host either answers
   * its own auth-discovery document — in which case the restored selection is
   * promoted to verified — or the selection is DISCARDED, storage included, so
   * a value poisoned before this rule existed cannot outlive one boot.
   */
  private async reverifyRestoredSelection(url: string): Promise<void> {
    const origin = await probeDoorway(url);
    const current = this.selectedSignal();
    if (!current || current.doorway.url !== url) return;

    if (origin === null) {
      this.clearSelection();
      return;
    }

    this.applySelection(current.doorway, current.isExplicit, true);
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
