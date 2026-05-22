import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export interface AttestationSummary {
  recentCount: number;
  byKindCounts: Record<string, number>;
  lastAttestationAt: string;
}

/**
 * Attestations panel — viewer's attestation history in this Qahal.
 *
 * Visual stub at MVP. Renders a recent-count headline, a by-kind breakdown
 * table, and the last-attestation timestamp. Per-attestation drill-down +
 * provenance trace are deferred to post-MVP.
 *
 * @element elohim-qahal-attestations-panel
 *
 * @prop {AttestationSummary} attestations - The viewer's attestation summary in this Qahal
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
export class ElohimQahalAttestationsPanel extends CapabilityAwareElement(LitElement) {
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

    .recent-headline {
      margin-block-end: 1rem;
    }

    .recent-label {
      font-size: 0.8rem;
      color: var(--elohim-color-fg-2, #666);
      text-transform: uppercase;
      letter-spacing: 0.04em;
      display: block;
    }

    .recent-count {
      font-size: 2rem;
      font-weight: 700;
      line-height: 1.1;
      color: var(--elohim-color-fg-1, #222);
    }

    .kind-table {
      border-collapse: collapse;
      inline-size: 100%;
      margin-block-end: 0.75rem;
    }

    .kind-table th,
    .kind-table td {
      text-align: start;
      padding-block: 0.3rem;
      padding-inline: 0.5rem;
      border-block-end: 1px solid var(--elohim-color-border, #ddd);
    }

    .kind-table th {
      font-size: 0.75rem;
      font-weight: 600;
      color: var(--elohim-color-fg-2, #555);
      text-transform: uppercase;
      letter-spacing: 0.04em;
    }

    .kind-table td {
      font-size: 0.875rem;
      color: var(--elohim-color-fg-1, #222);
    }

    .kind-table tr:last-child td {
      border-block-end: none;
    }

    .last-at {
      font-size: 0.8rem;
      color: var(--elohim-color-fg-2, #666);
      margin-block: 0 1rem;
      margin-inline: 0;
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

  @property({ type: Object }) attestations: AttestationSummary | null = null;

  override render() {
    if (!this.attestations) {
      return html`
        <section aria-label="Attestations">
          <h2>Attestations</h2>
        </section>
      `;
    }

    const { recentCount, byKindCounts, lastAttestationAt } = this.attestations;
    const kinds = Object.entries(byKindCounts);

    return html`
      <section aria-label="Attestations">
        <h2>Attestations</h2>
        <div class="recent-headline" aria-label="Recent attestations: ${recentCount}">
          <span class="recent-label">Recent</span>
          <span class="recent-count">${recentCount}</span>
        </div>
        ${kinds.length > 0
          ? html`
              <table class="kind-table">
                <thead>
                  <tr>
                    <th scope="col">Kind</th>
                    <th scope="col">Count</th>
                  </tr>
                </thead>
                <tbody>
                  ${kinds.map(
                    ([kind, count]) => html`
                      <tr>
                        <td>${kind}</td>
                        <td>${count}</td>
                      </tr>
                    `
                  )}
                </tbody>
              </table>
            `
          : ''}
        <p class="last-at">Last attestation: ${lastAttestationAt}</p>
        <p class="stub-note">
          Visual stub at MVP. Per-attestation drill-down + provenance trace are deferred to
          post-MVP.
        </p>
      </section>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-attestations-panel': ElohimQahalAttestationsPanel;
  }
}
