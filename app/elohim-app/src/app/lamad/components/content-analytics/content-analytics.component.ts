import { CommonModule } from '@angular/common';
import { Component, Input, OnChanges, inject } from '@angular/core';
import { forkJoin } from 'rxjs';

import { EventService } from '@app/shefa/services/event.service';

@Component({
  selector: 'app-content-analytics',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './content-analytics.component.html',
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
