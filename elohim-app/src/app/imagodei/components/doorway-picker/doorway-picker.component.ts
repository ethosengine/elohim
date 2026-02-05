/**
 * DoorwayPickerComponent - Gateway Selection UI
 *
 * Allows users to discover and select an Elohim doorway (network gateway).
 * Similar to selecting a Mastodon instance in the Fediverse, users choose
 * which doorway will serve as their identity provider and Holochain gateway.
 *
 * Features:
 * - Grid of doorway cards with status, region, features
 * - Search/filter by region
 * - Custom doorway URL input
 * - Health status indicators
 * - Selection persistence
 */

import { CommonModule } from '@angular/common';
import { Component, OnInit, inject, signal, computed, output, input } from '@angular/core';
import { FormsModule } from '@angular/forms';

// @coverage: 58.4% (2026-02-05)

import {
  type DoorwayInfo,
  type DoorwayRegion,
  type DoorwayWithHealth,
  getRegionDisplayName,
  getStatusDisplay,
  getFeatureDisplay,
} from '../../models/doorway.model';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { OAuthAuthProvider } from '../../services/providers/oauth-auth.provider';

/** Mode for the doorway picker */
export type DoorwayPickerMode = 'register' | 'login';

@Component({
  selector: 'app-doorway-picker',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './doorway-picker.component.html',
  styleUrls: ['./doorway-picker.component.css'],
})
export class DoorwayPickerComponent implements OnInit {
  // ===========================================================================
  // Dependencies
  // ===========================================================================

  private readonly registryService = inject(DoorwayRegistryService);
  private readonly oauthProvider = inject(OAuthAuthProvider);

  // ===========================================================================
  // Inputs
  // ===========================================================================

  /** Mode: 'register' shows "Create identity", 'login' shows "Sign in" */
  readonly mode = input<DoorwayPickerMode>('register');

  // ===========================================================================
  // Outputs
  // ===========================================================================

  /** Emitted when a doorway is selected */
  readonly doorwaySelected = output<DoorwayInfo>();

  /** Emitted when selection is cancelled */
  readonly cancelled = output<void>();

  // ===========================================================================
  // Component State
  // ===========================================================================

  readonly searchQuery = signal('');
  readonly selectedRegion = signal<DoorwayRegion | 'all'>('all');
  readonly showCustomInput = signal(false);
  readonly customUrl = signal('');
  readonly customValidating = signal(false);
  readonly customError = signal<string | null>(null);

  /** Sort option for doorway list */
  readonly sortBy = signal<'recommended' | 'latency' | 'name' | 'users'>('recommended');

  // ===========================================================================
  // Delegated State
  // ===========================================================================

  readonly doorways = this.registryService.doorwaysWithHealth;
  readonly isLoading = this.registryService.isLoading;
  readonly error = this.registryService.error;
  readonly selected = this.registryService.selected;

  // ===========================================================================
  // Computed State
  // ===========================================================================

  /** The recommended doorway (online, best latency, open registration) */
  readonly recommendedDoorway = computed(() => {
    const online = this.doorways().filter(
      d => d.status === 'online' && d.registrationOpen && d.latencyMs !== null
    );
    if (online.length === 0) return null;
    // Sort by latency and return the fastest
    const sorted = [...online].sort(
      (a, b) => (a.latencyMs ?? Infinity) - (b.latencyMs ?? Infinity)
    );
    return sorted[0];
  });

  /** Filtered and sorted doorways based on search, region, and sort option */
  readonly filteredDoorways = computed(() => {
    let result = this.doorways();

    // Filter by region
    const region = this.selectedRegion();
    if (region !== 'all') {
      result = result.filter(d => d.region === region);
    }

    // Filter by search query
    const query = this.searchQuery().toLowerCase().trim();
    if (query) {
      result = result.filter(
        d =>
          d.name.toLowerCase().includes(query) ||
          d.description.toLowerCase().includes(query) ||
          d.operator.toLowerCase().includes(query)
      );
    }

    // Sort by selected option
    const sortOption = this.sortBy();
    const recommended = this.recommendedDoorway();

    result = [...result].sort((a, b) => {
      // Always put recommended first if sorting by recommended
      if (sortOption === 'recommended' && recommended) {
        if (a.id === recommended.id) return -1;
        if (b.id === recommended.id) return 1;
      }

      switch (sortOption) {
        case 'latency':
        case 'recommended': {
          // Lower latency is better; null goes to end
          const latA = a.latencyMs ?? Infinity;
          const latB = b.latencyMs ?? Infinity;
          return latA - latB;
        }
        case 'name':
          return a.name.localeCompare(b.name);
        case 'users':
          // Higher user count is better
          return (b.userCount ?? 0) - (a.userCount ?? 0);
        default:
          return 0;
      }
    });

    return result;
  });

