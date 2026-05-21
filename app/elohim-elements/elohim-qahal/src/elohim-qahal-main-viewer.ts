import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

/**
 * Qahal main viewer — third column. Holds the active panel's rendered content.
 *
 * Used in the Qahal homepage 4-column chrome as the primary content area.
 * The default slot accepts the active panel element; `activePanelName` is used
 * for ARIA labelling of the `<main>` region.
 *
 * @element elohim-qahal-main-viewer
 *
 * @prop {string} activePanelName - Name of the currently-active panel (for ARIA labelling)
 *
 * @slot - The active panel's element
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
export class ElohimQahalMainViewer extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      padding-block: 1.5rem;
      padding-inline: 1.5rem;
      background: var(--elohim-color-surface-0, #fff);
      block-size: 100%;
      overflow-y: auto;
    }
  `;

  @property({ type: String, attribute: 'active-panel-name' }) activePanelName = '';

  override render() {
    return html`
      <main aria-label=${this.activePanelName || 'Active panel'}>
        <slot></slot>
      </main>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-main-viewer': ElohimQahalMainViewer;
  }
}
