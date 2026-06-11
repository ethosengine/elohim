import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement } from 'lit';
import { property, state } from 'lit/decorators.js';

/**
 * Discriminated-union outcome for the injected resolver callback.
 * Distinct from the `ResolveOutcome` in `federated-identifier.ts` (which
 * carries a `DoorwayDescriptor` object). This type is the element's own
 * event-surface contract — callers receive a flat `doorwayUrl` string.
 */
export interface FederatedResolveOutcome {
  ok: boolean;
  /** Canonical doorway URL on success. */
  doorwayUrl?: string;
  /** Human-readable failure reason on failure (surfaced in the UI). */
  reason?: string;
}

export type ResolveIdentifierFn = (identifier: string) => Promise<FederatedResolveOutcome>;

/**
 * <elohim-imagodei-federated-resolver> — input element for federated
 * identifiers (e.g. `matthew@alpha.elohim.host`). Calls the injected
 * `resolveIdentifier` callback to discover the doorway endpoint.
 *
 * Emits `resolved` with `{ identifier, doorwayUrl }` on success, or
 * `resolve-error` with `{ identifier, reason }` on failure.
 *
 * Persists the last-used identifier to `localStorage` under `rememberKey`
 * (default `elohim_auth_identifier`, matching the Angular `AUTH_IDENTIFIER_KEY`
 * constant) and pre-fills the input on mount.
 *
 * @element elohim-imagodei-federated-resolver
 *
 * @prop {ResolveIdentifierFn} resolveIdentifier - injected async resolver callback;
 *   defaults to a no-op that returns `{ ok: false, reason: 'no-resolver' }`
 * @prop {string} placeholder - input placeholder text
 * @prop {string} rememberKey - localStorage key for persistence (default: `elohim_auth_identifier`)
 *
 * @slot help-text - optional hint text rendered below the input
 *
 * @fires {CustomEvent<{identifier: string, doorwayUrl: string}>} resolved
 * @fires {CustomEvent<{identifier: string, reason: string}>} resolve-error
 *
 * @cssprop --elohim-resolver-bg - Form background (default: transparent)
 * @cssprop --elohim-resolver-fg - Form foreground / text color (default: inherit)
 * @cssprop --elohim-input-border - Input border (default: 30% currentColor mix)
 * @cssprop --elohim-input-focus-ring - Input focus ring color (default: currentColor)
 * @cssprop --elohim-input-error-fg - Error message text color (default: 70% currentColor/red mix)
 * @cssprop --elohim-resolver-gap - Gap between grid rows (default: 0.5rem)
 * @cssprop --elohim-resolver-label-size - Label font size (default: 0.875rem)
 *
 * @csspart form - The outer form container
 * @csspart label - The input label
 * @csspart input - The text input
 * @csspart help - The help-text slot wrapper
 * @csspart error - The error message container (present only in error state)
 * @csspart submit - The submit button
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual
 * @capabilityRequiredStandings any
 * @capabilityContentCertainty observed
 * @capabilityStates empty:designed, loading:designed, error:designed, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimImagodeiFederatedResolver extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      background: var(--elohim-resolver-bg, transparent);
      color: var(--elohim-resolver-fg, inherit);
      font: inherit;
    }

    form {
      display: grid;
      gap: var(--elohim-resolver-gap, 0.5rem);
    }

    label {
      display: block;
      font-size: var(--elohim-resolver-label-size, 0.875rem);
    }

    input {
      inline-size: 100%;
      padding-block: 0.5rem;
      padding-inline: 0.75rem;
      border: 1px solid
        var(--elohim-input-border, color-mix(in srgb, currentColor 30%, transparent));
      border-radius: 6px;
      background: transparent;
      color: inherit;
      font: inherit;
      box-sizing: border-box;
    }

    input:focus-visible {
      outline: 2px solid var(--elohim-input-focus-ring, currentColor);
      outline-offset: 2px;
    }

    [part='error'] {
      color: var(--elohim-input-error-fg, color-mix(in srgb, currentColor 70%, red));
      font-size: 0.875rem;
    }

    [part='submit'] {
      padding-block: 0.5rem;
      padding-inline: 1rem;
      border: 1px solid currentColor;
      border-radius: 6px;
      background: transparent;
      color: inherit;
      font: inherit;
      cursor: pointer;
    }

    [part='submit'][disabled] {
      opacity: 0.6;
      cursor: default;
    }

    @media (forced-colors: active) {
      input {
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

      [part='error'] {
        color: CanvasText;
      }
    }
  `;

  /**
   * Injected async callback that resolves a federated identifier to a doorway
   * URL. Consumers MUST provide a real implementation — the default returns
   * `{ ok: false, reason: 'no-resolver' }` so the element is usable in tests
   * and stories without a real resolver wired up.
   */
  @property({ attribute: false })
  resolveIdentifier: ResolveIdentifierFn = async () => ({ ok: false, reason: 'no-resolver' });

  /** Placeholder text for the identifier input. */
  @property() placeholder = 'you@your-doorway.host';

  /**
   * localStorage key used to persist the last-used identifier.
   * Defaults to `elohim_auth_identifier` — matches the Angular
   * `AUTH_IDENTIFIER_KEY` constant for backward compatibility.
   */
  @property({ attribute: 'remember-key' }) rememberKey = 'elohim_auth_identifier';

  @state() private _value = '';
  @state() private _error: string | null = null;
  @state() private _busy = false;

  override connectedCallback(): void {
    super.connectedCallback();
    try {
      const remembered = localStorage.getItem(this.rememberKey);
      if (remembered) this._value = remembered;
    } catch {
      // localStorage unavailable (private mode, SSR, etc.) — degrade silently.
    }
  }

  override render() {
    return html`
      <form part="form" @submit=${this._onSubmit}>
        <label part="label" for="federated-input">Sign in as</label>
        <input
          part="input"
          id="federated-input"
          type="text"
          autocomplete="username"
          inputmode="email"
          placeholder=${this.placeholder}
          .value=${this._value}
          @input=${(e: Event) => {
            this._value = (e.target as HTMLInputElement).value;
          }}
          ?disabled=${this._busy}
          aria-invalid=${this._error ? 'true' : 'false'}
          aria-describedby=${this._error ? 'federated-error' : ''}
        />
        <div part="help">
          <slot name="help-text"></slot>
        </div>
        ${this._error
          ? html`
              <div part="error" id="federated-error" role="alert">${this._error}</div>
            `
          : ''}
        <button type="submit" part="submit" ?disabled=${this._busy || !this._value.trim()}>
          ${this._busy ? 'Resolving…' : 'Continue'}
        </button>
      </form>
    `;
  }

  private async _onSubmit(e: Event): Promise<void> {
    e.preventDefault();
    if (this._busy) return;
    this._error = null;
    this._busy = true;
    const identifier = this._value.trim();
    try {
      const outcome = await this.resolveIdentifier(identifier);
      if (outcome.ok && outcome.doorwayUrl) {
        try {
          localStorage.setItem(this.rememberKey, identifier);
        } catch {
          // Persist failure is non-fatal.
        }
        this.dispatchEvent(
          new CustomEvent('resolved', {
            detail: { identifier, doorwayUrl: outcome.doorwayUrl },
            bubbles: true,
            composed: true,
          })
        );
      } else {
        this._error = outcome.reason ?? 'resolution failed';
        this.dispatchEvent(
          new CustomEvent('resolve-error', {
            detail: { identifier, reason: this._error },
            bubbles: true,
            composed: true,
          })
        );
      }
    } finally {
      this._busy = false;
    }
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-imagodei-federated-resolver': ElohimImagodeiFederatedResolver;
  }
}
