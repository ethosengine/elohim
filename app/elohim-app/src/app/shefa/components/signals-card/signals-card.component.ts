import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, inject, OnInit } from '@angular/core';

import type { PlacementGapView } from '@app/generated/placement-gap-view';
import { ResilienceService } from '@elohim/service/public-api';

@Component({
  selector: 'app-signals-card',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './signals-card.component.html',
  styleUrls: ['./signals-card.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SignalsCardComponent implements OnInit {
  private readonly resilience = inject(ResilienceService);

  gaps: PlacementGapView[] = [];
  loading = true;
  error?: string;

  ngOnInit(): void {
    this.resilience.listPlacementGaps().subscribe({
      next: ({ items }) => {
        this.gaps = items;
        this.loading = false;
      },
      error: (e: unknown) => {
        this.error = String((e as { message?: string })?.message ?? e);
        this.loading = false;
      },
    });
  }

  byKind(kind: string): PlacementGapView[] {
    return this.gaps.filter(g => g.gapKind === kind);
  }

  get totalGaps(): number {
    return this.gaps.length;
  }
}
