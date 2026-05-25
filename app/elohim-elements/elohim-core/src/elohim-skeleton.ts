import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

/**
 * <elohim-skeleton> — sized shimmer placeholder used during progressive
 * loading. Renders an empty box at the declared width/height with a
 * subtle shimmer animation. Page doesn't reflow when real content arrives.
 *
 * @element elohim-skeleton
 *
 * @prop {string} width  - CSS width (e.g. "200px" or "100%")
 * @prop {string} height - CSS height
 * @prop {string} radius - Optional border-radius (default: var(--elohim-radius-sm, 4px))
 *
 * @cssprop --elohim-skeleton-bg       - Override base color
 * @cssprop --elohim-skeleton-shimmer  - Override shimmer highlight color
 * @cssprop --elohim-skeleton-radius   - Override border-radius
 *
 * @capabilityMaxLens detail
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:n/a, loading:designed, error:n/a, stale:n/a, contested:n/a, offline:n/a, unauthorized:n/a
 */
export class ElohimSkeleton extends LitElement {
  @property() width = '100%';
  @property() height = '1rem';
  @property() radius = 'var(--elohim-radius-sm, 4px)';

  static override readonly styles = css`
    :host {
      display: inline-block;
      background: var(--elohim-skeleton-bg, color-mix(in oklch, currentColor 12%, transparent));
      border-radius: var(--elohim-skeleton-radius, var(--elohim-radius-sm, 4px));
      position: relative;
      overflow: hidden;
    }

    :host([hidden]) {
      display: none;
    }

    :host::before {
      content: '';
      position: absolute;
      inset: 0;
      background: linear-gradient(
        90deg,
        transparent 0%,
        var(
            --elohim-skeleton-shimmer,
            color-mix(in oklch, currentColor 20%, transparent)
          )
          50%,
        transparent 100%
      );
    }

    @media (prefers-reduced-motion: no-preference) and (update: fast) {
      :host::before {
        animation: elohim-skeleton-shimmer 1.5s infinite;
      }

      @keyframes elohim-skeleton-shimmer {
        0% {
          transform: translateX(-100%);
        }
        100% {
          transform: translateX(100%);
        }
      }
    }

    @media (forced-colors: active) {
      :host {
        background: ButtonFace;
        border: 1px solid ButtonText;
      }

      :host::before {
        display: none;
      }
    }
  `;

  protected override updated(): void {
    this.style.width = this.width;
    this.style.height = this.height;
    this.style.setProperty('--elohim-skeleton-radius', this.radius);
  }

  override render() {
    return html``;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-skeleton': ElohimSkeleton;
  }
}
