import { CommonModule } from '@angular/common';
import { Component } from '@angular/core';
import { RouterModule } from '@angular/router';

/**
 * CommunityHomeComponent - Community context app landing page
 *
 * The community layer of the Elohim Protocol.
 *
 * Future features:
 * - Human-to-human relationship management (graduated intimacy)
 * - Consent-based connections
 * - Governance deliberation (Loomio-style proposals)
 * - Place-based community coordination
 * - Affinity circles and community visualization
 */
@Component({
  selector: 'app-community-home',
  standalone: true,
  imports: [CommonModule, RouterModule],
  template: `
    <div class="community-home">
      <div class="hero">
        <div class="icon">👥</div>
        <h1>Community</h1>
        <p class="subtitle">Community & Governance</p>
      </div>

      <div class="coming-soon">
        <h2>Coming Soon</h2>
        <p>
          The community coordination layer of the Elohim Protocol, implementing consent-based
          relationships and collective governance.
        </p>
      </div>

      <div class="features">
        <div class="feature-card">
          <div class="feature-icon">🤝</div>
          <h3>Graduated Intimacy</h3>
          <p>Consent-based relationships from recognition to intimate trust</p>
        </div>

        <div class="feature-card">
          <div class="feature-icon">⚖️</div>
          <h3>Governance</h3>
          <p>Loomio-style deliberation with Polis-inspired sensemaking</p>
        </div>

        <div class="feature-card">
          <div class="feature-icon">🌍</div>
          <h3>Place-Based Community</h3>
          <p>Bioregional coordination with constitutional ecological limits</p>
        </div>

        <div class="feature-card">
          <div class="feature-icon">💬</div>
          <h3>Affinity Circles</h3>
          <p>Visual representation of your community connections</p>
        </div>
      </div>

      <div class="navigation">
        <a routerLink="/lamad" class="nav-link">📚 Explore Lamad (Content)</a>
        <a routerLink="/shefa" class="nav-link secondary">✨ View Shefa (Economy)</a>
      </div>
    </div>
  `,
  styles: [
    `
      .community-home {
        max-width: 800px;
        margin: 0 auto;
        padding: 2rem;
        text-align: center;
      }

      .hero {
        margin-bottom: 3rem;
      }

      .icon {
        font-size: 4rem;
        margin-bottom: 1rem;
      }

      h1 {
        font-size: 3rem;
        margin: 0;
        color: var(--text-primary, #1a1a1a);
      }

      .subtitle {
        font-size: 1.25rem;
        color: var(--text-secondary, #666);
        margin-top: 0.5rem;
      }

      .coming-soon {
        background: var(--surface-elevated, #f5f5f5);
        border-radius: 12px;
        padding: 2rem;
        margin-bottom: 3rem;
      }

      .coming-soon h2 {
        color: var(--primary, #6366f1);
        margin-top: 0;
      }

      .coming-soon p {
        color: var(--text-secondary, #666);
        max-width: 500px;
        margin: 0 auto;
        line-height: 1.6;
      }

      .features {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
        gap: 1.5rem;
        margin-bottom: 3rem;
      }

      .feature-card {
        background: var(--surface, #fff);
        border: 1px solid var(--border, #e5e5e5);
        border-radius: 12px;
        padding: 1.5rem;
        transition:
          transform 0.2s,
          box-shadow 0.2s;
      }

      .feature-card:hover {
        transform: translateY(-2px);
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
      }

      .feature-icon {
        font-size: 2rem;
        margin-bottom: 0.75rem;
      }

      .feature-card h3 {
        font-size: 1rem;
        margin: 0 0 0.5rem 0;
        color: var(--text-primary, #1a1a1a);
      }

      .feature-card p {
        font-size: 0.875rem;
        color: var(--text-secondary, #666);
        margin: 0;
        line-height: 1.5;
      }

      .navigation {
        margin-top: 2rem;
        display: flex;
        gap: 1rem;
        justify-content: center;
        flex-wrap: wrap;
      }

      .nav-link {
        display: inline-block;
        padding: 0.75rem 1.5rem;
        background: var(--primary, #6366f1);
        color: white;
        text-decoration: none;
        border-radius: 8px;
        font-weight: 500;
        transition: background 0.2s;
      }

      .nav-link:hover {
        background: var(--primary-dark, #4f46e5);
      }

      .nav-link.secondary {
        background: var(--surface-elevated, #f5f5f5);
        color: var(--text-primary, #1a1a1a);
        border: 1px solid var(--border, #e5e5e5);
      }

      .nav-link.secondary:hover {
        background: var(--surface, #e5e5e5);
      }

      @media (prefers-color-scheme: dark) {
        h1 {
          color: var(--text-primary, #f5f5f5);
        }

        .coming-soon {
          background: var(--surface-elevated, #2a2a2a);
        }

        .feature-card {
          background: var(--surface, #1a1a1a);
          border-color: var(--border, #333);
        }

        .feature-card h3 {
          color: var(--text-primary, #f5f5f5);
        }

        .nav-link.secondary {
          background: var(--surface-elevated, #2a2a2a);
          color: var(--text-primary, #f5f5f5);
          border-color: var(--border, #444);
        }
      }
    `,
  ],
})
export class CommunityHomeComponent {}
