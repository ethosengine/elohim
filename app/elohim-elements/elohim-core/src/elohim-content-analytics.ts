import { css, html, LitElement, nothing } from 'lit';
import { property, state } from 'lit/decorators.js';

import { CapabilityAwareElement } from './capability/index.js';

// ── Types ──────────────────────────────────────────────────────────────────────

export interface ContentAnalyticsMetrics {
  viewCount: number;
  completionCount: number;
  completionRate: number;
}

export interface ContentAnalyticsLoader {
  /** Load analytics for the given contentId; resolves to metrics or null on failure */
  load: (contentId: string) => Promise<ContentAnalyticsMetrics | null>;
}

/**
 * <elohim-content-analytics> — protocol-native attention metrics display primitive.
 *
 * Shows view count, completion count, and completion rate for a content node.
 * Metrics are protocol-native economic events — not external analytics.
 *
 * Service dependencies (the EventService calls) are abstracted via the
 * `loader` property. Callers inject a loader that wraps their service.
 * The element handles loading state and error state itself.
 *
 * Migrated from ContentAnalyticsComponent (app/elohim-app/elohim).
 *
 * @element elohim-content-analytics
 *
 * @prop {string} contentId - Content ID to load analytics for
 * @prop {ContentAnalyticsLoader | null} loader - Analytics data loader
 * @prop {ContentAnalyticsMetrics | null} metrics - Pre-loaded metrics (skips fetch if set)
 *
 * @fires analytics-loaded - { detail: ContentAnalyticsMetrics } when data loads
 * @fires analytics-error - when load fails
 *
 * @cssprop --elohim-analytics-padding - Container padding
 * @cssprop --elohim-analytics-bg - Container background
 * @cssprop --elohim-analytics-fg - Container foreground text color
 * @cssprop --elohim-analytics-title-font - Title font family (display face binding point)
 * @cssprop --elohim-analytics-title-size - Title font size
 * @cssprop --elohim-analytics-value-size - Metric value font size
 * @cssprop --elohim-analytics-value-weight - Metric value font weight
 * @cssprop --elohim-analytics-label-size - Metric label font size
 * @cssprop --elohim-analytics-label-color - Metric label color
 * @cssprop --elohim-analytics-grid-gap - Grid gap between metrics
 * @cssprop --elohim-analytics-note-size - Note text size
 * @cssprop --elohim-analytics-note-color - Note text color
 *
 * @csspart container - The outer container
 * @csspart title - The "Attention Metrics" title
 * @csspart loading - The loading state
 * @csspart grid - The metrics grid
 * @csspart metric - Each metric card
 * @csspart metric-value - The metric value span
 * @csspart metric-label - The metric label span
 * @csspart note - The explanatory note
 * @csspart error - The error state
 *
 * @capabilityMaxLens detail
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual
 * @capabilityRequiredStandings steward | elohim-support
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:designed, loading:designed, error:designed, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimContentAnalytics extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
    }

    :host([hidden]) {
      display: none;
    }

    .container {
      padding: var(--elohim-analytics-padding, 1rem);
      background: var(--elohim-analytics-bg, Canvas);
      color: var(--elohim-analytics-fg, CanvasText);
    }

    .title {
      font-family: var(--elohim-analytics-title-font, inherit);
      font-size: var(--elohim-analytics-title-size, 1rem);
      font-weight: 600;
      margin: 0 0 0.75rem;
    }

    .loading {
      font-size: 0.875rem;
      color: CanvasText;
    }

    .error {
      font-size: 0.875rem;
      color: CanvasText;
    }

    .grid {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: var(--elohim-analytics-grid-gap, 0.75rem);
      margin-block-end: 0.75rem;
    }

    .metric {
      display: flex;
      flex-direction: column;
      gap: 0.125rem;
    }

    .metric-value {
      font-size: var(--elohim-analytics-value-size, 1.5rem);
      font-weight: var(--elohim-analytics-value-weight, 700);
      font-variant-numeric: tabular-nums;
      line-height: 1.2;
    }

    .metric-label {
      font-size: var(--elohim-analytics-label-size, 0.75rem);
      color: var(--elohim-analytics-label-color, CanvasText);
    }

    .note {
      font-size: var(--elohim-analytics-note-size, 0.75rem);
      color: var(--elohim-analytics-note-color, CanvasText);
      font-style: italic;
      margin: 0;
    }

    @media (forced-colors: active) {
      .container {
        border: 1px solid CanvasText;
        background: Canvas;
        color: CanvasText;
      }

      .metric-label,
      .note,
      .loading,
      .error {
        color: CanvasText;
      }
    }
  `;

  @property({ attribute: 'content-id' }) contentId = '';

  @property({ attribute: false })
  loader: ContentAnalyticsLoader | null = null;

  @property({ attribute: false })
  metrics: ContentAnalyticsMetrics | null = null;

  @state() private isLoading = false;
  @state() private hasError = false;
  @state() private loadedMetrics: ContentAnalyticsMetrics | null = null;

  override updated(changed: Map<string | symbol, unknown>): void {
    super.updated(changed);
    if (changed.has('contentId') && !this.metrics) {
      void this.loadAnalytics();
    }
    if (changed.has('metrics') && this.metrics) {
      this.loadedMetrics = this.metrics;
    }
  }

  override render() {
    const display = this.metrics ?? this.loadedMetrics;

    return html`
      <div class="container" part="container" data-testid="content-analytics">
        <h3 class="title" part="title">Attention Metrics</h3>

        ${this.isLoading
          ? html`
              <div class="loading" part="loading" data-testid="analytics-loading">
                Loading metrics...
              </div>
            `
          : nothing}
        ${this.hasError && !display
          ? html`
              <div class="error" part="error" data-testid="analytics-error">
                Metrics unavailable.
              </div>
            `
          : nothing}
        ${!this.isLoading && display
          ? html`
              <div class="grid" part="grid">
                <div class="metric" part="metric" data-testid="analytics-views">
                  <span class="metric-value" part="metric-value">${display.viewCount}</span>
                  <span class="metric-label" part="metric-label">Views</span>
                </div>
                <div class="metric" part="metric" data-testid="analytics-completions">
                  <span class="metric-value" part="metric-value">${display.completionCount}</span>
                  <span class="metric-label" part="metric-label">Completions</span>
                </div>
                <div class="metric" part="metric" data-testid="analytics-completion-rate">
                  <span class="metric-value" part="metric-value">${display.completionRate}%</span>
                  <span class="metric-label" part="metric-label">Completion Rate</span>
                </div>
              </div>
            `
          : nothing}
        ${this.isLoading
          ? nothing
          : html`
              <p class="note" part="note">
                Metrics are protocol-native economic events, not external analytics. Views are
                recorded after 3 seconds of engagement.
              </p>
            `}
      </div>
    `;
  }

  private async loadAnalytics(): Promise<void> {
    if (!this.contentId || !this.loader) return;

    this.isLoading = true;
    this.hasError = false;

    try {
      const result = await this.loader.load(this.contentId);
      if (result) {
        this.loadedMetrics = result;
        this.dispatchEvent(
          new CustomEvent('analytics-loaded', {
            detail: result,
            bubbles: true,
            composed: true,
          })
        );
      } else {
        this.hasError = true;
        this.dispatchEvent(new CustomEvent('analytics-error', { bubbles: true, composed: true }));
      }
    } catch {
      this.hasError = true;
      this.dispatchEvent(new CustomEvent('analytics-error', { bubbles: true, composed: true }));
    } finally {
      this.isLoading = false;
    }
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-content-analytics': ElohimContentAnalytics;
  }
}
