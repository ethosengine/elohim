import { CapabilityAwareElement } from 'elohim-core';
import { css, html, LitElement, type PropertyValues } from 'lit';
import { property, state } from 'lit/decorators.js';

import './elohim-imagodei-attestor-row.js';
import './elohim-imagodei-trust-indicator.js';
import type { AttestorRef } from './elohim-imagodei-attestor-row.js';
import type { TrustMode } from './elohim-imagodei-trust-indicator.js';

export type PortalStep = 'resolve' | 'login' | 'consent' | 'callback';

export interface AuthorityResolution {
  trustMode: TrustMode;
  authority: { label: string; id?: string };
  flywheelHint?: boolean;
  attestors?: AttestorRef[];
}

/**
 * <elohim-imagodei-portal-shell> — the wizard's outer wrapper for the peer
 * OAuth portal. Discovers `trustMode` on mount via `authorityEndpoint` (default
 * `/auth/me`); propagates it to slotted children; renders persistent chrome
 * (trust-indicator + attestor-row in header by default; consumer can override
 * via the `header` slot); slots in the active step element via the `primary`
 * slot.
 *
 * NEVER auto-advances steps — consumers set `step` explicitly in response to
 * child events. This keeps the wizard predictable and externally testable.
 *
 * Security framing per spec §0 + §1.1: community-attestation security applies
 * in BOTH trust modes. `trustMode` is OPERATIONAL (which conductor signs for
 * you), not philosophical.
 *
 * @element elohim-imagodei-portal-shell
 *
 * @prop {string} authorityEndpoint - URL to discover trustMode; default `/auth/me`
 * @prop {PortalStep} step - which wizard step is active; default `resolve`
 * @prop {boolean} flywheelHint - propagated to trust-indicator in doorway-host mode
 *
 * @slot header - defaults to trust-indicator + attestor-row; consumer can override
 * @slot primary - the active step element
 * @slot footer - legal / help text
 * @slot error-region - rendered when error content is provided
 * @slot auth-wall - rendered when the portal itself needs auth (deferred)
 *
 * @fires {CustomEvent<AuthorityResolution>} authority-resolved - emitted after /auth/me resolves
 * @fires {CustomEvent<{step: PortalStep}>} step-change - emitted when `step` property changes
 *
 * @cssprop --elohim-portal-bg - Portal background (default: Canvas)
 * @cssprop --elohim-portal-fg - Portal foreground / text color (default: CanvasText)
 * @cssprop --elohim-portal-panel-bg - Panel / card background (default: color-mix Canvas/CanvasText)
 * @cssprop --elohim-portal-grid-gap - Gap between grid rows (default: 1rem)
 * @cssprop --elohim-portal-max-inline-size - Max width of the centered column (default: 480px)
 * @cssprop --elohim-portal-padding - Frame padding (default: 1rem)
 * @cssprop --elohim-portal-panel-radius - Panel border radius (default: 8px)
 * @cssprop --elohim-portal-panel-padding - Panel internal padding (default: 1rem)
 * @cssprop --elohim-portal-footer-size - Footer font size (default: 0.75rem)
 *
 * @csspart frame - The outer grid container
 * @csspart header - The header region
 * @csspart primary-region - The main panel containing the primary slot
 * @csspart error-region - The error region container
 * @csspart footer - The footer region
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
export class ElohimImagodeiPortalShell extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      background: var(--elohim-portal-bg, Canvas);
      color: var(--elohim-portal-fg, CanvasText);
      min-block-size: 100dvh;
    }

    [part='frame'] {
      display: grid;
      grid-template-rows: auto 1fr auto auto;
      gap: var(--elohim-portal-grid-gap, 1rem);
      padding: var(--elohim-portal-padding, 1rem);
      max-inline-size: var(--elohim-portal-max-inline-size, 480px);
      margin-inline: auto;
    }

    [part='header'] {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 0.5rem;
    }

    [part='primary-region'] {
      background: var(
        --elohim-portal-panel-bg,
        color-mix(in srgb, Canvas 96%, CanvasText)
      );
      border-radius: var(--elohim-portal-panel-radius, 8px);
      padding: var(--elohim-portal-panel-padding, 1rem);
    }

    [part='footer'] {
      font-size: var(--elohim-portal-footer-size, 0.75rem);
      opacity: 0.7;
    }

    [part='error-region']:empty {
      display: none;
    }

    @media (forced-colors: active) {
      :host {
        background: Canvas;
        color: CanvasText;
      }

      [part='primary-region'] {
        background: Canvas;
        border: 1px solid CanvasText;
      }
    }
  `;

  /** URL to fetch authority/trustMode from. */
  @property({ attribute: 'authority-endpoint' }) authorityEndpoint = '/auth/me';

  /** The currently active wizard step. Consumers set this; the shell never changes it internally. */
  @property() step: PortalStep = 'resolve';

  /** When true, propagated to trust-indicator to surface the "graduate to your own conductor" hint. */
  @property({ attribute: 'flywheel-hint', type: Boolean }) flywheelHint = false;

  @state() private _trustMode: TrustMode = 'doorway-host';
  @state() private _authorityLabel = '';
  @state() private _attestors: AttestorRef[] = [];

  override connectedCallback(): void {
    super.connectedCallback();
    void this._discoverAuthority();
  }

  protected override updated(changed: PropertyValues): void {
    super.updated?.(changed);
    if (changed.has('step')) {
      this.dispatchEvent(
        new CustomEvent('step-change', {
          detail: { step: this.step },
          bubbles: true,
          composed: true,
        })
      );
    }
    // Always re-propagate context when anything relevant changes.
    this._propagateContextToSlots();
  }

  /**
   * Test seam: directly set the authority resolution without an HTTP call.
   * Calling code (and tests) may invoke this to inject authority state.
   */
  _setAuthority(res: AuthorityResolution): void {
    this._trustMode = res.trustMode;
    this._authorityLabel = res.authority.label;
    this._attestors = res.attestors ?? [];
    if (res.flywheelHint !== undefined) {
      this.flywheelHint = res.flywheelHint;
    }
    this.dispatchEvent(
      new CustomEvent('authority-resolved', {
        detail: res,
        bubbles: true,
        composed: true,
      })
    );
  }

  private async _discoverAuthority(): Promise<void> {
    try {
      const resp = await fetch(this.authorityEndpoint, { credentials: 'include' });
      if (!resp.ok) return;
      const data = (await resp.json()) as Record<string, unknown>;
      const authority = (data['authority'] as Record<string, string> | undefined) ?? {};
      const res: AuthorityResolution = {
        trustMode: (data['trustMode'] as TrustMode | undefined) ?? 'doorway-host',
        authority: {
          label: (authority['label'] as string | undefined) ?? '',
          id: authority['id'] as string | undefined,
        },
        flywheelHint: data['flywheelHint'] as boolean | undefined,
        attestors: data['attestors'] as AttestorRef[] | undefined,
      };
      this._setAuthority(res);
    } catch {
      // Leave defaults; shell still renders with placeholder chrome.
    }
  }

  private _propagateContextToSlots(): void {
    const slot = this.shadowRoot?.querySelector('slot[name="primary"]') as HTMLSlotElement | null;
    if (!slot) return;
    for (const node of slot.assignedElements()) {
      (node as unknown as Record<string, unknown>)['trustMode'] = this._trustMode;
      (node as unknown as Record<string, unknown>)['authority'] = { label: this._authorityLabel };
    }
  }

  private _onSlotChange = (): void => {
    this._propagateContextToSlots();
  };

  override render() {
    return html`
      <div part="frame" class="frame">
        <header part="header">
          <slot name="header">
            <elohim-imagodei-trust-indicator
              trust-mode=${this._trustMode}
              authority-label=${this._authorityLabel}
              ?flywheel-hint=${this.flywheelHint}
            ></elohim-imagodei-trust-indicator>
            <elohim-imagodei-attestor-row
              .attestors=${this._attestors}
            ></elohim-imagodei-attestor-row>
          </slot>
        </header>

        <main part="primary-region">
          <slot
            name="primary"
            @slotchange=${this._onSlotChange}
          ></slot>
        </main>

        <div part="error-region">
          <slot name="error-region"></slot>
        </div>

        <footer part="footer">
          <slot name="footer"></slot>
        </footer>

        <slot name="auth-wall"></slot>
      </div>
    `;
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-imagodei-portal-shell': ElohimImagodeiPortalShell;
  }
}
