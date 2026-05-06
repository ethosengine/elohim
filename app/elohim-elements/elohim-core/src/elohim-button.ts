import { css, html, LitElement, type PropertyValues } from 'lit';
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
 * @event {MouseEvent} click - Fired on activation (mouse or keyboard via native button)
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
      background: var(--elohim-button-bg, var(--primary, #6b46c1));
      color: var(--elohim-button-fg, var(--text-light, #f3f4f6));
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
      color: var(--elohim-button-fg, var(--text-light, #f3f4f6));
    }

    :host([variant='secondary']) button {
      background: var(--elohim-button-bg, var(--secondary, #ec4899));
      color: var(--elohim-button-fg, var(--text-light, #f3f4f6));
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

  protected override updated(changed: PropertyValues<this>) {
    super.updated(changed);
    // No-op hook for future variant-derived state (e.g., loading, busy).
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-button': ElohimButton;
  }
}
