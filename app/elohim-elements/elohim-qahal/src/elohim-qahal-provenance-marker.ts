import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export type ProvenanceCategory =
  | 'protocol-panel'
  | 'curated-epr'
  | 'installed-applet'
  | 'external-hyperlink';

const SYMBOLS: Record<ProvenanceCategory, string> = {
  'protocol-panel': '●',
  'curated-epr': '◆',
  'installed-applet': '⬢',
  'external-hyperlink': '⤤',
};

const LABELS: Record<ProvenanceCategory, string> = {
  'protocol-panel': 'protocol panel',
  'curated-epr': 'curated EPR',
  'installed-applet': 'installed applet',
  'external-hyperlink': 'external hyperlink (leaving the elohim network)',
};

/**
 * Provenance marker — symbol denoting which substrate category an item belongs to.
 *
 * Used in the resource list sidebar to distinguish protocol-panel, curated EPR,
 * installed-applet, and external-hyperlink items at a glance.
 *
 * @element elohim-qahal-provenance-marker
 * @attr {string} category - The provenance category (protocol-panel | curated-epr | installed-applet | external-hyperlink)
 * @attr {boolean} offline - When true, render in greyed-out style (applicable to external-hyperlink)
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalProvenanceMarker extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: inline-block;
      font-size: 0.875rem;
    }

    :host([offline]) [aria-label] {
      opacity: 0.4;
      filter: grayscale(1);
    }
  `;

  @property({ type: String, reflect: true }) category: ProvenanceCategory = 'protocol-panel';
  @property({ type: Boolean, reflect: true }) offline = false;

  override render() {
    const symbol = (SYMBOLS as Record<string, string>)[this.category] ?? '';
    const label = (LABELS as Record<string, string>)[this.category] ?? this.category;
    return html`
      <span aria-label=${label}>${symbol}</span>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-provenance-marker': ElohimQahalProvenanceMarker;
  }
}
