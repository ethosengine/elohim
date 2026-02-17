import { Component, Input } from '@angular/core';

/**
 * Dashboard header showing learner profile, tier, XP, and streak.
 */
@Component({
  selector: 'app-dashboard-header',
  standalone: true,
  imports: [],
  template: `
    <div class="dashboard-header">
      <div class="avatar-section">
        <div class="avatar">
          {{ firstLetter }}
        </div>
      </div>
      <div class="name-tier-section">
        <h1 class="display-name">{{ displayName }}</h1>
        <div class="tier-badge">
          <span class="material-icons tier-icon">{{ tierIcon }}</span>
          <span class="tier-label">{{ tierLabel }}</span>
        </div>
      </div>
      <div class="xp-section">
        <div class="xp-count">{{ totalXp.toLocaleString() }} XP</div>
        <div class="xp-label">Total Experience</div>
      </div>
      <div class="streak-section">
        <span class="material-icons streak-icon">local_fire_department</span>
        <div class="streak-count">{{ currentStreak }}</div>
        <div class="streak-label">Day Streak</div>
      </div>
    </div>
  `,
  styles: [
    `
      .dashboard-header {
        display: flex;
        align-items: center;
        gap: 1.5rem;
        padding: 1.5rem;
        background: var(--surface-color, #fff);
        border: 1px solid var(--border-color, #e5e7eb);
        border-radius: 0.5rem;
        margin-bottom: 1.5rem;
      }

      .avatar-section {
        flex-shrink: 0;
      }

      .avatar {
        width: 4rem;
        height: 4rem;
        border-radius: 50%;
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
        display: flex;
        align-items: center;
        justify-content: center;
        color: white;
        font-size: 1.75rem;
        font-weight: 600;
        text-transform: uppercase;
      }

      .name-tier-section {
        flex-grow: 1;
        min-width: 0;
      }

      .display-name {
        font-size: 1.5rem;
        font-weight: 700;
        margin: 0 0 0.5rem 0;
        color: var(--text-primary, #111827);
      }

      .tier-badge {
        display: inline-flex;
        align-items: center;
        gap: 0.375rem;
        padding: 0.25rem 0.75rem;
        background: var(--bg-secondary, #f9fafb);
        border-radius: 1rem;
        font-size: 0.875rem;
        font-weight: 500;
        color: var(--text-secondary, #6b7280);
      }

      .tier-icon {
        font-size: 1rem;
      }

      .xp-section {
        text-align: center;
        padding: 0 1rem;
        border-left: 1px solid var(--border-color, #e5e7eb);
      }

      .xp-count {
        font-size: 1.5rem;
        font-weight: 700;
        color: var(--text-primary, #111827);
      }

      .xp-label {
        font-size: 0.75rem;
        color: var(--text-secondary, #6b7280);
        margin-top: 0.25rem;
      }

      .streak-section {
        display: flex;
        flex-direction: column;
        align-items: center;
        padding: 0 1rem;
        border-left: 1px solid var(--border-color, #e5e7eb);
      }

      .streak-icon {
        font-size: 2rem;
        color: #ff6b35;
        margin-bottom: 0.25rem;
      }

      .streak-count {
        font-size: 1.5rem;
        font-weight: 700;
        color: var(--text-primary, #111827);
      }

      .streak-label {
        font-size: 0.75rem;
        color: var(--text-secondary, #6b7280);
        margin-top: 0.25rem;
      }

      @media (max-width: 768px) {
        .dashboard-header {
          flex-wrap: wrap;
          gap: 1rem;
        }

        .xp-section,
        .streak-section {
          border-left: none;
          border-top: 1px solid var(--border-color, #e5e7eb);
          padding: 1rem 0.5rem 0;
          flex: 1;
        }
      }
    `,
  ],
})
export class DashboardHeaderComponent {
  @Input() displayName = 'Learner';
  @Input() tierLabel = 'Explorer';
  @Input() tierIcon = 'explore';
  @Input() totalXp = 0;
  @Input() currentStreak = 0;

  get firstLetter(): string {
    return this.displayName.charAt(0).toUpperCase();
  }
}
