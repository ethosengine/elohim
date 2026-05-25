import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export interface AttestorRef {
  eprRef: string;
  displayName: string;
  role: 'qahal-elder' | 'intimate-circle' | 'recovery-witness' | 'elohim-agent' | string;
}

/**
 * <elohim-imagodei-attestor-row> — avatar row of qahal / circle / witness
 * attestors. Foregrounds the social-trust model — your community attests
 * you in both trust modes. Overflow shows +N more. Empty state via named slot.
 *
 * Security framing: this row makes the community-attestation security model
 * VISIBLE without requiring the user to read docs. "Your people have your
 * back" — not crypto-bro sovereign-key framing. See spec §0 + §1.1 + §2.3.
 *
 * @element elohim-imagodei-attestor-row
 *
 * @prop {AttestorRef[]} attestors - array of attestor descriptors
 * @prop {number} maxVisible - max avatars to render before overflow; default 5
 * @prop {'compact' | 'standard'} density - layout density
 *
 * @slot empty - rendered when attestors array is empty
 *
 * @fires {CustomEvent<{eprRef: string}>} attestor-tap - emitted on avatar click
 *
 * @cssprop --elohim-attestor-ring - Avatar ring / border color (default: Canvas)
 * @cssprop --elohim-attestor-avatar-size - Avatar diameter (default: 28px)
 * @cssprop --elohim-attestor-gap - Gap between avatars (default: 0.25rem)
 * @cssprop --elohim-attestor-bg - Avatar background (default: 14% currentColor mix)
 * @cssprop --elohim-attestor-overflow-opacity - Opacity of the +N label (default: 0.7)
 * @cssprop --elohim-attestor-empty-opacity - Opacity of the empty state (default: 0.7)
 * @cssprop --elohim-attestor-font-size - Base font size override
 *
 * @csspart row - The flex row container (visible attestors + overflow)
 * @csspart avatar - Each individual avatar button
 * @csspart overflow - The "+N more" span
 * @csspart empty-wrapper - Wraps the empty slot
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings any
 * @capabilityContentCertainty observed
 * @capabilityStates empty:designed, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimImagodeiAttestorRow extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: inline-block;
      font: inherit;
      font-size: var(--elohim-attestor-font-size, inherit);
    }

    ul {
      display: inline-flex;
      align-items: center;
      gap: var(--elohim-attestor-gap, 0.25rem);
      list-style: none;
      margin: 0;
      padding: 0;
    }

    li {
      display: contents;
    }

    [part='avatar'] {
      inline-size: var(--elohim-attestor-avatar-size, 28px);
      block-size: var(--elohim-attestor-avatar-size, 28px);
      border-radius: 50%;
      background: var(
        --elohim-attestor-bg,
        color-mix(in srgb, currentColor 14%, transparent)
      );
      border: 2px solid var(--elohim-attestor-ring, Canvas);
      display: inline-flex;
      align-items: center;
      justify-content: center;
      font-size: 0.75rem;
      color: inherit;
      cursor: pointer;
      padding: 0;
      line-height: 1;
    }

    [part='avatar']:hover {
      background: color-mix(in srgb, currentColor 22%, transparent);
    }

    [part='avatar']:focus-visible {
      outline: 2px solid currentColor;
      outline-offset: 2px;
    }

    [part='overflow'] {
      margin-inline-start: 0.25rem;
      font-size: 0.75rem;
      opacity: var(--elohim-attestor-overflow-opacity, 0.7);
    }

    [part='empty-wrapper'] {
      font-size: 0.875rem;
      opacity: var(--elohim-attestor-empty-opacity, 0.7);
    }

    @media (forced-colors: active) {
      [part='avatar'] {
        border-color: CanvasText;
        background: ButtonFace;
        color: ButtonText;
      }

      [part='avatar']:hover,
      [part='avatar']:focus-visible {
        background: Highlight;
        color: HighlightText;
        outline: 2px solid ButtonText;
      }

      [part='overflow'] {
        color: CanvasText;
      }
    }
  `;

  @property({ attribute: false }) attestors: AttestorRef[] = [];

  @property({ type: Number, attribute: 'max-visible' }) maxVisible = 5;

  @property() density: 'compact' | 'standard' = 'standard';

  override render() {
    if (this.attestors.length === 0) {
      return html`
        <div part="empty-wrapper">
          <slot name="empty">No witnesses recorded yet.</slot>
        </div>
      `;
    }

    const visible = this.attestors.slice(0, this.maxVisible);
    const overflow = this.attestors.length - visible.length;

    return html`
      <ul part="row" aria-label="Attestors">
        ${visible.map(
          a => html`
            <li>
              <button
                type="button"
                part="avatar"
                title="${a.displayName} (${a.role})"
                aria-label="${a.displayName}, ${a.role}"
                @click=${() => this._onTap(a.eprRef)}
              >
                ${this._initials(a.displayName)}
              </button>
            </li>
          `
        )}
        ${overflow > 0
          ? html`<li><span part="overflow">+${overflow}</span></li>`
          : ''}
      </ul>
    `;
  }

  private _initials(name: string): string {
    const parts = name.trim().split(/\s+/);
    const first = parts[0];
    if (!first) return '?';
    if (parts.length === 1) return first.slice(0, 2).toUpperCase();
    const last = parts[parts.length - 1];
    return ((first[0] ?? '') + (last?.[0] ?? '')).toUpperCase();
  }

  private _onTap(eprRef: string) {
    this.dispatchEvent(
      new CustomEvent('attestor-tap', {
        detail: { eprRef },
        bubbles: true,
        composed: true,
      })
    );
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-imagodei-attestor-row': ElohimImagodeiAttestorRow;
  }
}
