import { css, html, LitElement, type PropertyValues } from 'lit';
import { property, state } from 'lit/decorators.js';

export type ResiliencyBadgeState = 'held' | 'thin' | 'unknown';

/** The slice of the resilience snapshot view (`feltStatus.floor`) this badge reads. */
interface ResilienceFloorSlice {
  feltStatus?: { floor?: { hasHouseholds?: number; wantsHouseholds?: number } | null } | null;
}

/**
 * <elohim-resiliency-badge> — how many households hold a piece of content, as a
 * single link to where it lives (the EPR page). The whole badge is the anchor.
 *
 * Wording is ported from the EPR page's held chip: "Held by {holders} of {target}
 * households"; zero holders reads "Not yet held by any household"; when the figures
 * are unknown, "Held by households · see where it lives".
 *
 * Figures come from the `holders` / `target` properties, or — when `content-id` is
 * set and neither is supplied — from a same-origin fetch of
 * `{base-url}/api/v1/resilience/{content-id}/household`, reading
 * `feltStatus.floor.hasHouseholds` / `wantsHouseholds`. A failed or unmeasured
 * fetch silently keeps the unknown state.
 *
 * `data-state` reflects `held | thin | unknown` (thin when holders < target) so
 * themes can colour it; the figures are always text, never colour alone.
 *
 * @element elohim-resiliency-badge
 *
 * @prop {string} contentId - (attr `content-id`) content to look up when figures are not supplied
 * @prop {string} href - EPR address the badge links to, e.g. `/epr/<id>`
 * @prop {number | undefined} holders - households holding the content
 * @prop {number | undefined} target - households wanted to hold it
 * @prop {ResiliencyBadgeState | undefined} state - explicit state override
 * @prop {string} baseUrl - (attr `base-url`) prefix for the lookup; default same-origin
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
  @property({ attribute: 'content-id' }) contentId = '';
  @property() href = '';
  @property({ type: Number }) holders?: number;
  @property({ type: Number }) target?: number;
  @property() state?: ResiliencyBadgeState;
  @property({ attribute: 'base-url' }) baseUrl = '';

  @state() private fetchedHolders?: number;
  @state() private fetchedTarget?: number;

  private fetchSeq = 0;

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

  protected override updated(changed: PropertyValues): void {
    if (changed.has('contentId') || changed.has('baseUrl')) void this.lookup();
  }

  private get figures(): { holders?: number; target?: number } {
    const supplied = this.holders !== undefined && this.target !== undefined;
    return supplied
      ? { holders: this.holders, target: this.target }
      : { holders: this.fetchedHolders, target: this.fetchedTarget };
  }

  private get derivedState(): ResiliencyBadgeState {
    if (this.state) return this.state;
    const { holders, target } = this.figures;
    if (holders === undefined || target === undefined) return 'unknown';
    return holders < target ? 'thin' : 'held';
  }

  private get label(): string {
    const { holders, target } = this.figures;
    if (holders === undefined || target === undefined) {
      return 'Held by households · see where it lives';
    }
    return holders === 0
      ? 'Not yet held by any household'
      : `Held by ${holders} of ${target} households`;
  }

  private async lookup(): Promise<void> {
    const seq = ++this.fetchSeq;
    this.fetchedHolders = undefined;
    this.fetchedTarget = undefined;
    if (!this.contentId || (this.holders !== undefined && this.target !== undefined)) return;
    try {
      const url = `${this.baseUrl.replace(/\/$/, '')}/api/v1/resilience/${encodeURIComponent(
        this.contentId
      )}/household`;
      const res = await fetch(url);
      if (!res.ok) return;
      const body = (await res.json()) as ResilienceFloorSlice;
      const floor = body.feltStatus?.floor;
      if (seq !== this.fetchSeq || !floor) return;
      if (typeof floor.hasHouseholds === 'number' && typeof floor.wantsHouseholds === 'number') {
        this.fetchedHolders = floor.hasHouseholds;
        this.fetchedTarget = floor.wantsHouseholds;
      }
    } catch {
      // Silent: the unknown state is the honest fallback.
    }
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
