import { CommonModule } from '@angular/common';
import { Component } from '@angular/core';
import { RouterModule } from '@angular/router';

// @coverage: 100.0% (2026-01-31)

/**
 * LearnerDashboardComponent - Personal learning dashboard.
 *
 * Route: /lamad/me
 *
 * Displays:
 * - Active paths with progress
 * - Completed paths
 * - Attestations earned
 * - Learning frontier (what's next)
 *
 * Implements Section 1.3 of LAMAD_API_SPECIFICATION_v1.0.md
 * Full implementation planned for Phase 5.
 */
@Component({
  selector: 'app-learner-dashboard',
  standalone: true,
  imports: [CommonModule, RouterModule],
  template: `
    <div class="learner-dashboard">
      <header class="dashboard-header">
        <h1>My Learning</h1>
        <p class="subtitle">Track your progress and discover what's next</p>
      </header>

      <section class="placeholder-section">
        <div class="placeholder-card">
          <h2>Active Paths</h2>
          <p>Your in-progress learning paths will appear here.</p>
          <a routerLink="/lamad" class="btn btn-primary">Explore Paths</a>
        </div>

        <div class="placeholder-card">
          <h2>Completed</h2>
          <p>Paths you've finished will be listed here.</p>
        </div>

        <div class="placeholder-card">
          <h2>Attestations</h2>
          <p>Achievements and credentials you've earned.</p>
        </div>
      </section>

      <p class="coming-soon">Full dashboard coming soon...</p>
    </div>
  `,
  styles: [
    `
      .learner-dashboard {
        max-width: 900px;
        margin: 0 auto;
        padding: 2rem;
      }

      .dashboard-header {
        padding-bottom: 1.5rem;
        border-bottom: 1px solid var(--border-color, #e5e7eb);
        margin-bottom: 2rem;
      }

      .dashboard-header h1 {
        font-size: 2rem;
        font-weight: 800;
        color: var(--text-primary, #111827);
        margin: 0 0 0.5rem 0;
      }

      .subtitle {
        color: var(--text-secondary, #6b7280);
      }

      .placeholder-section {
        display: grid;
        gap: 1.5rem;
        margin-bottom: 2rem;
      }

      .placeholder-card {
        background: var(--bg-secondary, #f9fafb);
        border: 1px solid var(--border-color, #e5e7eb);
        border-radius: 8px;
        padding: 1.5rem;
      }

      .placeholder-card h2 {
        font-size: 1.125rem;
        font-weight: 600;
        margin-bottom: 0.5rem;
      }

      .placeholder-card p {
        color: var(--text-secondary, #6b7280);
        margin-bottom: 1rem;
      }

      .btn {
        display: inline-block;
        padding: 0.5rem 1rem;
        border-radius: 6px;
        text-decoration: none;
        font-weight: 500;
      }

      .btn-primary {
        background: var(--primary-color, #3b82f6);
        color: white;
      }

      .coming-soon {
        text-align: center;
        color: var(--text-tertiary, #9ca3af);
        font-style: italic;
      }
    `,
  ],
})
export class LearnerDashboardComponent {}
