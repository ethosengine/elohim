import { css, html, LitElement, nothing } from 'lit';
import { property, state } from 'lit/decorators.js';

import { CapabilityAwareElement } from './capability/index.js';

// ── Types ──────────────────────────────────────────────────────────────────────

export type ElohimContextApp = 'lamad' | 'community' | 'shefa' | 'doorway' | 'avodah' | 'map';

export interface ElohimContextAppConfig {
  id: ElohimContextApp;
  name: string;
  icon: string;
  route: string;
  tagline: string;
  available: boolean;
}

export interface ElohimNavigatorSession {
  displayName?: string;
  initials?: string;
  /** Pillar-level stats summary, e.g. "12 explored · 3 paths" */
  statsSummary?: string;
}

export interface ElohimNavigatorBannerItem {
  id: string;
  message: string;
  severity?: 'info' | 'warning' | 'error';
}

const DEFAULT_CONTEXT_APPS: ElohimContextAppConfig[] = [
  { id: 'lamad', name: 'Lamad', icon: '📚', route: '/lamad', tagline: 'Learning & Content', available: true },
  { id: 'community', name: 'Qahal', icon: '👥', route: '/community', tagline: 'Community & Governance', available: true },
  { id: 'shefa', name: 'Shefa', icon: '✨', route: '/shefa', tagline: 'Economics of Flourishing', available: true },
  { id: 'avodah', name: 'Avodah', icon: '🔨', route: '/avodah', tagline: 'Work & Stewardship', available: true },
  { id: 'map', name: 'Map', icon: '🌍', route: '/map', tagline: 'Living Places', available: true },
];

/**
 * <elohim-navigator> — unified cross-pillar navigation primitive.
 *
 * Implements the "Google Account" navigation pattern:
 * - Identity layer (profile bubble) with profile tray
 * - Context app switcher (pillar tray)
 * - Optional search bar
 * - Banner notices
 *
 * All routing is done by emitting events or calling callbacks. The element
 * holds no router state itself — callers handle navigation.
 *
 * Migrated from ElohimNavigatorComponent (app/elohim-app/elohim).
 *
 * @element elohim-navigator
 *
 * @prop {ElohimContextApp} context - Active context app
 * @prop {boolean} showSearch - Show the search bar
 * @prop {boolean} isAuthenticated - Whether the user is authenticated
 * @prop {string} displayName - Authenticated user display name
 * @prop {string} identifier - Authenticated user identifier (email)
 * @prop {string} identityMode - Identity mode ('hosted' | 'steward' | 'anonymous')
 * @prop {ElohimNavigatorSession | null} session - Session human state
 * @prop {ElohimContextAppConfig[]} contextApps - List of context apps to show
 * @prop {ElohimNavigatorBannerItem[]} banners - Active banner notices
 * @prop {Function | null} onNavigate - Called with { route } when navigation triggered
 * @prop {Function | null} onSearch - Called with { query } when search submitted
 * @prop {Function | null} onLogout - Called when logout requested
 * @prop {Function | null} onBannerDismiss - Called with { id } when a banner is dismissed
 *
 * @fires navigate - { detail: { route: string } }
 * @fires search - { detail: { query: string, context: string } }
 * @fires logout - when logout is requested
 * @fires banner-dismiss - { detail: { id: string } }
 * @fires profile-tray-toggle - { detail: { open: boolean } }
 * @fires context-switcher-toggle - { detail: { open: boolean } }
 *
 * @cssprop --elohim-nav-bg - Navigator background
 * @cssprop --elohim-nav-fg - Navigator foreground
 * @cssprop --elohim-nav-border - Navigator border
 * @cssprop --elohim-nav-bubble-bg - Profile bubble background
 * @cssprop --elohim-nav-bubble-fg - Profile bubble foreground
 * @cssprop --elohim-nav-tray-bg - Dropdown tray background
 * @cssprop --elohim-nav-tray-border - Dropdown tray border
 * @cssprop --elohim-nav-tray-shadow - Dropdown tray shadow
 * @cssprop --elohim-nav-search-bg - Search input background
 * @cssprop --elohim-nav-search-border - Search input border
 * @cssprop --elohim-nav-search-radius - Search input border radius
 *
 * @csspart nav - The root navigation bar
 * @csspart identity-section - The left identity section
 * @csspart search-section - The center search section
 * @csspart actions-section - The right actions section
 * @csspart profile-bubble - The profile bubble button
 * @csspart context-switcher-btn - The app-switcher button
 * @csspart profile-tray - The profile dropdown tray
 * @csspart context-tray - The context-switcher dropdown tray
 * @csspart search-input - The search input element
 * @csspart banner - Each banner notice
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en, es, he
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a
 */
