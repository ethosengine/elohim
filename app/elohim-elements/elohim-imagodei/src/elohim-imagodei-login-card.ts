import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property, state } from 'lit/decorators.js';

import type { TrustMode } from './elohim-imagodei-trust-indicator.js';

export type { TrustMode };

export interface OAuthProviderRef {
  id: string;
  displayName: string;
  iconUrl?: string;
}

/**
 * <elohim-imagodei-login-card> — credentials form for the peer OAuth portal.
 * Renders a password form and/or OAuth provider buttons in a single visual block.
 * Adapts copy based on `trustMode` (set by portal-shell via DOM property assignment
 * on slotchange). The `unlock-prompt` slot supports Tauri's local-key-unlock flow.
 *
 * Security framing per spec §0 + §1.1: BOTH trust modes carry community-attested
 * security. The copy difference ("Sign in" vs "Unlock your conductor") is
 * OPERATIONAL — what's happening on which machine — not a philosophical claim
 * about who holds keys. Avoid crypto-bro/sovereign-key framing.
 *
 * Shell never auto-advances — the consumer calls `setError(message)` to surface
 * async errors, then decides next step. See spec §3.4.
 *
 * @element elohim-imagodei-login-card
 *
 * @prop {OAuthProviderRef[]} oauthProviders - list of OAuth providers to render as buttons
 * @prop {boolean} allowPassword - whether to render the password form; default true
 * @prop {boolean} remember - current state of the "remember me" checkbox
 * @prop {string} rememberedIdentifier - injected by parent for the submit event detail
 * @prop {TrustMode} trustMode - set by portal-shell via property assignment; adapts copy
 *
 * @slot unlock-prompt - Tauri local-key-unlock UI; rendered above the password form
 *
 * @fires {CustomEvent<{identifier: string, password: string, remember: boolean}>} password-submit
 * @fires {CustomEvent<{providerId: string}>} oauth-start
 * @fires {CustomEvent} cancel
 *
 * @cssprop --elohim-login-bg - Card background (default: transparent)
 * @cssprop --elohim-login-fg - Card foreground text color (default: inherit)
 * @cssprop --elohim-login-input-bg - Password input background (default: transparent)
 * @cssprop --elohim-login-input-border - Input border (default: 30% currentColor mix)
 * @cssprop --elohim-login-button-bg - Submit button background (default: transparent)
 * @cssprop --elohim-login-button-fg - Submit button foreground (default: inherit)
 * @cssprop --elohim-login-oauth-border - OAuth button border (default: 30% currentColor mix)
 * @cssprop --elohim-login-gap - Grid gap between rows (default: 0.75rem)
 * @cssprop --elohim-login-radius - Border radius for inputs and buttons (default: 6px)
 * @cssprop --elohim-login-error-fg - Error message text color (default: 70% currentColor/red mix)
 *
 * @csspart form - The password form element
 * @csspart oauth-row - The container wrapping all OAuth buttons
 * @csspart oauth-button - Individual OAuth provider button (carries data-provider attribute)
 * @csspart error - The error message container; present only when an error is set
 * @csspart submit - The password form submit button
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings any
 * @capabilityContentCertainty observed
 * @capabilityStates empty:designed, loading:designed, error:designed, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimImagodeiLoginCard extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      background: var(--elohim-login-bg, transparent);
      color: var(--elohim-login-fg, inherit);
      font: inherit;
    }

    .card-grid {
      display: grid;
      gap: var(--elohim-login-gap, 0.75rem);
    }

    form {
      display: grid;
      gap: var(--elohim-login-gap, 0.75rem);
    }

    label {
      display: block;
      font-size: 0.875rem;
    }

    input[type='password'] {
      inline-size: 100%;
      padding-block: 0.5rem;
      padding-inline: 0.75rem;
      border: 1px solid
        var(--elohim-login-input-border, color-mix(in srgb, currentColor 30%, transparent));
      border-radius: var(--elohim-login-radius, 6px);
      background: var(--elohim-login-input-bg, transparent);
      color: inherit;
      font: inherit;
      box-sizing: border-box;
    }

    input[type='password']:focus-visible {
      outline: 2px solid currentColor;
      outline-offset: 2px;
    }

    .remember {
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
      font-size: 0.875rem;
    }

    [part='submit'] {
      padding-block: 0.5rem;
      padding-inline: 1rem;
      border: 1px solid currentColor;
      border-radius: var(--elohim-login-radius, 6px);
      background: var(--elohim-login-button-bg, transparent);
      color: var(--elohim-login-button-fg, inherit);
      font: inherit;
      cursor: pointer;
    }

    [part='submit'][disabled] {
      opacity: 0.6;
      cursor: default;
    }

    [part='oauth-row'] {
      display: grid;
      gap: 0.5rem;
    }

    [part='oauth-button'] {
      padding-block: 0.5rem;
      padding-inline: 0.75rem;
      border: 1px solid
        var(--elohim-login-oauth-border, color-mix(in srgb, currentColor 30%, transparent));
      border-radius: var(--elohim-login-radius, 6px);
      background: transparent;
      color: inherit;
      font: inherit;
      cursor: pointer;
      text-align: start;
    }

    [part='oauth-button']:hover,
    [part='oauth-button']:focus-visible {
      background: color-mix(in srgb, currentColor 6%, transparent);
    }

    [part='oauth-button']:focus-visible {
      outline: 2px solid currentColor;
      outline-offset: 2px;
    }

    .divider {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      opacity: 0.6;
      font-size: 0.75rem;
    }

    .divider::before,
    .divider::after {
      content: '';
      flex: 1;
      border-block-start: 1px solid currentColor;
      opacity: 0.3;
    }

    [part='error'] {
      color: var(--elohim-login-error-fg, color-mix(in srgb, currentColor 70%, red));
      font-size: 0.875rem;
    }

    @media (forced-colors: active) {
      input[type='password'] {
        border-color: CanvasText;
        background: Canvas;
        color: CanvasText;
      }

      [part='submit'] {
        border-color: ButtonText;
        background: ButtonFace;
        color: ButtonText;
      }

      [part='submit'][disabled] {
        border-color: GrayText;
        color: GrayText;
      }

      [part='oauth-button'] {
        border-color: CanvasText;
        background: Canvas;
        color: CanvasText;
      }

      [part='oauth-button']:hover,
      [part='oauth-button']:focus-visible {
        background: Highlight;
        color: HighlightText;
        outline: 2px solid ButtonText;
      }

      [part='error'] {
        color: CanvasText;
      }
    }
  `;

  /** OAuth providers to render as sign-in buttons. */
  @property({ attribute: false }) oauthProviders: OAuthProviderRef[] = [];

  /** Whether to render the password form. Default: true. */
  @property({ type: Boolean, attribute: 'allow-password' }) allowPassword = true;

  /** Current state of the "remember me" checkbox. */
  @property({ type: Boolean }) remember = false;

  /**
   * The federated identifier (e.g. `matthew@alpha.elohim.host`) injected by the
   * parent portal-shell. Included in the `password-submit` event detail so the
   * consumer has the full context without re-querying state.
   */
  @property({ attribute: 'remembered-identifier' }) rememberedIdentifier = '';

  /**
   * Operational trust mode — set by portal-shell via DOM property assignment on
   * slotchange. Controls copy: "Sign in" (doorway-host) vs "Unlock your conductor"
   * (peer-conductor). Default: doorway-host.
   */
  @property() trustMode: TrustMode = 'doorway-host';

  @state() private _password = '';
  @state() private _busy = false;
  @state() private _error: string | null = null;

  override render() {
    const isPeer = this.trustMode === 'peer-conductor';
    const passwordLabel = isPeer ? 'Unlock your conductor' : 'Sign in with password';
    const submitLabel = this._busy ? 'Signing in…' : isPeer ? 'Unlock' : 'Sign in';

    return html`
      <div class="card-grid">
        <slot name="unlock-prompt"></slot>

        ${this.oauthProviders.length > 0
          ? html`
              <div class="oauth-row" part="oauth-row">
                ${this.oauthProviders.map(
                  p => html`
                    <button
                      type="button"
                      part="oauth-button"
                      data-provider=${p.id}
                      @click=${() => this._onOAuth(p.id)}
                    >
                      Continue with ${p.displayName}
                    </button>
                  `
                )}
              </div>
              ${this.allowPassword
                ? html`
                    <div class="divider">or</div>
                  `
                : ''}
            `
          : ''}
        ${this.allowPassword
          ? html`
              <form part="form" @submit=${this._onSubmit}>
                <label for="pw">${passwordLabel}</label>
                <input
                  id="pw"
                  type="password"
                  autocomplete="current-password"
                  .value=${this._password}
                  @input=${(e: Event) => {
                    this._password = (e.target as HTMLInputElement).value;
                  }}
                  ?disabled=${this._busy}
                  aria-invalid=${this._error ? 'true' : 'false'}
                  aria-describedby=${this._error ? 'login-error' : ''}
                />
                <label class="remember">
                  <input
                    type="checkbox"
                    ?checked=${this.remember}
                    @change=${(e: Event) => {
                      this.remember = (e.target as HTMLInputElement).checked;
                    }}
                  />
                  Remember me on this device
                </label>
                ${this._error
                  ? html`
                      <div part="error" id="login-error" role="alert" aria-live="polite">
                        ${this._error}
                      </div>
                    `
                  : ''}
                <button type="submit" part="submit" ?disabled=${this._busy || !this._password}>
                  ${submitLabel}
                </button>
              </form>
            `
          : ''}
      </div>
    `;
  }

  private _onSubmit = (e: Event): void => {
    e.preventDefault();
    if (this._busy || !this._password) return;
    this._busy = true;
    this._error = null;
    this.dispatchEvent(
      new CustomEvent('password-submit', {
        detail: {
          identifier: this.rememberedIdentifier,
          password: this._password,
          remember: this.remember,
        },
        bubbles: true,
        composed: true,
      })
    );
    // Consumer drives the rest. Release busy after a tick so the button
    // feels responsive and the consumer can call setError() immediately.
    setTimeout(() => {
      this._busy = false;
    }, 0);
  };

  private _onOAuth(providerId: string): void {
    this.dispatchEvent(
      new CustomEvent('oauth-start', {
        detail: { providerId },
        bubbles: true,
        composed: true,
      })
    );
  }

  /**
   * Public API for the consumer to surface errors after an async submit.
   * Calling `setError(null)` clears the error region.
   * Setting an error also releases the busy state so the form is re-enabled.
   */
  setError(message: string | null): void {
    this._error = message;
    this._busy = false;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-imagodei-login-card': ElohimImagodeiLoginCard;
  }
}
