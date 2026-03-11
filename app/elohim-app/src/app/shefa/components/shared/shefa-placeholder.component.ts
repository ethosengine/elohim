import { Component, inject } from '@angular/core';
import { ActivatedRoute } from '@angular/router';

export interface PlaceholderData {
  title: string;
  description: string;
  features: string[];
}

@Component({
  selector: 'app-shefa-placeholder',
  standalone: true,
  template: `
    <div class="placeholder-container">
      <div class="placeholder-card">
        <span class="material-icons placeholder-icon">construction</span>
        <h2 class="placeholder-title">{{ data.title }}</h2>
        <p class="placeholder-description">{{ data.description }}</p>
        @if (data.features.length > 0) {
          <div class="planned-features">
            <h3 class="features-heading">Planned Features</h3>
            <ul class="features-list">
              @for (feature of data.features; track feature) {
                <li>{{ feature }}</li>
              }
            </ul>
          </div>
        }
      </div>
    </div>
  `,
  styles: [
    `
      .placeholder-container {
        display: flex;
        justify-content: center;
        align-items: flex-start;
        padding: 3rem 1.5rem;
        min-height: 60vh;
      }

      .placeholder-card {
        max-width: 520px;
        width: 100%;
        text-align: center;
        padding: 2.5rem 2rem;
        background: var(--lamad-surface, rgba(30, 30, 46, 0.6));
        border: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.15));
        border-radius: 0.75rem;
      }

      .placeholder-icon {
        font-size: 3rem;
        color: var(--lamad-text-muted, #64748b);
        margin-bottom: 1rem;
      }

      .placeholder-title {
        font-size: 1.5rem;
        font-weight: 600;
        color: var(--lamad-text-primary, #f8fafc);
        margin: 0 0 0.75rem;
      }

      .placeholder-description {
        font-size: 0.9375rem;
        color: var(--lamad-text-secondary, #e2e8f0);
        line-height: 1.6;
        margin: 0 0 1.5rem;
      }

      .planned-features {
        text-align: left;
        border-top: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.1));
        padding-top: 1.25rem;
      }

      .features-heading {
        font-size: 0.8125rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: var(--lamad-text-muted, #64748b);
        margin: 0 0 0.75rem;
      }

      .features-list {
        list-style: none;
        padding: 0;
        margin: 0;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
      }

      .features-list li {
        font-size: 0.875rem;
        color: var(--lamad-text-secondary, #e2e8f0);
        padding-left: 1.5rem;
        position: relative;
      }

      .features-list li::before {
        content: '';
        position: absolute;
        left: 0;
        top: 0.5em;
        width: 6px;
        height: 6px;
        border-radius: 50%;
        background: var(--lamad-accent-primary, #6366f1);
        opacity: 0.6;
      }
    `,
  ],
})
export class ShefaPlaceholderComponent {
  private readonly route = inject(ActivatedRoute);

  readonly data: PlaceholderData = (this.route.snapshot.data['placeholder'] as
    | PlaceholderData
    | undefined) ?? {
    title: 'Coming Soon',
    description: 'This section is under development.',
    features: [],
  };
}
