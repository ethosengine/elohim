import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement, nothing } from 'lit';
import { property } from 'lit/decorators.js';

import './elohim-qahal-provenance-marker.js';

export interface ExternalLink {
  id: string;
  url: string;
  label: string;
}

export type ExternalLinkVisibilityMap = Record<
  string,
  'full' | 'filtered_via_co_steward' | 'hidden'
>;

/**
 * External link list — capability-gated external links with offline-grey treatment.
 *
 * Looks up `visibilityMap[viewerTier]` to determine whether to show links fully,
 * with a filtered-via-co-steward annotation, or not at all. When offline, link entries
 * render in greyed-out style via the provenance-marker's offline attribute.
 *
 * Each rendered link composes `<elohim-qahal-provenance-marker category="external-hyperlink">`
 * for the ⤤ marker. Anchors use `target="_blank" rel="noopener noreferrer"`.
 *
 * @element elohim-qahal-external-link-list
 *
 * @prop {ExternalLink[]} links - Array of external link descriptors
 * @prop {ExternalLinkVisibilityMap} visibilityMap - Map from tier string to visibility level
 * @prop {string} viewerTier - The current viewer's capability tier
 * @prop {boolean} offline - When true, render links in greyed-out style
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:supported, unauthorized:supported
 */
export class ElohimQahalExternalLinkList extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
    }

    ul {
      list-style: none;
      padding-block: 0;
      padding-inline: 0;
      margin-block: 0;
      margin-inline: 0;
    }

    li {
      padding-block: 0.25rem;
      padding-inline: 0;
    }

    .link-row {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding-block: 0.375rem;
      padding-inline: 0.5rem;
      border-radius: 0.375rem;
      text-decoration: none;
      font-size: 0.875rem;
      color: var(--elohim-color-fg-1, #222);
    }

    .link-row:hover {
      background: var(--elohim-color-surface-2, #ebebe8);
    }

    .link-row:focus-visible {
      outline: 2px solid var(--elohim-color-focus, #6c9);
      outline-offset: 2px;
    }

    .link-label {
      flex: 1 1 auto;
    }

    .hidden-note,
    .filtered-note,
    .empty-note {
      font-size: 0.875rem;
      color: var(--elohim-color-fg-2, #666);
      font-style: italic;
    }

    .filtered-summary {
      font-size: 0.875rem;
      font-weight: 600;
      color: var(--elohim-color-fg-2, #666);
      cursor: pointer;
      padding-block: 0.25rem;
      padding-inline: 0;
    }

    .filtered-annotation {
      font-size: 0.75rem;
      color: var(--elohim-color-fg-2, #888);
      font-style: italic;
      padding-block: 0 0.5rem;
      padding-inline: 0;
    }
  `;

  @property({ type: Array }) links: ExternalLink[] = [];
  @property({ type: Object }) visibilityMap: ExternalLinkVisibilityMap = {};
  @property({ type: String, attribute: 'viewer-tier' }) viewerTier = '';
  @property({ type: Boolean, reflect: true }) offline = false;

  private _renderLink(link: ExternalLink) {
    return html`
      <li>
        <a
          class="link-row"
          href=${link.url}
          target="_blank"
          rel="noopener noreferrer"
          aria-label=${link.label}
        >
          <elohim-qahal-provenance-marker
            category="external-hyperlink"
            ?offline=${this.offline}
          ></elohim-qahal-provenance-marker>
          <span class="link-label">${link.label}</span>
        </a>
      </li>
    `;
  }

  override render() {
    const visibility = this.visibilityMap[this.viewerTier];

    if (visibility === 'hidden') {
      return html`
        <p class="hidden-note">Links hidden for this tier.</p>
      `;
    }

    if (this.links.length === 0) {
      return html`
        <p class="empty-note">No external links configured.</p>
      `;
    }

    if (visibility === 'filtered_via_co_steward') {
      return html`
        <details>
          <summary class="filtered-summary">External links</summary>
          <p class="filtered-annotation">Filtered via co-steward for this tier.</p>
          <ul>
            ${this.links.map(l => this._renderLink(l))}
          </ul>
        </details>
      `;
    }

    // 'full' or unknown tier (default to full — no restriction)
    return html`
      <ul>
        ${this.links.map(l => this._renderLink(l))}
      </ul>
      ${nothing}
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-external-link-list': ElohimQahalExternalLinkList;
  }
}
