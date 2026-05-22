import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

/**
 * Co-steward panel — observational right-context view from the commons-elohim.
 *
 * Renders the commons-elohim's primary observation prominently. When pending
 * acknowledgments are present, lists them with a soft no-urgency footer.
 * Tone discipline: no "alert", "warning", or exclamation marks in any
 * rendered content.
 *
 * @element elohim-qahal-co-steward-panel
 *
 * @prop {string} primaryObservation - The co-steward's primary observation text
 * @prop {string[]} pendingAcknowledgments - Optional list of pending care acknowledgment labels
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
export class ElohimQahalCoStewardPanel extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
    }

    .primary-observation {
      font-size: 1rem;
      font-weight: 500;
      color: var(--elohim-color-fg-1, #222);
      margin-block: 0 1rem;
      margin-inline: 0;
    }

    .pending-heading {
      font-size: 0.8rem;
      font-weight: 600;
      color: var(--elohim-color-fg-2, #555);
      margin-block: 0 0.5rem;
      margin-inline: 0;
    }

    .pending-list {
      list-style: disc;
      padding-inline-start: 1.25rem;
      margin-block: 0 0.75rem;
      margin-inline: 0;
    }

    .pending-list li {
      font-size: 0.875rem;
      margin-block-end: 0.25rem;
      color: var(--elohim-color-fg-1, #222);
    }

    .no-urgency-footer {
      font-size: 0.8rem;
      color: var(--elohim-color-fg-2, #666);
      font-style: italic;
      margin-block: 0;
      margin-inline: 0;
    }
  `;

  @property({ type: String, attribute: 'primary-observation' })
  primaryObservation = '';

  @property({ type: Array })
  pendingAcknowledgments: string[] = [];

  override render() {
    const hasPending = this.pendingAcknowledgments.length > 0;
    return html`
      <div>
        <p class="primary-observation">${this.primaryObservation}</p>
        ${hasPending
          ? html`
              <p class="pending-heading">Three care contributions pending acknowledgment:</p>
              <ul class="pending-list">
                ${this.pendingAcknowledgments.map(
                  ack => html`
                    <li>${ack}</li>
                  `
                )}
              </ul>
              <p class="no-urgency-footer">
                No urgency. You can acknowledge them when you next sit down.
              </p>
            `
          : ''}
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-co-steward-panel': ElohimQahalCoStewardPanel;
  }
}
