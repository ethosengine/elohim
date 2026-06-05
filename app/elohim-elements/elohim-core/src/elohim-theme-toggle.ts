import { msg, updateWhenLocaleChanges } from '@lit/localize';
import { css, html, LitElement, nothing } from 'lit';
import { state } from 'lit/decorators.js';

import { getThemeStore, type ElohimTheme } from './theme/theme-store.js';

/**
 * <elohim-theme-toggle> — cycles the device-scoped theme preference
 * (device → light → dark) through the shared ThemeStore. Pure
 * sense-and-respond: renders current state, forwards the user's intent to
 * the store; the theming itself happens via body[data-theme] + CSS cascade.
 *
 * @element elohim-theme-toggle
 *
 * @fires theme-changed - detail: { theme } after the user cycles
 *
 * @cssprop --elohim-theme-toggle-bg       - button background (default: transparent)
 * @cssprop --elohim-theme-toggle-fg       - button foreground (default: inherit)
 * @cssprop --elohim-theme-toggle-border   - button border color (default: transparent)
 * @cssprop --elohim-theme-toggle-badge-bg - auto badge background (default: ButtonFace)
 * @cssprop --elohim-theme-toggle-badge-fg - auto badge foreground (default: ButtonText)
 *
 * @csspart button         - the toggle button
 * @csspart icon           - the sun/moon glyph
 * @csspart auto-indicator - the "A" badge shown in device (auto) mode
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a, unauthorized:n/a
 */
export class ElohimThemeToggle extends LitElement {
  private _store = getThemeStore();

  @state() private _theme: ElohimTheme = this._store.theme;

  private _unsub?: () => void;

  constructor() {
    super();
    updateWhenLocaleChanges(this);
  }

  static override readonly styles = css`
    :host {
      display: inline-flex;
    }

    :host([hidden]) {
      display: none;
    }

    button {
      position: relative;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 0.15rem;
      min-inline-size: 44px;
      min-block-size: 44px;
      padding: 0.25rem;
      background: var(--elohim-theme-toggle-bg, transparent);
      color: var(--elohim-theme-toggle-fg, inherit);
      border: 1px solid var(--elohim-theme-toggle-border, transparent);
      border-radius: 999px;
      font: inherit;
      cursor: pointer;
    }

    button:focus-visible {
      outline: 2px solid Highlight;
      outline-offset: 2px;
    }

    [part='auto-indicator'] {
      font-size: 0.6em;
      font-weight: 700;
      padding-block: 0;
      padding-inline: 0.3em;
      border-radius: 999px;
      background: var(--elohim-theme-toggle-badge-bg, ButtonFace);
      color: var(--elohim-theme-toggle-badge-fg, ButtonText);
    }

    @media (forced-colors: active) {
      button {
        border-color: ButtonText;
        color: ButtonText;
      }
    }
  `;

  override connectedCallback(): void {
    super.connectedCallback();
    this._unsub = this._store.subscribe((t) => {
      this._theme = t;
    });
    this._theme = this._store.theme;
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this._unsub?.();
  }

  private _label(): string {
    const names: Record<ElohimTheme, string> = {
      device: msg('Theme: follow device — click for light'),
      light: msg('Theme: light — click for dark'),
      dark: msg('Theme: dark — click to follow device'),
    };
    return names[this._theme];
  }

  private _cycle(): void {
    this._store.cycle();
    this.dispatchEvent(
      new CustomEvent('theme-changed', {
        detail: { theme: this._store.theme },
        bubbles: true,
        composed: true,
      }),
    );
  }

  override render() {
    const label = this._label();
    return html`
      <button part="button" type="button" aria-label=${label} title=${label} @click=${this._cycle}>
        <span part="icon" aria-hidden="true">${this._store.effectiveTheme === 'dark' ? '🌙' : '☀️'}</span>
        ${this._theme === 'device'
          ? html`<span part="auto-indicator" aria-hidden="true">A</span>`
          : nothing}
      </button>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-theme-toggle': ElohimThemeToggle;
  }
}
