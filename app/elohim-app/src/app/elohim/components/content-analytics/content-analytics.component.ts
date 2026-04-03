import { CommonModule } from '@angular/common';
import { Component, Input, OnChanges, inject } from '@angular/core';
import { forkJoin } from 'rxjs';

import { EventService } from '@app/shefa/services/event.service';

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
        Metrics are protocol-native economic events, not external analytics.
        Views are recorded after 3 seconds of engagement.
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

  private readonly eventService = inject(EventService);

  ngOnChanges(): void {
    this.loadAnalytics();
  }

  private loadAnalytics(): void {
    this.isLoading = true;

    forkJoin({
      views: this.eventService.getViewCount(this.contentId),
      completions: this.eventService.getCompletionCount(this.contentId),
    }).subscribe({
      next: ({ views, completions }) => {
        this.viewCount = views;
        this.completionCount = completions;
        this.completionRate = views > 0 ? Math.round((completions / views) * 100) : 0;
        this.isLoading = false;
      },
      error: () => {
        this.isLoading = false;
      },
    });
  }
}
