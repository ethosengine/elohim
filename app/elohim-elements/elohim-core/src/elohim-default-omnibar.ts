import { css, html, LitElement } from 'lit';
import { state } from 'lit/decorators.js';

import { Session, type CurrentUserView } from './session/session.js';

/**
 * <elohim-default-omnibar> — the fallback omnibar used when a bundle doesn't
 * BYO its own toolbar. Minimal MVP shape: brand mark + auth indicator.
 *
 * The session primitive doesn't poll cookies. On `visibilitychange` the
 * element calls session.refreshFromCookies() so cross-tab auth changes
 * are picked up opportunistically on tab re-focus. Callers may also call
 * session.refreshFromCookies() directly after any auth-related event.
 *
 * @element elohim-default-omnibar
 *
 * @cssprop --elohim-omnibar-bg     - Override background (default: blend of Canvas + currentColor)
 * @cssprop --elohim-omnibar-border - Override bottom border color
 *
 * @csspart brand     - The brand mark span
 * @csspart user      - The user area (name or sign-in link)
 * @csspart user-name - The authenticated user's humanId (present only when signed in)
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a, unauthorized:designed
 */
export class ElohimDefaultOmnibar extends LitElement {
  private _session = new Session();

  @state() private _user: CurrentUserView | null = this._session.currentUser;

  private _unsub?: () => void;

  private _visibilityHandler = () => {
    if (!document.hidden) this._session.refreshFromCookies();
  };

  static override readonly styles = css`
    :host {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding-block: 0.5rem;
      padding-inline: 1rem;
      background: var(--elohim-omnibar-bg, color-mix(in oklch, Canvas 92%, currentColor));
      border-block-end: 1px solid var(--elohim-omnibar-border, color-mix(in oklch, currentColor 12%, transparent));
    }

    :host([hidden]) {
      display: none;
    }

    .brand {
      font-weight: 600;
    }

    .user {
      font-size: 0.875rem;
      opacity: 0.8;
    }

    a {
      color: inherit;
    }

    @media (forced-colors: active) {
      :host {
        background: Canvas;
        border-block-end-color: ButtonText;
      }

      .user {
        color: CanvasText;
        opacity: 1;
      }

      a {
        color: LinkText;
      }
    }
  `;

  override connectedCallback(): void {
    super.connectedCallback();
    this._unsub = this._session.subscribe((u) => {
      this._user = u;
    });
    document.addEventListener('visibilitychange', this._visibilityHandler);
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this._unsub?.();
    document.removeEventListener('visibilitychange', this._visibilityHandler);
  }

  override render() {
    return html`
      <span class="brand" part="brand">elohim.host</span>
      <span class="user" part="user">
        ${this._user
          ? html`<span part="user-name">${this._user.humanId}</span>`
          : html`<a href="/auth/signin">sign in</a>`}
      </span>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-default-omnibar': ElohimDefaultOmnibar;
  }
}
