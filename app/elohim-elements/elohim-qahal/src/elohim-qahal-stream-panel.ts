import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

import './elohim-qahal-care-economy-marker.js';
import './elohim-qahal-imagodei-badge.js';

export interface StreamEventRea {
  kind: 'care' | 'presence' | 'repair' | 'growth' | 'time';
  tokens: number;
}

export interface StreamEvent {
  id: string;
  authorId: string;
  timestamp: string;
  content: string;
  rea?: StreamEventRea;
  acknowledgmentPending?: boolean;
}

/**
 * Stream panel — renders the commons stream as a feed.
 *
 * Consumes an array of StreamEvent objects and renders each as a list item
 * with an author badge, timestamp, content, and optional care-economy marker.
 * Events with acknowledgmentPending=true receive visual emphasis.
 * Events authored by commons-elohim receive distinct styling.
 *
 * @element elohim-qahal-stream-panel
 *
 * @prop {StreamEvent[]} events - Array of stream events to render
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings visitor | engaged | contributor | steward
 * @capabilityContentCertainty observed-private
 * @capabilityStates empty:supported, loading:n/a, error:n/a, stale:supported, contested:n/a, offline:supported, unauthorized:n/a
 */
export class ElohimQahalStreamPanel extends CapabilityAwareElement(LitElement) {
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

    .stream-list {
      list-style: none;
      padding-block: 0;
      padding-inline: 0;
      margin-block: 0;
      margin-inline: 0;
    }

    .stream-item {
      padding-block: 0.75rem;
      padding-inline: 0.75rem;
      border-block-end: 1px solid var(--elohim-color-border, #ddd);
    }

    .stream-item:last-child {
      border-block-end: none;
    }

    .stream-item[data-acknowledgment-pending] {
      border: 1px dashed var(--elohim-color-accent, #99c);
      border-radius: 4px;
    }

    .stream-item[data-commons-elohim] {
      background: var(--elohim-color-surface-1, #fafafa);
      border-inline-start: 3px solid var(--elohim-color-accent, #99c);
    }

    .stream-header {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      margin-block-end: 0.375rem;
    }

    .stream-timestamp {
      font-size: 0.75rem;
      color: var(--elohim-color-fg-2, #666);
      margin-inline-start: auto;
    }

    .stream-content {
      font-size: 0.875rem;
      margin-block: 0.25rem;
      margin-inline: 0;
      color: var(--elohim-color-fg-1, #222);
    }

    .stream-rea {
      margin-block-start: 0.375rem;
    }

    .stream-empty {
      font-size: 0.875rem;
      color: var(--elohim-color-fg-2, #666);
      font-style: italic;
      padding-block: 1rem;
      padding-inline: 0;
    }
  `;

  @property({ type: Array }) events: StreamEvent[] = [];

  private _renderItem(event: StreamEvent) {
    const isCommonsElohim = event.authorId === 'commons-elohim';
    return html`
      <li
        class="stream-item"
        ?data-acknowledgment-pending=${event.acknowledgmentPending ?? false}
        ?data-commons-elohim=${isCommonsElohim}
      >
        <div class="stream-header">
          <elohim-qahal-imagodei-badge
            name=${event.authorId}
            standing-tier="visitor"
          ></elohim-qahal-imagodei-badge>
          <time class="stream-timestamp" datetime=${event.timestamp}>${event.timestamp}</time>
        </div>
        <p class="stream-content">${event.content}</p>
        ${event.rea
          ? html`
              <div class="stream-rea">
                <elohim-qahal-care-economy-marker
                  kind=${event.rea.kind}
                  tokens=${event.rea.tokens}
                ></elohim-qahal-care-economy-marker>
              </div>
            `
          : ''}
      </li>
    `;
  }

  override render() {
    return html`
      <section aria-label="Stream">
        <h2>Stream</h2>
        ${this.events.length === 0
          ? html`
              <p class="stream-empty">No activity yet — the household is quiet.</p>
            `
          : html`
              <ol class="stream-list">
                ${this.events.map(e => this._renderItem(e))}
              </ol>
            `}
      </section>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-qahal-stream-panel': ElohimQahalStreamPanel;
  }
}
