import { msg, updateWhenLocaleChanges } from '@lit/localize';
import { css, html, LitElement } from 'lit';
import { state } from 'lit/decorators.js';

import {
  LOCALE_LABELS,
  SUPPORTED_LOCALES,
  getLocaleStore,
  type ElohimLocale,
} from './localize/locale-store.js';

/**
 * <elohim-lang-picker> — per-site language override the web platform doesn't
 * provide natively. Drives the shared LocaleStore (lit-localize setLocale +
 * document lang/dir + localStorage persistence). Native <select> for
 * built-in keyboard + screen-reader semantics.
 *
 * @element elohim-lang-picker
 *
 * @fires locale-changed - detail: { locale } after the user selects
 *
 * @cssprop --elohim-lang-picker-bg     - select background (default: transparent)
 * @cssprop --elohim-lang-picker-fg     - select foreground (default: inherit)
 * @cssprop --elohim-lang-picker-border - select border color
 *
 * @csspart label  - the (visually hidden) label wrapper
 * @csspart select - the locale select
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
export class ElohimLangPicker extends LitElement {
  private readonly _store = getLocaleStore();

  @state() private _locale: ElohimLocale = this._store.locale;

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

    select {
      font: inherit;
      color: var(--elohim-lang-picker-fg, inherit);
      background: var(--elohim-lang-picker-bg, transparent);
      border: 1px solid
        var(--elohim-lang-picker-border, color-mix(in oklch, currentColor 25%, transparent));
      border-radius: 0.35rem;
      padding-block: 0.15rem;
      padding-inline: 0.35rem;
      min-block-size: 2rem;
      cursor: pointer;
    }

    select:focus-visible {
      outline: 2px solid Highlight;
      outline-offset: 2px;
    }

    .visually-hidden {
      position: absolute;
      inline-size: 1px;
      block-size: 1px;
      overflow: hidden;
      clip-path: inset(50%);
      white-space: nowrap;
    }

    @media (forced-colors: active) {
      select {
        border-color: ButtonText;
        color: ButtonText;
        background: ButtonFace;
      }
    }
  `;

  override connectedCallback(): void {
    super.connectedCallback();
    this._unsub = this._store.subscribe(l => {
      this._locale = l;
    });
    this._locale = this._store.locale;
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this._unsub?.();
  }

  private _onChange(e: Event): void {
    const value = (e.target as HTMLSelectElement).value as ElohimLocale;
    this._store.set(value);
    this.dispatchEvent(
      new CustomEvent('locale-changed', {
        detail: { locale: value },
        bubbles: true,
        composed: true,
      })
    );
  }

  override render() {
    return html`
      <label part="label">
        <span class="visually-hidden">${msg('Language')}</span>
        <select part="select" @change=${this._onChange} .value=${this._locale}>
          ${SUPPORTED_LOCALES.map(
            l => html`
              <option value=${l} ?selected=${l === this._locale} lang=${l}>
                ${LOCALE_LABELS[l]}
              </option>
            `
          )}
        </select>
      </label>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-lang-picker': ElohimLangPicker;
  }
}
