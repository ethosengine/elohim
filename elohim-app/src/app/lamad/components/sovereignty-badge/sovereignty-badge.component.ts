/**
 * Sovereignty Badge Component
 *
 * Compact at-a-glance view of user's sovereignty status for the profile tray.
 * Shows current stage, connection status, and provides link to detailed view.
 */

import { CommonModule } from '@angular/common';
import { Component, inject, signal, output, computed } from '@angular/core';
import { Router } from '@angular/router';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { SovereigntyService } from '@app/imagodei/services/sovereignty.service';

@Component({
  selector: 'app-sovereignty-badge',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './sovereignty-badge.component.html',
  styleUrl: './sovereignty-badge.component.css',
})
export class SovereigntyBadgeComponent {
  private readonly sovereigntyService = inject(SovereigntyService);
  private readonly holochainService = inject(HolochainClientService);
  private readonly router = inject(Router);

  /** Whether the details section is expanded */
  readonly expanded = signal(false);

  /** Emit when user wants to view full network details */
  readonly viewDetails = output<void>();

  /** Emit when user wants to upgrade */
  readonly upgrade = output<void>();

  // Computed values from sovereignty service
  readonly state = this.sovereigntyService.sovereigntyState;
  readonly stageInfo = this.sovereigntyService.stageInfo;
  readonly connectionStatus = this.sovereigntyService.connectionStatus;
  readonly summary = computed(() => this.sovereigntyService.getStageSummary());
  readonly canUpgrade = this.sovereigntyService.canUpgrade;

  /** Connection state for Edge Node details */
  readonly edgeNodeInfo = computed(() => this.holochainService.getDisplayInfo());

  /**
   * Get CSS class for stage badge color.
   */
  getStageBadgeClass(): string {
    const stage = this.state().currentStage;
    return `stage-badge--${stage}`;
  }

  /**
   * Get CSS class for connection status dot.
   */
  getStatusDotClass(): string {
    const status = this.connectionStatus().state;
    return `status-dot--${status}`;
  }

  /**
   * Toggle expanded state.
   */
  toggleExpand(): void {
    this.expanded.update(v => !v);
  }

  /**
   * Navigate to full network details in profile.
   */
  onViewDetails(): void {
    this.viewDetails.emit();
    this.router.navigate(['/lamad/human'], { fragment: 'network' });
  }

  /**
   * Start upgrade flow.
   */
  onUpgrade(): void {
    this.upgrade.emit();
    // TODO: Open upgrade modal or navigate to upgrade flow
  }

  /**
   * Reconnect to network.
   */
  async onReconnect(): Promise<void> {
    await this.holochainService.disconnect();
    await this.holochainService.connect();
  }

  /**
   * Copy value to clipboard.
   */
  async copyToClipboard(value: string, event: Event): Promise<void> {
    event.stopPropagation();
    try {
      await navigator.clipboard.writeText(value);
    } catch (err) {
      console.warn('Failed to copy to clipboard:', err);
    }
  }

  /**
   * Truncate hash for display.
   */
  truncateHash(hash: string | null): string {
    if (!hash) return '';
    if (hash.length <= 16) return hash;
    return `${hash.substring(0, 8)}...${hash.substring(hash.length - 4)}`;
  }
}
