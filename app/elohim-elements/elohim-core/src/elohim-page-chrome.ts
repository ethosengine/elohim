import { css, html, LitElement } from 'lit';
import { state } from 'lit/decorators.js';

import { installEprLinkInterceptor } from './navigation/epr-link-interceptor.js';

/**
 * <elohim-page-chrome> — the bundle's root wrapper. Provides a slotted
 * omnibar contract: bundles that have their own toolbar place it in
 * slot="omnibar"; bundles that don't get <elohim-default-omnibar>
 * automatically.
 *
 * @element elohim-page-chrome
 *
 * @slot omnibar - The omnibar surface; defaults to <elohim-default-omnibar>
 * @slot         - Default slot for page content
 *
 * @cssprop --elohim-chrome-omnibar-z - Override sticky z-index of the omnibar host (default: 10)
 *
 * @csspart omnibar-host - The sticky container wrapping the active omnibar
 * @csspart main         - The scrolling content area
 *
 * @capabilityMaxLens detail
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a, unauthorized:n/a
 *
 * Auto-installs the EPR link interceptor (cross-bundle anchor handling) on connect.
 */
export class ElohimPageChrome extends LitElement {
  @state() private hasSlottedOmnibar = false;

  private _uninstallInterceptor?: () => void;

  override connectedCallback(): void {
    super.connectedCallback();
    // Default (base-href heuristic) install — a shell that installs
    // explicitly (Angular elohim-app) wins via the explicit flag.
    this._uninstallInterceptor = installEprLinkInterceptor();
  }

  override disconnectedCallback(): void {
    super.disconnectedCallback();
    this._uninstallInterceptor?.();
    this._uninstallInterceptor = undefined;
  }

  static override readonly styles = css`
    :host {
      display: flex;
      flex-direction: column;
      min-block-size: 100vh;
    }

    :host([hidden]) {
      display: none;
    }

    .omnibar-host {
      position: sticky;
      inset-block-start: 0;
      z-index: var(--elohim-chrome-omnibar-z, 10);
    }

    main {
      flex: 1;
    }

    @media (forced-colors: active) {
      .omnibar-host {
        border-block-end: 1px solid ButtonText;
      }
    }
  `;

  override render() {
    return html`
      <div class="omnibar-host" part="omnibar-host">
        <slot name="omnibar" @slotchange=${this._onOmnibarSlotChange}></slot>
        <elohim-default-omnibar ?hidden=${this.hasSlottedOmnibar}></elohim-default-omnibar>
      </div>
      <main part="main">
        <slot></slot>
      </main>
    `;
  }

  private _onOmnibarSlotChange(e: Event) {
    const slot = e.target as HTMLSlotElement;
    const assigned = slot
      .assignedNodes({ flatten: true })
      .filter(n => n.nodeType === Node.ELEMENT_NODE || (n.textContent?.trim().length ?? 0) > 0);
    this.hasSlottedOmnibar = assigned.length > 0;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-page-chrome': ElohimPageChrome;
  }
}
