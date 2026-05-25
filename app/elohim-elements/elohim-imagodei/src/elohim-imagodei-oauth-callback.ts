import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement, type PropertyValues } from 'lit';
import { property, state } from 'lit/decorators.js';

/**
 * The resolved shape returned by a successful code exchange.
 * `session` is intentionally typed as `unknown` — the consuming app owns
 * the session shape; this primitive is transport-only.
 */
export interface ExchangeOutcome {
  session: unknown;
}

/**
 * Injected callback that performs the actual OAuth token exchange.
 * Receives the authorization `code` and `state` from the redirect URL.
 * Must resolve with an `ExchangeOutcome` on success; reject with an
 * `Error` (or anything coercible via `String()`) on failure.
 */
export type ExchangeCodeFn = (code: string, state: string) => Promise<ExchangeOutcome>;

/**
 * <elohim-imagodei-oauth-callback> — OAuth code-exchange callback handler.
 *
 * Renders a skeleton while the exchange is in progress, then emits
 * `success` or `error`. The exchange runs **exactly once** when both
 * `code` and `state` are set — re-assigning the same values does not
 * trigger a re-exchange. Auto-retry is not performed; the `recovery-cta`
 * slot lets consumers provide their own retry or recovery links.
 *
 * ## State machine
 * - `idle` — code or state is absent; waiting for OAuth redirect params.
 * - `exchanging` — both params present; exchange in flight; skeleton shown.
 * - `done` — exchange resolved; `success` event emitted.
 * - `error` — exchange rejected; `error` event emitted; error/recovery slots visible.
 *
 * @element elohim-imagodei-oauth-callback
 *
 * @prop {string} code - OAuth authorization code from the redirect URL
 * @prop {string} state - OAuth state parameter (CSRF defense, RFC-6749 §10.12)
 * @prop {string} providerLabel - Human-readable provider name for status messages
 * @prop {ExchangeCodeFn} exchangeCode - Injected code-exchange callback
 *
 * @slot error-detail - Full error envelope for error state (shown below reason)
 * @slot recovery-cta - Try-again / recovery links shown in error state
 *
 * @fires {CustomEvent<{session: unknown}>} success - Exchange completed; detail contains the session
 * @fires {CustomEvent<{reason: string, recoverable: boolean}>} error - Exchange failed; reason is the error message
 *
 * @cssprop --elohim-callback-padding - Container padding (default: 1rem)
 * @cssprop --elohim-callback-skeleton-bg - Skeleton bone background (default: 14% currentColor mix)
 * @cssprop --elohim-callback-skeleton-radius - Skeleton bone border-radius (default: 4px)
 * @cssprop --elohim-callback-error-color-mix - Error text color mix fraction (default: 70%)
 * @cssprop --elohim-callback-gap - Grid gap between skeleton bones (default: 0.5rem)
 *
 * @csspart skeleton - Animated skeleton container (visible during exchange)
 * @csspart exchanging-message - Status message shown below the skeleton during exchange
 * @csspart idle - Container shown when code or state is absent
 * @csspart done - Container shown after successful exchange
 * @csspart error - Alert container shown after a failed exchange
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings any
 * @capabilityContentCertainty observed
 * @capabilityStates empty:n/a, loading:designed, error:designed, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimImagodeiOauthCallback extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      padding: var(--elohim-callback-padding, 1rem);
      font: inherit;
    }

    [part='skeleton'] {
      display: grid;
      gap: var(--elohim-callback-gap, 0.5rem);
    }

    [part='skeleton'] > div {
      block-size: 1rem;
      background: var(
        --elohim-callback-skeleton-bg,
        color-mix(in srgb, currentColor 14%, transparent)
      );
      border-radius: var(--elohim-callback-skeleton-radius, 4px);
    }

    @media (prefers-reduced-motion: no-preference) and (update: fast) {
      [part='skeleton'] {
        animation: elohim-callback-pulse 1.2s ease-in-out infinite;
      }
    }

    @keyframes elohim-callback-pulse {
      0%, 100% { opacity: 0.6; }
      50% { opacity: 1; }
    }

    [part='error'] {
      color: color-mix(
        in srgb,
        currentColor var(--elohim-callback-error-color-mix, 70%),
        red
      );
    }

    @media (forced-colors: active) {
      [part='skeleton'] > div {
        background: GrayText;
      }

      [part='error'] {
        color: CanvasText;
        border: 1px solid CanvasText;
        padding-block: 0.5rem;
        padding-inline: 0.75rem;
      }
    }
  `;

  /** OAuth authorization code received from the redirect URL. */
  @property() code = '';

  /** OAuth state parameter — CSRF defense per RFC-6749 §10.12. */
  @property() state = '';

  /** Human-readable provider name used in status messages. */
  @property({ attribute: 'provider-label' }) providerLabel = '';

  /**
   * Injected code-exchange callback. Must resolve with `ExchangeOutcome`
   * on success or reject on failure. Never called without both code and state.
   */
  @property({ attribute: false })
  exchangeCode: ExchangeCodeFn = async () => ({ session: null });

  @state() private _status: 'idle' | 'exchanging' | 'done' | 'error' = 'idle';
  @state() private _errorReason: string | null = null;

  /**
   * Guards against re-triggering the exchange if code/state are re-set
   * to the same values after the first exchange has already started.
   */
  private _started = false;

  protected override updated(_changed: PropertyValues): void {
    if (!this._started && this.code && this.state) {
      this._started = true;
      void this._runExchange();
    }
  }

  private async _runExchange(): Promise<void> {
    this._status = 'exchanging';
    try {
      const outcome = await this.exchangeCode(this.code, this.state);
      this._status = 'done';
      // Dispatch in a macrotask so that any event listeners registered
      // synchronously after the `fixture()` / `await` that initiated the
      // element are in place before the event fires.
      setTimeout(() => {
        this.dispatchEvent(
          new CustomEvent('success', {
            detail: { session: outcome.session },
            bubbles: true,
            composed: true,
          })
        );
      }, 0);
    } catch (e: unknown) {
      this._status = 'error';
      this._errorReason = e instanceof Error ? e.message : String(e);
      // `bubbles: false, composed: false` prevents the custom `error` event
      // from propagating to `window` as an unhandled page error. Callers
      // listen on the element directly.
      // Dispatched in a macrotask for the same listener-readiness reason.
      setTimeout(() => {
        this.dispatchEvent(
          new CustomEvent('error', {
            detail: { reason: this._errorReason, recoverable: true },
            bubbles: false,
            composed: false,
          })
        );
      }, 0);
    }
  }

  override render() {
    if (this._status === 'idle') {
      return html`
        <div part="idle">
          Waiting for OAuth response${this.providerLabel ? html` from ${this.providerLabel}` : ''}…
        </div>
      `;
    }

    if (this._status === 'exchanging') {
      return html`
        <div part="skeleton" aria-live="polite" aria-busy="true">
          <div></div>
          <div></div>
          <div></div>
        </div>
        <p part="exchanging-message">
          Finishing sign-in${this.providerLabel ? html` with ${this.providerLabel}` : ''}…
        </p>
      `;
    }

    if (this._status === 'error') {
      return html`
        <div part="error" role="alert">
          Sign-in failed${this.providerLabel ? html` (${this.providerLabel})` : ''}: ${this._errorReason}
        </div>
        <slot name="error-detail"></slot>
        <slot name="recovery-cta"></slot>
      `;
    }

    // done
    return html`<p part="done">Signed in. Redirecting…</p>`;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-imagodei-oauth-callback': ElohimImagodeiOauthCallback;
  }
}