  /** Available regions from doorways */
  readonly availableRegions = computed(() => {
    const regions = new Set(this.doorways().map(d => d.region));
    return Array.from(regions).sort((a, b) => a.localeCompare(b));
  });

  /** Currently selected doorway ID (for highlighting) */
  readonly selectedId = computed(() => this.selected()?.doorway.id ?? null);

  /** Title text based on mode */
  readonly titleText = computed(() => 'Choose Your Gateway');

  /** Subtitle text based on mode */
  readonly subtitleText = computed(() =>
    this.mode() === 'login'
      ? 'Select the doorway where you registered'
      : 'Select a doorway to create your identity'
  );

  /** Action button text based on mode */
  readonly actionText = computed(() =>
    this.mode() === 'login' ? 'Sign In Here' : 'Join This Doorway'
  );

  // ===========================================================================
  // Template Helpers
  // ===========================================================================

  readonly getRegionDisplayName = getRegionDisplayName;
  readonly getStatusDisplay = getStatusDisplay;
  readonly getFeatureDisplay = getFeatureDisplay;

  // ===========================================================================
  // Lifecycle
  // ===========================================================================

  ngOnInit(): void {
    void this.loadDoorways();
  }

  // ===========================================================================
  // Public Methods
  // ===========================================================================

  async loadDoorways(): Promise<void> {
    await this.registryService.loadDoorways();
    // Refresh health after loading
    await this.registryService.refreshHealth();
  }

  selectDoorway(doorway: DoorwayInfo): void {
    // Persist selection
    this.registryService.selectDoorway(doorway, true);

    // In login mode, initiate OAuth redirect to doorway's /auth/authorize
    if (this.mode() === 'login') {
      // Use different return URL for Tauri (custom URL scheme) vs browser (web origin)
      const returnUrl = this.isTauriEnvironment()
        ? 'elohim://auth/callback'
        : `${globalThis.location.origin}/auth/callback`;

      this.oauthProvider.initiateLogin(doorway.url, returnUrl);
      // Note: This redirects the browser (or opens external browser in Tauri)
    } else {
      // In register mode, just emit the selection for the parent to handle
      this.doorwaySelected.emit(doorway);
    }
  }

  /**
   * Detect if running in Tauri native app.
   */
  private isTauriEnvironment(): boolean {
    return typeof window !== 'undefined' && '__TAURI__' in window;
  }

  toggleCustomInput(): void {
    this.showCustomInput.update(v => !v);
    this.customError.set(null);
    this.customUrl.set('');
  }

  async validateAndSelectCustom(): Promise<void> {
    const url = this.customUrl().trim();
    if (!url) {
      this.customError.set('Please enter a doorway URL');
      return;
    }

    this.customValidating.set(true);
    this.customError.set(null);

    try {
      const result = await this.registryService.validateDoorway(url);

      if (result.isValid && result.doorway) {
        this.registryService.selectDoorway(result.doorway, true);
        this.doorwaySelected.emit(result.doorway);
      } else {
        this.customError.set(result.error ?? 'Invalid doorway');
      }
    } catch (err) {
      this.customError.set(err instanceof Error ? err.message : 'Validation failed');
    } finally {
      this.customValidating.set(false);
    }
  }

  cancel(): void {
    this.cancelled.emit();
  }

  // ===========================================================================
  // Template Helpers
  // ===========================================================================

  getLatencyClass(latencyMs: number | null): string {
    if (latencyMs === null) return 'latency-unknown';
    if (latencyMs < 100) return 'latency-fast';
    if (latencyMs < 300) return 'latency-medium';
    return 'latency-slow';
  }

  formatLatency(latencyMs: number | null): string {
    if (latencyMs === null) return '--';
    return `${latencyMs}ms`;
  }

  /** Calculate latency bar width (0-100%) for visual indicator */
  getLatencyBarWidth(latencyMs: number | null): number {
    if (latencyMs === null) return 0;
    // Cap at 500ms for visual purposes (anything above is "slow")
    const capped = Math.min(latencyMs, 500);
    // Invert: lower latency = fuller bar (better)
    return Math.round(((500 - capped) / 500) * 100);
  }

  /** Check if doorway is the recommended one */
  isRecommended(doorway: DoorwayWithHealth): boolean {
    const rec = this.recommendedDoorway();
    return rec !== null && rec.id === doorway.id;
  }

  /** Auto-select the recommended doorway */
  selectRecommended(): void {
    const rec = this.recommendedDoorway();
    if (rec) {
      this.selectDoorway(rec);
    }
  }

  trackByDoorway(_index: number, doorway: DoorwayWithHealth): string {
    return doorway.id;
  }
}
