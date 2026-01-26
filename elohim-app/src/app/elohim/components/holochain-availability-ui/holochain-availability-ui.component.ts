import { CommonModule } from '@angular/common';
import { Component, inject, computed, signal } from '@angular/core';

import { HolochainClientService } from '../../services/holochain-client.service';
import { HolochainContentService } from '../../services/holochain-content.service';
import { OfflineOperationQueueService } from '../../services/offline-operation-queue.service';

/**
 * Holochain Availability UI Component
 *
 * Displays unified connection status banner with graceful degradation.
 *
 * Features:
 * - Shows connection status (connected, connecting, error, offline)
 * - Displays queue size for offline operations
 * - Provides retry button for manual reconnection
 * - Dismissible warnings
 * - Clear messaging on feature availability in degraded mode
 *
 * States:
 * - Connected: Green banner, all features available
 * - Connecting: Yellow banner, features degraded, show progress
 * - Error: Red banner, error message, retry button
 * - Offline: Gray banner, cached content only, queue visible
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
  private readonly holochainContent = inject(HolochainContentService);
  private readonly operationQueue = inject(OfflineOperationQueueService);

  // Exposed state from services
  readonly connectionState = this.holochainClient.state;
  readonly isConnected = this.holochainClient.isConnected;
  readonly error = this.holochainClient.error;
  readonly contentAvailable = this.holochainContent.availableSignal;

  // Local component state
  readonly isDismissed = signal(false);
  readonly queuedOperations = computed(() => this.operationQueue.getQueueSize());
  readonly hasQueuedOperations = computed(() => this.queuedOperations() > 0);

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
      (state === 'connecting' ||
        state === 'error' ||
        state === 'disconnected' ||
        this.hasQueuedOperations())
    );
  });

  // Status messaging
  readonly statusMessage = computed(() => {
    const state = this.connectionState();
    const queueSize = this.queuedOperations();

    if (state === 'connected') {
      return 'Connected to Holochain';
    }

    if (state === 'connecting') {
      return 'Connecting to Holochain...';
    }

    if (state === 'error') {
      const errorMsg = this.error();
      return `Connection Error: ${errorMsg || 'Unknown error'}`;
    }

    if (state === 'disconnected') {
      if (queueSize > 0) {
        return `Offline - ${queueSize} operations queued`;
      }
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

    return (
      'Working in offline mode. Some features are unavailable. ' +
      'Write operations will be queued and synced when connection is restored.'
    );
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
    const subscription = this.connectionState.subscribe(() => {
      const currentState = this.connectionState();
      if (currentState === 'error' || currentState === 'connecting') {
        this.isDismissed.set(false);
      }
    });

    // Note: In production, properly manage subscription lifecycle
  }

  /**
   * Retry connection
   */
  async retryConnection(): Promise<void> {
    this.isDismissed.set(false);
    try {
      await this.holochainClient.connect();
    } catch (err) {
      console.error('Retry failed:', err);
    }
  }

  /**
   * Sync queued operations
   */
  async syncQueuedOperations(): Promise<void> {
    try {
      await this.operationQueue.syncAll();
      this.isDismissed.set(false); // Show success message
    } catch (err) {
      console.error('Sync failed:', err);
    }
  }

  /**
   * Get degradation features list
   */
  getDegradedFeatures(): string[] {
    if (this.isConnected()) {
      return [];
    }

    const features = [
      'Creating new content',
      'Submitting mastery progress',
      'Recording appreciation',
      'Accessing real-time data',
    ];

    if (this.hasQueuedOperations()) {
      features.push('Syncing queued operations');
    }

    return features;
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
