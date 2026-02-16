/**
 * Agency Badge Component
 *
 * Compact at-a-glance view of user's agency status for the profile tray.
 * Shows current stage, connection status, and provides link to detailed view.
 */

import { CommonModule } from '@angular/common';
import { Component, inject, signal, output, computed } from '@angular/core';
import { Router } from '@angular/router';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { AgencyService } from '@app/imagodei/services/agency.service';

@Component({
  selector: 'app-agency-badge',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './agency-badge.component.html',
  styleUrl: './agency-badge.component.css',
})
export class AgencyBadgeComponent {
  private readonly agencyService = inject(AgencyService);
  private readonly holochainService = inject(HolochainClientService);
  private readonly router = inject(Router);

  /** Whether the details section is expanded */
  readonly expanded = signal(false);

  /** Emit when user wants to view full network details */
  readonly viewDetails = output<void>();

  /** Emit when user wants to upgrade */
  readonly upgrade = output<void>();

  // Computed values from agency service
  readonly state = this.agencyService.agencyState;
  readonly stageInfo = this.agencyService.stageInfo;
  readonly connectionStatus = this.agencyService.connectionStatus;
  readonly summary = computed(() => this.agencyService.getStageSummary());
  readonly canUpgrade = this.agencyService.canUpgrade;

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
    void this.router.navigate(['/identity/profile'], { fragment: 'network' });
  }

  /**
   * Start upgrade flow.
   */
  onUpgrade(): void {
    this.upgrade.emit();
    void this.router.navigate(['/identity/profile'], { fragment: 'upgrade' });
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
    } catch {
      // Clipboard write failed silently - not all browsers support this API
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
