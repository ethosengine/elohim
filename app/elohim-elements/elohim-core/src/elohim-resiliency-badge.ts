import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export type ResiliencyBadgeState = 'held' | 'thin' | 'unknown' | 'known';

/**
 * <elohim-resiliency-badge> — a link-badge that shows the holders headline it is
 * GIVEN, as a single link to where the content lives (the EPR page). The whole badge
 * is the anchor.
 *
 * It owns no wording logic and does no lookup. The sentence is the resilience fold's
 * own `FeltStatusView.headline` (the chrome's holders reading carries it as
 * `holders.headline`), passed in through `headline`. The only sentence this element
 * adds is the unknown-state one, shown when no headline was supplied:
 * "Held by households · see where it lives".
 *
 * `data-state` reflects the explicit `state` property (`held | thin | unknown`) so
 * themes can colour it; with none it is `known` when a headline is shown and
 * `unknown` otherwise. The sentence is always text, never colour alone.
 *
 * @element elohim-resiliency-badge
 *
 * @prop {string} headline - the sentence to show, verbatim (the fold's headline)
 * @prop {string} href - EPR address the badge links to, e.g. `/epr/<id>`
 * @prop {ResiliencyBadgeState | undefined} state - explicit display state
 *
 * @cssprop --elohim-resiliency-badge-fg - Foreground color (default: inherit)
 * @cssprop --elohim-resiliency-badge-bg - Background (default: transparent)
 * @cssprop --elohim-resiliency-badge-border - Border (default: 1px solid currentColor)
 * @cssprop --elohim-resiliency-badge-radius - Border radius (default: 999px)
 * @cssprop --elohim-resiliency-badge-padding - Padding (default: 0.125rem 0.625rem)
 * @cssprop --elohim-resiliency-badge-held-fg - Foreground when data-state=held
 * @cssprop --elohim-resiliency-badge-thin-fg - Foreground when data-state=thin
 * @cssprop --elohim-resiliency-badge-hover-bg - Background on hover / focus
 *
 * @csspart badge - The anchor
 *
 * @capabilityMaxLens simple
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty observed
 * @capabilityStates empty:n/a, loading:n/a, error:designed, stale:n/a, contested:n/a, offline:designed, unauthorized:n/a
 */
export class ElohimResiliencyBadge extends LitElement {
  @property() headline = '';
  @property() href = '';
  @property() state?: ResiliencyBadgeState;

  static override readonly styles = css`
    :host {
      display: inline-block;
    }

    :host([hidden]) {
      display: none;
    }

    a {
      display: inline-flex;
      align-items: center;
      padding: var(--elohim-resiliency-badge-padding, 0.125rem 0.625rem);
      border: var(--elohim-resiliency-badge-border, 1px solid currentColor);
      border-radius: var(--elohim-resiliency-badge-radius, 999px);
      background: var(--elohim-resiliency-badge-bg, transparent);
      color: var(--elohim-resiliency-badge-fg, inherit);
      font: inherit;
      text-decoration: none;
    }

    a[data-state='held'] {
      color: var(--elohim-resiliency-badge-held-fg, var(--elohim-resiliency-badge-fg, inherit));
    }

    a[data-state='thin'] {
      color: var(--elohim-resiliency-badge-thin-fg, var(--elohim-resiliency-badge-fg, inherit));
    }

    a:hover,
    a:focus-visible {
      background: var(
        --elohim-resiliency-badge-hover-bg,
        color-mix(in oklch, currentColor 8%, transparent)
      );
    }

    @media (pointer: coarse) {
      a {
        min-block-size: 44px;
        min-inline-size: 44px;
      }
    }

    @media (forced-colors: active) {
      a {
        border-color: CanvasText;
        background: ButtonFace;
        color: LinkText;
      }

      a:hover,
      a:focus-visible {
        background: Highlight;
        color: HighlightText;
      }
    }
  `;

  private get derivedState(): ResiliencyBadgeState {
    if (this.state) return this.state;
    return this.headline ? 'known' : 'unknown';
  }

  private get label(): string {
    return this.headline || 'Held by households · see where it lives';
  }

  override render() {
    return html`
      <a part="badge" href=${this.href || '#'} data-state=${this.derivedState}>${this.label}</a>
    `;
  }

  protected override firstUpdated(): void {
    this.setAttribute('data-state', this.derivedState);
  }

  protected override willUpdate(): void {
    this.setAttribute('data-state', this.derivedState);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-resiliency-badge': ElohimResiliencyBadge;
  }
}
