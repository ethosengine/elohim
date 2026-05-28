import { CommonModule } from '@angular/common';
import { Component, Input, OnChanges, inject } from '@angular/core';

import { StorageClientService } from '@app/elohim/services/storage-client.service';

@Component({
  selector: 'app-content-analytics',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="content-analytics" data-testid="content-analytics">
      <h3 class="analytics-title">Attention Metrics</h3>

      <div *ngIf="isLoading" class="loading">Loading metrics...</div>

      <div *ngIf="!isLoading" class="metrics-grid">
        <div class="metric" data-testid="analytics-views">
          <span class="metric-value">{{ viewCount }}</span>
          <span class="metric-label">Views</span>
        </div>
        <div class="metric" data-testid="analytics-completions">
          <span class="metric-value">{{ completionCount }}</span>
          <span class="metric-label">Completions</span>
        </div>
        <div class="metric" data-testid="analytics-completion-rate">
          <span class="metric-value">{{ completionRate }}%</span>
          <span class="metric-label">Completion Rate</span>
        </div>
      </div>

      <p class="analytics-note">
        Metrics are protocol-native economic events, not external analytics. Views are recorded
        after 3 seconds of engagement.
      </p>
    </div>
  `,
  styleUrls: ['./content-analytics.component.css'],
})
export class ContentAnalyticsComponent implements OnChanges {
  @Input({ required: true }) contentId!: string;

  viewCount = 0;
  completionCount = 0;
  completionRate = 0;
  isLoading = true;

  private readonly storageClient = inject(StorageClientService);

  ngOnChanges(): void {
    this.loadAnalytics();
  }

  private loadAnalytics(): void {
    this.isLoading = true;

    // M-AGGR-3: reads server-side ContentEngagementStatsView projection at
    // GET /api/v1/lamad/content/{contentId}/engagement.
    // completionRate is pre-computed by the projection writer (0.0 when views == 0).
    this.storageClient.getContentEngagement(this.contentId).subscribe({
      next: (stats) => {
        this.viewCount = Number(stats.views);
        this.completionCount = Number(stats.completions);
        this.completionRate = Math.round(stats.completionRate * 100);
        this.isLoading = false;
      },
      error: () => {
        this.isLoading = false;
      },
    });
  }
}
