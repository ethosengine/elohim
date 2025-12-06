import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';

/**
 * ShefaHomeComponent - Economy context app landing page
 *
 * Shefa (Hebrew: abundance, flow) is the economic layer of the Elohim Protocol.
 *
 * Future features:
 * - ValueFlows reports and visualizations
 * - REA (Resource-Event-Agent) economic coordination
 * - Multi-dimensional token tracking (care, time, learning, steward, creator)
 * - Constitutional flow control (dignity floor, attribution, circulation)
 * - Contributor presence and value attribution
 */
@Component({
  selector: 'app-shefa-home',
  standalone: true,
  imports: [CommonModule, RouterModule],
  template: `
    <div class="shefa-home">
      <div class="hero">
        <div class="icon">&#x2728;</div>
        <h1>Shefa</h1>
        <p class="subtitle">Economics of Human Flourishing</p>
      </div>

      <div class="coming-soon">
        <h2>Coming Soon</h2>
        <p>
          Shefa is the economic coordination layer of the Elohim Protocol,
          implementing ValueFlows patterns for multi-dimensional value tracking.
        </p>
      </div>

      <div class="features">
        <div class="feature-card">
          <div class="feature-icon">&#x1F4CA;</div>
          <h3>ValueFlows Reports</h3>
          <p>REA-based economic event tracking and visualization</p>
        </div>

        <div class="feature-card">
          <div class="feature-icon">&#x2764;&#xFE0F;</div>
          <h3>Multi-Dimensional Value</h3>
          <p>Track care, time, learning, stewardship, and creation tokens</p>
        </div>

        <div class="feature-card">
          <div class="feature-icon">&#x2696;&#xFE0F;</div>
          <h3>Constitutional Flows</h3>
          <p>Dignity floor, attribution persistence, circulation requirements</p>
        </div>

        <div class="feature-card">
          <div class="feature-icon">&#x1F91D;</div>
          <h3>Contributor Presence</h3>
          <p>Stewardship and value attribution for all contributors</p>
        </div>
      </div>

      <div class="navigation">
        <a routerLink="/lamad" class="nav-link">
          &#x1F4DA; Explore Lamad (Content)
        </a>
      </div>
    </div>
  `,
  styles: [`
    .shefa-home {
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
      transition: transform 0.2s, box-shadow 0.2s;
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
    }
  `]
})
export class ShefaHomeComponent {}
