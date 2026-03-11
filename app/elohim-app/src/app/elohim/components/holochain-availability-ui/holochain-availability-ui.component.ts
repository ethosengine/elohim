import { CommonModule } from '@angular/common';
import { Component, inject, computed, signal } from '@angular/core';

// @coverage: 30.2% (2026-02-24)

import { HolochainClientService } from '../../services/holochain-client.service';

/**
 * Holochain Availability UI Component
 *
 * Displays unified connection status banner with graceful degradation.
 *
 * Features:
 * - Shows connection status (connected, connecting, error, offline)
 * - Provides retry button for manual reconnection
 * - Dismissible warnings
 * - Clear messaging on feature availability in degraded mode
 *
 * States:
 * - Connected: Green banner, all features available
 * - Connecting: Yellow banner, features degraded, show progress
 * - Error: Red banner, error message, retry button
 * - Offline: Gray banner, cached content only
 */
@Component({
  selector: 'app-holochain-availability-ui',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './holochain-availability-ui.component.html',
  styleUrl: './holochain-availability-ui.component.css',
})
export class HolochainAvailabilityUiComponent {
  private readonly holochainClient = inject(HolochainClientService);

  // Exposed state from services
  readonly connectionState = this.holochainClient.state;
  readonly isConnected = this.holochainClient.isConnected;
  readonly error = this.holochainClient.error;
  /** @deprecated Content availability is now determined by doorway projection, not Holochain */
  readonly contentAvailable = this.isConnected;

  // Local component state
  readonly isDismissed = signal(false);

  // Computed display states
  readonly isConnecting = computed(() => this.connectionState() === 'connecting');
  readonly isError = computed(() => this.connectionState() === 'error');
  readonly isOffline = computed(
    () => this.connectionState() === 'disconnected' || this.connectionState() === 'error'
  );

  // UI visibility
  readonly shouldShow = computed(() => {
    const state = this.connectionState();
    return (
      !this.isDismissed() &&
      (state === 'connecting' || state === 'error' || state === 'disconnected')
    );
  });

  // Status messaging
  readonly statusMessage = computed(() => {
    const state = this.connectionState();

    if (state === 'connected') {
      return 'Connected to Holochain';
    }

    if (state === 'connecting') {
      return 'Connecting to Holochain...';
    }

    if (state === 'error') {
      const errorMsg = this.error();
      return `Connection Error: ${errorMsg ?? 'Unknown error'}`;
    }

    if (state === 'disconnected') {
      return 'Offline - Using cached content';
    }

    return 'Unknown connection state';
  });

  // Degradation messaging
  readonly degradationMessage = computed(() => {
    if (this.isConnected()) {
      return '';
    }

    if (this.isConnecting()) {
      return 'Some features may be temporarily unavailable while connecting.';
    }

    return 'Working in offline mode. Some features are unavailable.';
  });

  // CSS class bindings
  readonly bannerClass = computed(() => {
    if (this.isConnected()) return 'connected';
    if (this.isConnecting()) return 'connecting';
    if (this.isError()) return 'error';
    return 'offline';
  });

  readonly bannerIcon = computed(() => {
    if (this.isConnected()) return '✓';
    if (this.isConnecting()) return '⟳';
    if (this.isError()) return '⚠';
    return '⊗';
  });

  // Note: ngOnInit/ngOnDestroy intentionally not implemented
  // Auto-dismiss would need a proper subscription in a real app - simplified here

  /**
   * Dismiss the banner
   */
  dismissBanner(): void {
    this.isDismissed.set(true);

    // Auto-show again if connection state changes
    // Note: In production, properly manage subscription lifecycle with OnDestroy
    // Using signals directly - subscription pattern needs refactoring to use effect()
    // This is a known issue to be addressed in a separate PR
  }

  /**
   * Retry connection
   */
  async retryConnection(): Promise<void> {
    this.isDismissed.set(false);
    try {
      await this.holochainClient.connect();
    } catch {
      // Connection retry failed silently - user can try again
    }
  }

  /**
   * Get degradation features list
   */
  getDegradedFeatures(): string[] {
    if (this.isConnected()) {
      return [];
    }

    return [
      'Creating new content',
      'Submitting mastery progress',
      'Recording appreciation',
      'Accessing real-time data',
    ];
  }

  /**
   * Get available features in degraded mode
   */
  getAvailableFeatures(): string[] {
    return [
      'Reading cached content',
      'Browsing learning paths',
      'Viewing cached blobs',
      'Offline caching',
    ];
  }
}