export class ElohimNavigator extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
    }

    :host([hidden]) {
      display: none;
    }

    .nav {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding-block: 0.5rem;
      padding-inline: 1rem;
      background: var(--elohim-nav-bg, Canvas);
      color: var(--elohim-nav-fg, CanvasText);
      border-block-end: var(--elohim-nav-border, 1px solid color-mix(in oklch, currentColor 15%, transparent));
      position: relative;
    }

    .identity-section {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      flex-shrink: 0;
      position: relative;
    }

    .search-section {
      flex: 1;
      display: flex;
      justify-content: center;
    }

    .actions-section {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      flex-shrink: 0;
      position: relative;
    }

    .search-input {
      inline-size: 100%;
      max-inline-size: 480px;
      padding-block: 0.375rem;
      padding-inline: 0.75rem;
      border: var(--elohim-nav-search-border, 1px solid color-mix(in oklch, currentColor 25%, transparent));
      border-radius: var(--elohim-nav-search-radius, 999px);
      background: var(--elohim-nav-search-bg, Canvas);
      color: CanvasText;
      font: inherit;
      font-size: 0.875rem;
    }

    .search-input:focus {
      outline: 2px solid Highlight;
      outline-offset: 1px;
    }

    .profile-bubble {
      inline-size: 32px;
      block-size: 32px;
      border-radius: 50%;
      background: var(--elohim-nav-bubble-bg, ButtonFace);
      color: var(--elohim-nav-bubble-fg, ButtonText);
      border: none;
      cursor: pointer;
      font: inherit;
      font-size: 0.875rem;
      font-weight: 600;
      display: flex;
      align-items: center;
      justify-content: center;
      min-block-size: 44px;
      min-inline-size: 44px;
    }

    .profile-bubble:focus-visible {
      outline: 2px solid Highlight;
      outline-offset: 2px;
    }

    .context-switcher-btn {
      background: none;
      border: none;
      cursor: pointer;
      padding-block: 0.25rem;
      padding-inline: 0.5rem;
      font: inherit;
      font-size: 0.875rem;
      font-weight: 500;
      color: CanvasText;
      display: flex;
      align-items: center;
      gap: 0.25rem;
      min-block-size: 44px;
    }

    .context-switcher-btn:focus-visible {
      outline: 2px solid Highlight;
      outline-offset: 1px;
    }

    .tray {
      position: absolute;
      inset-block-start: calc(100% + 0.25rem);
      background: var(--elohim-nav-tray-bg, Canvas);
      border: var(--elohim-nav-tray-border, 1px solid color-mix(in oklch, currentColor 20%, transparent));
      border-radius: 0.5rem;
      box-shadow: var(--elohim-nav-tray-shadow, 0 4px 16px rgba(0, 0, 0, 0.15));
      z-index: 100;
      min-inline-size: 200px;
      padding: 0.5rem;
    }

    .profile-tray {
      inset-inline-end: 0;
    }

    .context-tray {
      inset-inline-start: 0;
    }

    .tray-item {
      display: block;
      inline-size: 100%;
      background: none;
      border: none;
      padding-block: 0.5rem;
      padding-inline: 0.75rem;
      text-align: start;
      cursor: pointer;
      font: inherit;
      font-size: 0.875rem;
      color: CanvasText;
      border-radius: 0.25rem;
      min-block-size: 44px;
    }

    .tray-item:hover,
    .tray-item:focus-visible {
      background: color-mix(in oklch, Canvas 90%, CanvasText);
      outline: none;
    }

    .tray-item.danger {
      color: LinkText;
    }

    .tray-divider {
      block-size: 1px;
      background: color-mix(in oklch, currentColor 15%, transparent);
      margin-block: 0.25rem;
      margin-inline: 0.5rem;
    }

    .context-app-item {
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .context-app-icon {
      font-size: 1.125rem;
      inline-size: 1.5rem;
      text-align: center;
    }

    .context-app-info {
      display: flex;
      flex-direction: column;
    }

    .context-app-name {
      font-weight: 500;
    }

    .context-app-tagline {
      font-size: 0.75rem;
      opacity: 0.6;
    }

    .banner {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 0.5rem;
      padding-block: 0.375rem;
      padding-inline: 1rem;
      font-size: 0.875rem;
    }

    .banner-info { background: color-mix(in oklch, Canvas 90%, LinkText); }
    .banner-warning { background: color-mix(in oklch, Canvas 85%, Canvas); }
    .banner-error { background: color-mix(in oklch, Canvas 85%, Canvas); }

    .banner-dismiss {
      background: none;
      border: none;
      cursor: pointer;
      padding: 0.25rem;
      color: GrayText;
      min-block-size: 44px;
      min-inline-size: 44px;
    }

    @media (pointer: coarse) {
      .tray-item,
      .context-switcher-btn,
      .profile-bubble {
        min-block-size: 44px;
      }
    }

    @media (forced-colors: active) {
      .nav {
        border-color: ButtonText;
        background: Canvas;
        color: CanvasText;
      }

      .tray {
        border-color: ButtonText;
        background: Canvas;
      }

      .profile-bubble {
        background: ButtonFace;
        color: ButtonText;
        border: 1px solid ButtonText;
      }

      .tray-item:hover {
        background: Highlight;
        color: HighlightText;
      }
    }
  `;

  @property({ reflect: true }) context: ElohimContextApp = 'lamad';
  @property({ type: Boolean }) showSearch = true;
  @property({ type: Boolean }) isAuthenticated = false;
  @property() displayName = 'Traveler';
  @property() identifier = '';
  @property() identityMode = 'anonymous';
  @property({ attribute: false }) session: ElohimNavigatorSession | null = null;
  @property({ attribute: false }) contextApps: ElohimContextAppConfig[] = DEFAULT_CONTEXT_APPS;
  @property({ attribute: false }) banners: ElohimNavigatorBannerItem[] = [];
  @property({ attribute: false }) onNavigate: ((route: string) => void) | null = null;
  @property({ attribute: false }) onSearch: ((query: string, context: string) => void) | null = null;
  @property({ attribute: false }) onLogout: (() => void) | null = null;
  @property({ attribute: false }) onBannerDismiss: ((id: string) => void) | null = null;

  @state() private showProfileTray = false;
  @state() private showContextSwitcher = false;
  @state() private searchQuery = '';

  override render() {
    const currentApp = this.contextApps.find(a => a.id === this.context) ?? this.contextApps[0];
    const initials = this.session?.initials ?? this.displayName?.charAt(0).toUpperCase() ?? 'T';

    return html`
      <div>
        ${this.banners.map(
          banner => html`
            <div
              class="banner banner-${banner.severity ?? 'info'}"
              part="banner"
              role="alert"
              data-testid="banner-${banner.id}"
            >
              <span>${banner.message}</span>
              <button
                class="banner-dismiss"
                type="button"
                aria-label="Dismiss"
                @click=${() => this.dismissBanner(banner.id)}
              >
                &times;
              </button>
            </div>
          `,
        )}

        <nav class="nav" part="nav" aria-label="Protocol navigation">
          <div class="identity-section" part="identity-section">
            <div style="position: relative;">
              <button
                class="context-switcher-btn"
                part="context-switcher-btn"
                type="button"
                aria-label="Switch context app"
                aria-expanded=${this.showContextSwitcher ? 'true' : 'false'}
                aria-haspopup="menu"
                @click=${this.toggleContextSwitcher}
              >
                <span aria-hidden="true">${currentApp?.icon ?? ''}</span>
                <span>${currentApp?.name ?? ''}</span>
                <span aria-hidden="true">▾</span>
              </button>

              ${this.showContextSwitcher
                ? html`
                    <div
                      class="tray context-tray"
                      part="context-tray"
                      role="menu"
                      aria-label="Context apps"
                    >
                      ${this.contextApps.map(
                        app => html`
                          <button
                            class="tray-item"
                            type="button"
                            role="menuitem"
                            aria-current=${app.id === this.context ? 'true' : 'false'}
                            data-testid="context-app-${app.id}"
                            ?disabled=${!app.available}
                            @click=${() => this.switchContext(app)}
                          >
                            <div class="context-app-item">
                              <span class="context-app-icon" aria-hidden="true">${app.icon}</span>
                              <div class="context-app-info">
                                <span class="context-app-name">${app.name}</span>
                                <span class="context-app-tagline">${app.tagline}</span>
                              </div>
                            </div>
                          </button>
                        `,
                      )}
                    </div>
                  `
                : nothing}
            </div>
          </div>

          ${this.showSearch
            ? html`
                <div class="search-section" part="search-section">
                  <input
                    class="search-input"
                    part="search-input"
                    type="search"
                    placeholder="Search ${currentApp?.name ?? 'protocol'}..."
                    aria-label="Search ${currentApp?.name ?? 'protocol'}"
                    .value=${this.searchQuery}
                    @input=${(e: Event) => {
                      this.searchQuery = (e.target as HTMLInputElement).value;
                    }}
                    @keydown=${(e: KeyboardEvent) => {
                      if (e.key === 'Enter') this.handleSearch();
                    }}
                  />
                </div>
              `
            : nothing}

          <div class="actions-section" part="actions-section">
            <div style="position: relative;">
              <button
                class="profile-bubble"
                part="profile-bubble"
                type="button"
                aria-label="${this.isAuthenticated ? this.displayName : 'Sign in'}"
                aria-expanded=${this.showProfileTray ? 'true' : 'false'}
                aria-haspopup="menu"
                data-testid="profile-bubble"
                @click=${this.toggleProfileTray}
              >
                ${initials}
              </button>

              ${this.showProfileTray
                ? html`
                    <div
                      class="tray profile-tray"
                      part="profile-tray"
                      role="menu"
                      aria-label="Profile menu"
                    >
                      ${this.isAuthenticated
                        ? html`
                            <div style="padding: 0.5rem 0.75rem; font-size: 0.875rem;">
                              <div style="font-weight: 600;">${this.displayName}</div>
                              ${this.identifier
                                ? html`<div style="opacity: 0.6; font-size: 0.8em;">${this.identifier}</div>`
                                : nothing}
                            </div>
                            <div class="tray-divider"></div>
                            <button
                              class="tray-item"
                              type="button"
                              role="menuitem"
                              data-testid="nav-identity-profile"
                              @click=${() => this.navigate('/identity/profile')}
                            >
                              Identity Profile
                            </button>
                            <div class="tray-divider"></div>
                            <button
                              class="tray-item danger"
                              type="button"
                              role="menuitem"
                              data-testid="nav-logout"
                              @click=${this.handleLogout}
                            >
                              Sign out
                            </button>
                          `
                        : html`
                            <button
                              class="tray-item"
                              type="button"
                              role="menuitem"
                              data-testid="nav-login"
                              @click=${() => this.navigate('/identity/login')}
                            >
                              Sign in
                            </button>
                            <button
                              class="tray-item"
                              type="button"
                              role="menuitem"
                              data-testid="nav-register"
                              @click=${() => this.navigate('/identity/register')}
                            >
                              Register
                            </button>
                          `}
                    </div>
                  `
                : nothing}
            </div>
          </div>
        </nav>
      </div>
    `;
  }

  private navigate(route: string): void {
    this.showProfileTray = false;
    this.showContextSwitcher = false;
    if (this.onNavigate) {
      this.onNavigate(route);
    } else {
      this.dispatchEvent(
        new CustomEvent('navigate', { detail: { route }, bubbles: true, composed: true }),
      );
    }
  }

  private toggleProfileTray(): void {
    this.showProfileTray = !this.showProfileTray;
    if (this.showProfileTray) this.showContextSwitcher = false;
    this.dispatchEvent(
      new CustomEvent('profile-tray-toggle', {
        detail: { open: this.showProfileTray },
        bubbles: true,
        composed: true,
      }),
    );
  }

  private toggleContextSwitcher(): void {
    this.showContextSwitcher = !this.showContextSwitcher;
    if (this.showContextSwitcher) this.showProfileTray = false;
    this.dispatchEvent(
      new CustomEvent('context-switcher-toggle', {
        detail: { open: this.showContextSwitcher },
        bubbles: true,
        composed: true,
      }),
    );
  }

  private switchContext(app: ElohimContextAppConfig): void {
    if (!app.available) return;
    this.showContextSwitcher = false;
    this.navigate(app.route);
  }

  private handleSearch(): void {
    if (!this.searchQuery.trim()) return;
    if (this.onSearch) {
      this.onSearch(this.searchQuery, this.context);
    }
    this.dispatchEvent(
      new CustomEvent('search', {
        detail: { query: this.searchQuery, context: this.context },
        bubbles: true,
        composed: true,
      }),
    );
  }

  private handleLogout(): void {
    this.showProfileTray = false;
    if (this.onLogout) {
      this.onLogout();
    }
    this.dispatchEvent(new CustomEvent('logout', { bubbles: true, composed: true }));
  }

  private dismissBanner(id: string): void {
    if (this.onBannerDismiss) {
      this.onBannerDismiss(id);
    }
    this.dispatchEvent(
      new CustomEvent('banner-dismiss', {
        detail: { id },
        bubbles: true,
        composed: true,
      }),
    );
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'elohim-navigator': ElohimNavigator;
  }
}
