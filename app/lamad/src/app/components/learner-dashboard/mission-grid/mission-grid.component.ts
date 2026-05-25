import { Component, Input } from '@angular/core';

import { MissionCardComponent } from './mission-card.component';

import type { DashboardPathEntry } from '../../../models/learner-mastery-profile.model';

/**
 * Grid of active learning missions.
 */
@Component({
  selector: 'app-mission-grid',
  standalone: true,
  imports: [MissionCardComponent],
  template: `
    <div class="mission-grid">
      @for (path of paths; track path.pathId) {
        <app-mission-card [path]="path" />
      } @empty {
        <div class="empty-state">
          <p>No active missions yet. Start a learning path to begin your journey!</p>
        </div>
      }
    </div>
  `,
  styles: [
    `
      .mission-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
        gap: 1.5rem;
        margin-bottom: 2rem;
      }

      .empty-state {
        grid-column: 1 / -1;
        padding: 3rem 1.5rem;
        text-align: center;
        background: var(--bg-secondary, #f9fafb);
        border-radius: 0.5rem;
        color: var(--text-secondary, #6b7280);
      }

      .empty-state p {
        margin: 0;
        font-size: 1rem;
      }
    `,
  ],
})
export class MissionGridComponent {
  @Input() paths: DashboardPathEntry[] = [];
}
