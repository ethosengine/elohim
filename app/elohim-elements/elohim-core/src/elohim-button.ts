import { css, html, LitElement } from 'lit';
import { property } from 'lit/decorators.js';

export type ElohimButtonVariant = 'primary' | 'secondary' | 'ghost';

/**
 * The elohim button atom — substrate primitive for action affordances.
 *
 * Token-driven; respects light/dark theme via the global tokens cascade.
 *
 * @element elohim-button
 *
 * @prop {ElohimButtonVariant} variant - Visual variant: primary | secondary | ghost
 * @prop {boolean} disabled - Disabled state. Suppresses click and applies aria-disabled.
 *
 * @fires {MouseEvent} click - Fired on activation (mouse or keyboard via native button). Native bubbling click from the inner <button>.
 *
 * @slot - Default slot for label content (text or icon+text)
 *
 * @cssprop --elohim-button-bg - Override background color
 * @cssprop --elohim-button-fg - Override foreground (label) color
 * @cssprop --elohim-button-border - Override border style
 * @cssprop --elohim-button-radius - Override border-radius
 *
 * @csspart button - The internal native <button> element
 */
export class ElohimButton extends LitElement {
  /** @ignore */
  static override readonly shadowRootOptions: ShadowRootInit = {
    ...LitElement.shadowRootOptions,
    delegatesFocus: true,
  };

  static override readonly styles = css`
    :host {
      display: inline-block;
    }

    :host([hidden]) {
      display: none;
    }

    button {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 0.5rem;
      min-height: 44px;
      min-width: 44px;
      padding: 0.625rem 1.25rem;
      font: inherit;
      font-weight: 500;
      line-height: 1.2;
      border-radius: var(--elohim-button-radius, 0.375rem);
      border: var(--elohim-button-border, 1px solid transparent);
      cursor: pointer;
      transition:
        background-color 150ms ease,
        border-color 150ms ease,
        color 150ms ease,
        transform 80ms ease;
    }

    button:focus-visible {
      outline: 2px solid var(--tech-glow, #7fcbee);
      outline-offset: 2px;
    }

    button:hover:not([aria-disabled='true']) {
      filter: brightness(1.08);
    }

    button:active:not([aria-disabled='true']) {
      transform: translateY(1px);
    }

    button[aria-disabled='true'] {
      cursor: not-allowed;
      opacity: 0.55;
    }

    :host([variant='primary']) button {
      background: var(--elohim-button-bg, var(--primary, #6b46c1));
      color: var(--elohim-button-fg, #fff);
    }

    :host([variant='secondary']) button {
      background: var(--elohim-button-bg, var(--secondary, #ec4899));
      color: var(--elohim-button-fg, #1a1a1a);
    }

    :host([variant='ghost']) button {
      background: var(--elohim-button-bg, transparent);
      color: var(--elohim-button-fg, var(--text-light, #f3f4f6));
      border: var(--elohim-button-border, 1px solid currentColor);
    }
  `;

  @property({ reflect: true })
  variant: ElohimButtonVariant = 'primary';

  @property({ type: Boolean, reflect: true })
  disabled = false;

  override render() {
    // aria-disabled is always set ('true'|'false'). The disabled-state CSS selector uses
    // [aria-disabled='true'], and the test asserts 'false' on enabled buttons — change both
    // if you switch to absence-means-false.
    return html`
      <button
        part="button"
        type="button"
        ?disabled=${this.disabled}
        aria-disabled=${this.disabled ? 'true' : 'false'}
      >
        <slot></slot>
      </button>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-button': ElohimButton;
  }
}
