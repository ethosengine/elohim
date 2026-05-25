import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

/**
 * <elohim-mention-base> — the generic fallback chip when a specific pillar's
 * mention element (e.g. <elohim-qahal-mention>) isn't loaded. Renders
 * title + EPR id from whatever metadata is resolvable, with a subtle
 * indicator that this is a fallback rendering.
 *
 * Pillar mention elements (e.g. <elohim-lamad-mention>) supersede this
 * element when the relevant pillar bundle is loaded. This element is the
 * safe substrate-level default.
 *
 * @element elohim-mention-base
 *
 * @prop {string} epr    - The EPR id being mentioned
 * @prop {string} label  - Optional display label (rendered if known; falls back to epr)
 * @prop {string} pillar - Optional pillar hint for iconography (reserved for future use)
 *
 * @cssprop --elohim-mention-border       - Override border color
 * @cssprop --elohim-mention-fallback-dot - Override fallback indicator dot color
 * @cssprop --elohim-mention-bg           - Override chip background
 * @cssprop --elohim-mention-fg           - Override chip foreground text color
 * @cssprop --elohim-mention-radius       - Override chip border-radius
 *
 * @csspart chip         - The outer inline chip container
 * @csspart fallback-mark - The dot indicating this is a generic (fallback) render
 * @csspart label        - The display label text span
 * @csspart id           - The EPR id text span (present only when label is shown)
 *
 * @capabilityMaxLens detail
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:designed, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a, unauthorized:n/a
 */
export class ElohimMentionBase extends LitElement {
  @property() epr = '';
  @property() label = '';
  @property() pillar = '';

  static override readonly styles = css`
    :host {
      display: inline-flex;
      align-items: center;
    }

    :host([hidden]) {
      display: none;
    }

    .chip {
      display: inline-flex;
      align-items: center;
      gap: 0.25rem;
      padding-block: 0.125rem;
      padding-inline: 0.5rem;
      background: var(--elohim-mention-bg, transparent);
      color: var(--elohim-mention-fg, inherit);
      border: 1px solid
        var(--elohim-mention-border, color-mix(in oklch, currentColor 25%, transparent));
      border-radius: var(--elohim-mention-radius, 999px);
      font: inherit;
      font-size: 0.875em;
    }

    .label {
      /* display label — no extra styling; inherits chip font */
    }

    .id {
      opacity: 0.6;
      font-size: 0.75em;
    }

    .fallback-mark {
      width: 6px;
      height: 6px;
      background: var(
        --elohim-mention-fallback-dot,
        color-mix(in oklch, currentColor 40%, transparent)
      );
      border-radius: 50%;
      flex-shrink: 0;
    }

    @media (forced-colors: active) {
      .chip {
        background: Canvas;
        color: CanvasText;
        border-color: ButtonText;
      }

      .fallback-mark {
        background: ButtonText;
      }
    }
  `;

  override render() {
    const displayText = this.label || this.epr;
    return html`
      <span class="chip" part="chip">
        <span
          class="fallback-mark"
          part="fallback-mark"
          title="generic chip (pillar element not loaded)"
          role="presentation"
        ></span>
        <span class="label" part="label">${displayText}</span>
        ${this.label ? html`<span class="id" part="id">${this.epr}</span>` : ''}
      </span>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-mention-base': ElohimMentionBase;
  }
}
