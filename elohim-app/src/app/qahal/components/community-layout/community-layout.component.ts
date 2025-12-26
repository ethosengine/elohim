import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterOutlet, RouterLink } from '@angular/router';
import { ElohimNavigatorComponent } from '@app/elohim/components/elohim-navigator/elohim-navigator.component';

/**
 * CommunityLayoutComponent - Layout wrapper for the Community context app
 *
 * Uses the unified ElohimNavigator for consistent navigation across all contexts.
 */
@Component({
  selector: 'app-community-layout',
  standalone: true,
  imports: [CommonModule, RouterOutlet, RouterLink, ElohimNavigatorComponent],
  template: `
    <div class="community-container">
      <!-- Elohim Navigator (unified header) -->
      <app-elohim-navigator [context]="'community'" [showSearch]="false">
        <!-- Main Content -->
        <div class="community-main">
          <router-outlet></router-outlet>
        </div>
      </app-elohim-navigator>

      <!-- Footer -->
      <footer class="community-footer">
        <p>Qahal - Community & Governance | <a routerLink="/" class="footer-link">Powered by Elohim Protocol</a></p>
      </footer>
    </div>
  `,
  styles: [`
    .community-container {
      min-height: 100vh;
      display: flex;
      flex-direction: column;
      background: linear-gradient(135deg, var(--lamad-bg-primary, #0f0f1a) 0%, var(--lamad-bg-secondary, #1a1a2e) 100%);
      color: var(--lamad-text-secondary, #e2e8f0);
    }

    .community-main {
      flex: 1;
      max-width: 1400px;
      width: 100%;
      margin: 0 auto;
      display: flex;
      flex-direction: column;
    }

    .community-footer {
      background: var(--lamad-surface, rgba(30, 30, 46, 0.8));
      border-top: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.1));
      padding: 1.5rem 2rem;
      text-align: center;
      color: var(--lamad-text-muted, #64748b);
      font-size: 0.875rem;
    }

    .footer-link {
      color: var(--lamad-text-secondary, #e2e8f0);
      text-decoration: none;
      transition: color 0.2s;
    }

    .footer-link:hover {
      color: var(--lamad-accent-primary, #6366f1);
    }

    @media (max-width: 768px) {
      .community-footer {
        padding: 1rem;
        font-size: 0.75rem;
      }
    }
  `]
})
export class CommunityLayoutComponent {}
