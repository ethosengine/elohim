import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

/**
 * Qahal sidebar — second column. Layout container for resource list sections.
 *
 * Used in the Qahal homepage 4-column chrome as the second column.
 * Provides four named slots for the four resource-list section types.
 *
 * @element elohim-qahal-sidebar
 *
 * @prop {string} qahalName - Display name of the active Qahal
 *
 * @slot panels - The protocol panels list section
 * @slot curated - The curated EPRs section
 * @slot external - The external hyperlinks section
 * @slot power-user - The power-user expandables section
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
export class ElohimQahalSidebar extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      padding: 1rem;
      background: var(--elohim-color-surface-1, #fafafa);
      border-inline-end: 1px solid var(--elohim-color-border, #ddd);
      block-size: 100%;
      overflow-y: auto;
    }

    h2 {
      font-size: 0.95rem;
      font-weight: 600;
      margin-block: 0 0.75rem;
      margin-inline: 0;
    }

    section {
      margin-block-end: 1rem;
    }

    .section-divider {
      border: 0;
      border-block-start: 1px solid var(--elohim-color-border, #ddd);
      margin-block: 0.75rem;
    }
  `;

  @property({ type: String, attribute: 'qahal-name' }) qahalName = '';

  override render() {
    return html`
      <h2>${this.qahalName}</h2>
      <section><slot name="panels"></slot></section>
      <hr class="section-divider" />
      <section><slot name="curated"></slot></section>
      <hr class="section-divider" />
      <section><slot name="external"></slot></section>
      <hr class="section-divider" />
      <section><slot name="power-user"></slot></section>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-sidebar': ElohimQahalSidebar;
  }
}
