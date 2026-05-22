import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export interface DiscoverySuggestion {
  id: string;
  kind: 'adjacent-qahal' | 'federation-candidate' | 'contributor-presence';
  label: string;
  relevance: number;
}

const KIND_LABEL: Record<DiscoverySuggestion['kind'], string> = {
  'adjacent-qahal': 'Adjacent Qahal',
  'federation-candidate': 'Federation candidate',
  'contributor-presence': 'Contributor presence',
};

/**
 * Graph discovery panel — adjacent Qahals + federation candidates.
 *
 * Visual stub at MVP. Renders a list of discovery suggestions with kind,
 * label, and relevance score. Full graph-walk discovery + commons-elohim
 * federation-candidate generation are deferred to post-MVP.
 *
 * @element elohim-qahal-graph-discovery-panel
 *
 * @prop {DiscoverySuggestion[]} suggestions - Discovery suggestions to render
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual
 * @capabilityRequiredStandings engaged | contributor | steward
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:n/a, error:n/a, stale:supported, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalGraphDiscoveryPanel extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
    }

    h2 {
      font-size: 1rem;
      font-weight: 600;
      margin-block: 0 1rem;
      margin-inline: 0;
    }

    .suggestion-list {
      list-style: none;
      padding-block: 0;
      padding-inline: 0;
      margin-block: 0 1rem;
      margin-inline: 0;
    }

    .suggestion-item {
      padding-block: 0.5rem;
      padding-inline: 0;
      border-block-end: 1px solid var(--elohim-color-border, #ddd);
    }

    .suggestion-item:last-child {
      border-block-end: none;
    }

    .suggestion-row {
      display: flex;
      align-items: baseline;
      gap: 0.5rem;
    }

    .suggestion-kind {
      font-size: 0.75rem;
      font-weight: 600;
      color: var(--elohim-color-fg-2, #555);
      text-transform: uppercase;
      letter-spacing: 0.04em;
      flex-shrink: 0;
    }

    .suggestion-label {
      font-size: 0.875rem;
      font-weight: 500;
      color: var(--elohim-color-fg-1, #222);
      flex: 1 1 auto;
    }

    .suggestion-relevance {
      font-size: 0.8rem;
      color: var(--elohim-color-fg-2, #555);
      flex-shrink: 0;
    }

    .empty-note {
      font-size: 0.875rem;
      color: var(--elohim-color-fg-2, #666);
      font-style: italic;
      margin-block-end: 1rem;
    }

    .stub-note {
      font-size: 0.8rem;
      color: var(--elohim-color-fg-2, #666);
      font-style: italic;
      border-block-start: 1px solid var(--elohim-color-border, #ddd);
      padding-block-start: 0.75rem;
      padding-block-end: 0;
      padding-inline: 0;
      margin-block: 0;
      margin-inline: 0;
    }
  `;

  @property({ type: Array }) suggestions: DiscoverySuggestion[] = [];

  private _renderSuggestion(s: DiscoverySuggestion) {
    const kindLabel = KIND_LABEL[s.kind] ?? s.kind;
    const relevancePct = Math.round(s.relevance * 100);
    return html`
      <li class="suggestion-item" data-kind=${s.kind}>
        <div class="suggestion-row">
          <span class="suggestion-kind">${kindLabel}</span>
          <span class="suggestion-label">${s.label}</span>
          <span class="suggestion-relevance">${relevancePct}%</span>
        </div>
      </li>
    `;
  }

  override render() {
    return html`
      <section aria-label="Graph Discovery">
        <h2>Graph Discovery</h2>
        ${this.suggestions.length === 0
          ? html`
              <p class="empty-note">No suggestions yet.</p>
            `
          : html`
              <ul class="suggestion-list">
                ${this.suggestions.map(s => this._renderSuggestion(s))}
              </ul>
            `}
        <p class="stub-note">
          Visual stub at MVP. Full graph-walk discovery + commons-elohim federation-candidate
          generation are deferred to post-MVP.
        </p>
      </section>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-graph-discovery-panel': ElohimQahalGraphDiscoveryPanel;
  }
}
