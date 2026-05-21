import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';

/**
 * Qahal context column — fourth column. Right-context panels.
 *
 * Used in the Qahal homepage 4-column chrome as the rightmost column.
 * Provides three named slots for co-steward, rules, and discovery content.
 *
 * @element elohim-qahal-context-column
 *
 * @slot co-steward - The commons-elohim co-steward view (always present)
 * @slot rules - Condensed rules summary
 * @slot discovery - Graph discovery suggestions
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:supported, error:supported, stale:n/a, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalContextColumn extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      padding: 1rem;
      background: var(--elohim-color-surface-1, #fafafa);
      border-inline-start: 1px solid var(--elohim-color-border, #ddd);
      block-size: 100%;
      overflow-y: auto;
    }

    section {
      margin-block-end: 1.25rem;
    }

    h3 {
      font-size: 0.85rem;
      font-weight: 600;
      margin-block: 0 0.5rem;
      margin-inline: 0;
      color: var(--elohim-color-fg-2, #666);
      text-transform: uppercase;
      letter-spacing: 0.04em;
    }
  `;

  override render() {
    return html`
      <aside aria-label="Context">
        <section>
          <h3>Co-steward</h3>
          <slot name="co-steward"></slot>
        </section>
        <section>
          <h3>Rules</h3>
          <slot name="rules"></slot>
        </section>
        <section>
          <h3>Discovery</h3>
          <slot name="discovery"></slot>
        </section>
      </aside>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-context-column': ElohimQahalContextColumn;
  }
}
