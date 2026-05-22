import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export interface ShefaSummary {
  inflows: number;
  outflows: number;
  commonsShareBalance: number;
  agreementClauses: number;
}

/**
 * Shefa resources panel — REA flows summary for this Qahal.
 *
 * Visual stub at MVP. Renders the key REA economic flow dimensions as labelled
 * rows: inflows, outflows, commons-share balance, and agreement clauses. Full
 * REA flow visualization + Agreement cascade rules are deferred to a post-MVP
 * shefa-pillar wiring sprint.
 *
 * @element elohim-qahal-shefa-resources-panel
 *
 * @prop {ShefaSummary} shefa - The REA flows summary for this Qahal
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
export class ElohimQahalShefaResourcesPanel extends CapabilityAwareElement(LitElement) {
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

    .summary-list {
      list-style: none;
      padding-block: 0;
      padding-inline: 0;
      margin-block: 0 1rem;
      margin-inline: 0;
    }

    .summary-item {
      display: flex;
      align-items: baseline;
      gap: 0.5rem;
      padding-block: 0.375rem;
      padding-inline: 0;
      border-block-end: 1px solid var(--elohim-color-border, #ddd);
    }

    .summary-item:last-child {
      border-block-end: none;
    }

    .summary-label {
      font-size: 0.875rem;
      color: var(--elohim-color-fg-2, #555);
      flex: 1 1 auto;
    }

    .summary-value {
      font-size: 0.875rem;
      font-weight: 600;
      color: var(--elohim-color-fg-1, #222);
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

  @property({ type: Object }) shefa: ShefaSummary | null = null;

  override render() {
    if (!this.shefa) {
      return html`
        <section aria-label="Shefa Resources">
          <h2>Shefa Resources</h2>
        </section>
      `;
    }

    const { inflows, outflows, commonsShareBalance, agreementClauses } = this.shefa;

    return html`
      <section aria-label="Shefa Resources">
        <h2>Shefa Resources</h2>
        <ul class="summary-list">
          <li class="summary-item">
            <span class="summary-label">Inflows</span>
            <span class="summary-value">${inflows}</span>
          </li>
          <li class="summary-item">
            <span class="summary-label">Outflows</span>
            <span class="summary-value">${outflows}</span>
          </li>
          <li class="summary-item">
            <span class="summary-label">Commons share balance</span>
            <span class="summary-value">${commonsShareBalance}</span>
          </li>
          <li class="summary-item">
            <span class="summary-label">Agreement clauses</span>
            <span class="summary-value">${agreementClauses}</span>
          </li>
        </ul>
        <p class="stub-note">
          Visual stub at MVP. Full REA flow visualization + Agreement cascade rules are deferred to
          a post-MVP shefa-pillar wiring sprint.
        </p>
      </section>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-shefa-resources-panel': ElohimQahalShefaResourcesPanel;
  }
}
