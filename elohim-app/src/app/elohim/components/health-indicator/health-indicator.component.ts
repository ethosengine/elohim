/**
 * Health Indicator Component
 *
 * Displays system health status with expandable details.
 * Uses the HealthCheckService to show real-time system status.
 */

import { CommonModule } from '@angular/common';
import { Component, inject, computed } from '@angular/core';

import { HealthCheckService, HealthState } from '../../services/health-check.service';

@Component({
  selector: 'app-health-indicator',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './health-indicator.component.html',
  styleUrls: ['./health-indicator.component.css'],
})
export class HealthIndicatorComponent {
  private readonly healthService = inject(HealthCheckService);

  /** Current health status */
  readonly status = this.healthService.status;
  readonly isChecking = this.healthService.isChecking;

  /** Whether details panel is expanded */
  isExpanded = false;

  /** Quick status for the indicator dot */
  readonly quickStatus = computed(() => this.healthService.getQuickStatus());

  /** Individual checks as array for iteration */
  readonly checksArray = computed(() => {
    const checks = this.status().checks;
    return [
      { key: 'holochain', label: 'Holochain', ...checks.holochain },
      { key: 'indexedDb', label: 'Cache', ...checks.indexedDb },
      { key: 'blobCache', label: 'Blob Cache', ...checks.blobCache },
      { key: 'network', label: 'Network', ...checks.network },
    ];
  });

  /**
   * Toggle expanded state.
   */
  toggleExpanded(): void {
    this.isExpanded = !this.isExpanded;
  }

  /**
   * Trigger manual refresh.
   */
  async refresh(): Promise<void> {
    await this.healthService.refresh();
  }

  /**
   * Get status icon for a check.
   */
  getStatusIcon(status: HealthState): string {
    switch (status) {
      case 'healthy':
        return '✓';
      case 'degraded':
        return '!';
      case 'unhealthy':
        return '✕';
      default:
        return '?';
    }
  }

  /**
   * Get CSS class for status.
   */
  getStatusClass(status: HealthState): string {
    return `status-${status}`;
  }

  /**
   * Format timestamp for display.
   */
  formatTime(isoString: string): string {
    try {
      const date = new Date(isoString);
      return date.toLocaleTimeString();
    } catch {
      return 'Unknown';
    }
  }
}
