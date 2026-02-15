import { Component, Input } from '@angular/core';

/**
 * Row of stat cards showing mastery and freshness metrics.
 */
@Component({
  selector: 'app-stats-row',
  standalone: true,
  imports: [],
  template: `
    <div class="stats-row">
      <div class="stat-card">
        <div class="stat-icon mastered-icon">
          <span class="material-icons">stars</span>
        </div>
        <div class="stat-content">
          <div class="stat-value">{{ masteredNodes }}</div>
          <div class="stat-label">Mastered Nodes</div>
        </div>
      </div>

      <div class="stat-card">
        <div class="stat-icon gate-icon">
          <span class="material-icons">verified</span>
        </div>
        <div class="stat-content">
          <div class="stat-value">{{ aboveGate }}</div>
          <div class="stat-label">Above Gate</div>
        </div>
      </div>

      <div class="stat-card">
        <div class="stat-icon" [style.background-color]="freshnessColor">
          <span class="material-icons">{{ freshnessIcon }}</span>
        </div>
        <div class="stat-content">
          <div class="stat-value">{{ freshnessHealthPercent }}%</div>
          <div class="stat-label">Freshness Health</div>
          <div class="stat-breakdown">
            <span class="fresh-count">{{ freshCount }} fresh</span>
            <span class="stale-count">{{ staleCount }} stale</span>
            <span class="critical-count">{{ criticalCount }} critical</span>
          </div>
        </div>
      </div>

      <div class="stat-card">
        <div class="stat-icon xp-icon">
          <span class="material-icons">school</span>
        </div>
        <div class="stat-content">
          <div class="stat-value">{{ totalXp.toLocaleString() }}</div>
          <div class="stat-label">Learning XP</div>
          <div class="stat-subtitle">+{{ xpEarnedThisWeek }} this week</div>
        </div>
      </div>
    </div>
  `,
  styles: [
    `
      .stats-row {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
        gap: 1rem;
        margin-bottom: 2rem;
      }

      .stat-card {
        display: flex;
        align-items: flex-start;
        gap: 1rem;
        padding: 1.25rem;
        background: var(--surface-color, #fff);
        border: 1px solid var(--border-color, #e5e7eb);
        border-radius: 0.5rem;
      }

      .stat-icon {
        flex-shrink: 0;
        width: 3rem;
        height: 3rem;
        border-radius: 0.5rem;
        display: flex;
        align-items: center;
        justify-content: center;
        color: white;
      }

      .stat-icon .material-icons {
        font-size: 1.5rem;
      }

      .mastered-icon {
        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
      }

      .gate-icon {
        background: #4caf50;
      }

      .xp-icon {
        background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
      }

      .stat-content {
        flex: 1;
        min-width: 0;
      }

      .stat-value {
        font-size: 1.75rem;
        font-weight: 700;
        color: var(--text-primary, #111827);
        line-height: 1.2;
      }

      .stat-label {
        font-size: 0.875rem;
        font-weight: 500;
        color: var(--text-secondary, #6b7280);
        margin-top: 0.25rem;
      }

      .stat-subtitle {
        font-size: 0.75rem;
        color: var(--text-secondary, #6b7280);
        margin-top: 0.25rem;
      }

      .stat-breakdown {
        display: flex;
        gap: 0.75rem;
        margin-top: 0.5rem;
        font-size: 0.75rem;
      }

      .fresh-count {
        color: #4caf50;
      }

      .stale-count {
        color: #ff9800;
      }

      .critical-count {
        color: #f44336;
      }

      @media (max-width: 768px) {
        .stats-row {
          grid-template-columns: 1fr;
        }
      }
    `,
  ],
})
export class StatsRowComponent {
  @Input() masteredNodes = 0;
  @Input() aboveGate = 0;
  @Input() freshnessHealthPercent = 100;
  @Input() freshCount = 0;
  @Input() staleCount = 0;
  @Input() criticalCount = 0;
  @Input() totalXp = 0;
  @Input() xpEarnedThisWeek = 0;

  get freshnessColor(): string {
    if (this.freshnessHealthPercent >= 70) return '#4caf50';
    if (this.freshnessHealthPercent >= 40) return '#ff9800';
    return '#f44336';
  }

  get freshnessIcon(): string {
    if (this.freshnessHealthPercent >= 70) return 'check_circle';
    if (this.freshnessHealthPercent >= 40) return 'warning';
    return 'error';
  }
}
