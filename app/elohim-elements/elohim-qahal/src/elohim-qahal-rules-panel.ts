import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export type BloomTierKey = 'remember' | 'understand' | 'apply' | 'analyze' | 'evaluate' | 'create';

export interface BloomMapping {
  tier: BloomTierKey;
  standingLabel: string;
}

export interface Rubric {
  name: string;
  standingHonors: string[];
  bloomMapping: BloomMapping[];
  cadenceLabel: string;
  frictionGradientNote: string;
  configuredBy: string[];
  lastRevised: string;
}

/**
 * Rules panel — renders the Qahal rubric in human-readable form.
 *
 * Displays the rubric name, standing honors, Bloom-tier progression mapping,
 * cadence, friction-gradient note, configured-by, and last-revised date.
 *
 * @element elohim-qahal-rules-panel
 *
 * @prop {Rubric | null} rubric - The rubric to render; null renders empty state
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual
 * @capabilityRequiredStandings visitor | engaged | contributor | steward
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:n/a, error:n/a, stale:supported, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalRulesPanel extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
    }

    h2 {
      font-size: 1rem;
      font-weight: 700;
      margin-block: 0 1rem;
      margin-inline: 0;
    }

    h3 {
      font-size: 0.875rem;
      font-weight: 600;
      margin-block: 1rem 0.375rem;
      margin-inline: 0;
      color: var(--elohim-color-fg-2, #555);
    }

    ul {
      list-style: disc;
      padding-inline-start: 1.25rem;
      margin-block: 0;
      margin-inline: 0;
    }

    li {
      font-size: 0.875rem;
      margin-block-end: 0.25rem;
    }

    dl {
      display: grid;
      grid-template-columns: auto 1fr;
      row-gap: 0.25rem;
      column-gap: 0.75rem;
      margin-block: 0;
      margin-inline: 0;
    }

    dt {
      font-size: 0.75rem;
      font-weight: 600;
      color: var(--elohim-color-fg-2, #555);
      text-transform: capitalize;
    }

    dd {
      font-size: 0.875rem;
      margin-inline-start: 0;
      color: var(--elohim-color-fg-1, #222);
    }

    .meta-row {
      font-size: 0.8rem;
      color: var(--elohim-color-fg-2, #666);
      margin-block-end: 0.375rem;
    }

    .meta-label {
      font-weight: 600;
    }

    .empty {
      font-size: 0.875rem;
      color: var(--elohim-color-fg-2, #666);
      font-style: italic;
    }
  `;

  @property({ type: Object }) rubric: Rubric | null = null;

  override render() {
    if (!this.rubric) {
      return html`
        <p class="empty">No rubric configured for this Qahal.</p>
      `;
    }
    const r = this.rubric;
    return html`
      <section aria-label="Rules">
        <h2>${r.name}</h2>

        <h3>Standing in this Qahal is:</h3>
        <ul>
          ${r.standingHonors.map(
            honor => html`
              <li>${honor}</li>
            `
          )}
        </ul>

        <h3>Mastery progression (Bloom curve):</h3>
        <dl>
          ${r.bloomMapping.map(
            bm => html`
              <dt>${bm.tier}</dt>
              <dd>${bm.standingLabel}</dd>
            `
          )}
        </dl>

        <div class="meta-row">
          <span class="meta-label">Cadence:</span>
          ${r.cadenceLabel}
        </div>
        <div class="meta-row">
          <span class="meta-label">Friction-gradient:</span>
          ${r.frictionGradientNote}
        </div>
        <div class="meta-row">
          <span class="meta-label">Configured by:</span>
          ${r.configuredBy.join(', ')}
        </div>
        <div class="meta-row">
          <span class="meta-label">Last revised:</span>
          ${r.lastRevised}
        </div>
      </section>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-rules-panel': ElohimQahalRulesPanel;
  }
}
