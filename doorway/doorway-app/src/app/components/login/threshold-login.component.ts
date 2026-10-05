/**
 * Threshold Login Component
 *
 * Handles OAuth authorization at the doorway.
 * When elohim-app redirects a user here, this component:
 * 1. Reads OAuth params from URL (?client_id, ?redirect_uri, ?response_type, ?state)
 * 2. Shows login form for the user to authenticate
 * 3. On success, generates authorization code and redirects back to elohim-app
 *
 * This enables the thin-federated architecture where any doorway
 * can be an identity provider for elohim-app.
 */

import { CUSTOM_ELEMENTS_SCHEMA, Component, OnInit, inject, signal, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { ActivatedRoute, Router } from '@angular/router';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';
import { firstValueFrom } from 'rxjs';

// Wire shapes (/auth/login, /auth/session-token) are the schema-contract-
// pinned generated contracts (auth-wire plan Task 4 — the drifted local
// duplicates, e.g. `expiresAt: string`, were retired).
import type { AuthResponse } from '../../generated/auth-response';
import type { SessionTokenResponse } from '../../generated/session-token-response';
import { gatewayDomain } from '../../core/gateway-domain';
import { RETURN_URL_PARAM, safeReturnUrl } from '../../core/guards/auth.guard';
import { AuthStateService } from '../../services/auth-state.service';
import { IDENTITY_ELEMENTS } from '../../elements/identity-elements';
import { type WitnessStep, withStepState } from '../../models/witness-step';
import { DEVICE_CONSENT_PATH } from '../consent/device-consent.logic';

/** The one thing that is true while sign-in is in flight: the doorway checks the password. */
const DOORWAY_CHECK = 'doorway-check';

/** OAuth params from query string */
interface OAuthParams {
  clientId: string;
  redirectUri: string;
  responseType: string;
  state: string;
  scope?: string;
  loginHint?: string;
}

/** Login form state */
interface LoginForm {
  identifier: string;
  password: string;
}

/** State machine for login flow */
type LoginState = 'form' | 'authenticating' | 'authorizing' | 'error';

@Component({
  selector: 'app-threshold-login',
  standalone: true,
  imports: [CommonModule, FormsModule],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  template: `
    <div class="portal-page">
      <section class="portal-card" aria-labelledby="threshold-login-title">
        <span class="portal-mark" role="img" aria-label="Elohim"></span>
        <div class="portal-heading">
          <h1 id="threshold-login-title">Sign in</h1>
          <p>Welcome back.</p>
        </div>

        @if (oauthParams()) {
          <!-- prettier-ignore -->
          <p class="portal-context oauth-info">
            Sign in to continue to <strong class="app-name">{{ clientDisplayName() }}</strong>.
          </p>
        } @else if (returningToDeviceConsent()) {
          <p class="portal-context" data-testid="threshold-device-consent-context">
            Sign in to review a device that is asking to act for you.
          </p>
        }

        @if (error()) {
          <div class="error-banner" role="alert" data-testid="threshold-error">
            <span id="threshold-error-text">{{ error() }}</span>
            <button
              class="dismiss"
              type="button"
              aria-label="Dismiss this message"
              (click)="clearError()"
              data-testid="threshold-error-dismiss"
            >
              ×
            </button>
          </div>
        }

        @if (state() === 'form' || state() === 'authenticating') {
          <form
            class="portal-form"
            (ngSubmit)="onSubmit(loginForm.valid)"
            #loginForm="ngForm"
            [attr.aria-busy]="busy() ? 'true' : null"
          >
            <div class="portal-field">
              <label for="identifier">Username</label>
              <div class="identifier-wrapper" [class.is-invalid]="identifierError()">
                <input
                  type="text"
                  id="identifier"
                  name="identifier"
                  data-testid="threshold-identifier"
                  [ngModel]="form.identifier"
                  (ngModelChange)="onIdentifierChange($event)"
                  #identifierModel="ngModel"
                  required
                  autocomplete="username"
                  autocapitalize="none"
                  spellcheck="false"
                  pattern="[^@\\s]+"
                  inputmode="text"
                  class="identifier-input"
                  [readonly]="busy()"
                  [attr.aria-invalid]="identifierError() ? 'true' : null"
                  [attr.aria-describedby]="
                    identifierError() ? 'identifier-error identifier-hint' : 'identifier-hint'
                  "
                />
                <span
                  class="domain-suffix"
                  data-testid="threshold-domain-suffix"
                  aria-hidden="true"
                >
                  &#64;{{ gatewayDomain() }}
                </span>
              </div>
              @if (identifierError(); as message) {
                <p class="field-error" id="identifier-error">{{ message }}</p>
              }
              <!-- prettier-ignore -->
              <p class="input-hint" id="identifier-hint">
                Your account lives at <strong>{{ gatewayDomain() }}</strong>, so type only the part before the &#64;.
                Account somewhere else?
                <a class="portal-link" [href]="federatedLoginUrl()" data-testid="threshold-different-doorway-hint">Use a different doorway</a>
              </p>
            </div>

            <div class="portal-field">
              <label for="password">Password</label>
              <input
                type="password"
                id="password"
                name="password"
                data-testid="threshold-password"
                [(ngModel)]="form.password"
                required
                autocomplete="current-password"
                [readonly]="busy()"
                [attr.aria-invalid]="passwordError() ? 'true' : null"
                [attr.aria-describedby]="passwordError() ? 'password-error' : null"
              />
              @if (passwordError(); as message) {
                <p class="field-error" id="password-error">{{ message }}</p>
              }
            </div>

            <button
              type="submit"
              class="btn-primary"
              data-testid="threshold-submit"
              [attr.aria-disabled]="busy() ? 'true' : null"
            >
              {{ busy() ? 'Signing in…' : 'Sign in' }}
            </button>

            <!-- Only after a submit, and only once the element has loaded while
                 the request is still in flight. It never delays navigation. -->
            @if (trailSteps(); as steps) {
              @if (trailElementReady()) {
                <elohim-imagodei-witness-trail
                  class="portal-trail"
                  data-testid="threshold-witness-trail"
                  layout="list"
                  [steps]="steps"
                  [attr.reveal-after-ms]="trailRevealAfterMs()"
                ></elohim-imagodei-witness-trail>
              }
            }

            @if (oauthParams()) {
              <div class="federated-section">
                <div class="divider"><span>or</span></div>
                <a
                  [href]="federatedLoginUrl()"
                  class="federated-link"
                  data-testid="threshold-federated-login"
                >
                  Sign in with a different doorway
                </a>
              </div>
            }
          </form>
        }

        @if (state() === 'authorizing') {
          <div class="loading-state" role="status">
            <div class="trace" aria-hidden="true">
              <span></span>
              <span></span>
              <span></span>
            </div>
            <p>Taking you to {{ clientDisplayName() }}…</p>
          </div>
        }

        @if (state() === 'error') {
          <div class="error-state">
            <button
              class="btn-secondary"
              type="button"
              data-testid="threshold-retry"
              (click)="retry()"
            >
              Try again
            </button>
          </div>
        }

        <div class="portal-footer">
          <p>
            New here?
            <a class="portal-link" [href]="registerUrl()" data-testid="threshold-register-link">
              Create an account
            </a>
          </p>
        </div>
      </section>
    </div>
  `,
  styleUrl: './threshold-login.component.css',
})
export class ThresholdLoginComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly http = inject(HttpClient);
  private readonly authState = inject(AuthStateService);
  private readonly loadElements = inject(IDENTITY_ELEMENTS);

  // State
  readonly state = signal<LoginState>('form');
  readonly error = signal<string>('');
  readonly oauthParams = signal<OAuthParams | null>(null);
  /** In-app page to return to after signing in (set by authGuard), if any. */
  readonly returnUrl = signal<string | null>(null);
  /** True once the person has tried to submit — field messages show from then on. */
  readonly attempted = signal(false);
  /** The request is in flight: the button says so and the form holds still. */
  readonly busy = computed(() => this.state() === 'authenticating');
  /** What the witness trail shows; null until the person submits. */
  readonly trailSteps = signal<WitnessStep[] | null>(null);
  /** The trail element registered while a request was in flight. */
  readonly trailElementReady = signal(false);
  /** How long the live trail waits before showing (the dev preview pins it to 0). */
  readonly trailRevealAfterMs = signal(400);

  // Form model
  form: LoginForm = {
    identifier: '',
    password: '',
  };

  // Computed values
  /** doorway-alpha.elohim.host --> alpha.elohim.host */
  readonly gatewayDomain = computed(() => gatewayDomain(window.location.hostname));

  /** Coming here on the way to approving a device: say so above the form. */
  readonly returningToDeviceConsent = computed(
    () => this.returnUrl()?.startsWith(DEVICE_CONSENT_PATH) ?? false
  );

  readonly clientDisplayName = computed(() => {
    const params = this.oauthParams();
    if (!params) return 'Unknown App';

    // Map known client IDs to friendly names
    const clientNames: Record<string, string> = {
      'elohim-app': 'Elohim App',
      'doorway-app': 'Doorway Dashboard',
    };

    return clientNames[params.clientId] ?? params.clientId;
  });

  readonly registerUrl = computed(() => {
    // Link to doorway's own registration page with OAuth params
    const params = this.oauthParams();
    if (params) {
      const searchParams = new URLSearchParams({
        client_id: params.clientId,
        redirect_uri: params.redirectUri,
        response_type: params.responseType,
        state: params.state,
      });
      if (params.scope) {
        searchParams.set('scope', params.scope);
      }
      return `/threshold/register?${searchParams.toString()}`;
    }
    return '/threshold/register';
  });

  readonly federatedLoginUrl = computed(() => {
    const params = this.oauthParams();
    if (params) {
      const searchParams = new URLSearchParams({
        client_id: params.clientId,
        redirect_uri: params.redirectUri,
        response_type: params.responseType,
        state: params.state,
      });
      if (params.scope) {
        searchParams.set('scope', params.scope);
      }
      return `/threshold/doorways?${searchParams.toString()}`;
    }
    return '/threshold/doorways';
  });

  ngOnInit(): void {
    // Parse OAuth params from URL
    this.parseOAuthParams();
  }

  private parseOAuthParams(): void {
    const params = this.route.snapshot.queryParams;

    const clientId = params['client_id'];
    const redirectUri = params['redirect_uri'];
    const responseType = params['response_type'];
    const state = params['state'];
    const scope = params['scope'];
    const loginHint = params['login_hint'];

    if (clientId && redirectUri && state) {
      this.oauthParams.set({
        clientId,
        redirectUri,
        responseType: responseType ?? 'code',
        state,
        scope,
        loginHint,
      });
    }

    // Pre-fill identifier from login_hint
    if (loginHint) {
      this.form.identifier = loginHint;
    }

    this.returnUrl.set(safeReturnUrl(params[RETURN_URL_PARAM]));
  }

  /** Message for the username field, once a submit has been tried. */
  identifierError(): string | null {
    if (!this.attempted()) return null;
    if (!this.form.identifier) return 'Enter your username.';
    if (/\s/.test(this.form.identifier)) return 'A username can’t contain spaces.';
    return null;
  }

  /** Message for the password field, once a submit has been tried. */
  passwordError(): string | null {
    return this.attempted() && !this.form.password ? 'Enter your password.' : null;
  }

  /**
   * Strip any '@' segment a paste or muscle-memory may have introduced —
   * the gateway suffix is enforced by the doorway, not chosen by the user.
   * Federation routing lives behind the "Use a different doorway" link.
   */
  onIdentifierChange(value: string): void {
    const atIndex = value.indexOf('@');
    this.form.identifier = atIndex === -1 ? value : value.slice(0, atIndex);
  }

  /**
   * @param formValid the template form's validity; the button is never
   *   disabled, so an incomplete form explains itself instead of looking dead.
   */
  async onSubmit(formValid: boolean | null = true): Promise<void> {
    if (this.state() !== 'form') return; // already in flight
    this.attempted.set(true);
    if (formValid === false || !this.form.identifier || !this.form.password) {
      return;
    }

    this.state.set('authenticating');
    this.error.set('');
    this.startTrail();

    try {
      const authResult = await this.authenticate();

      if (!authResult) {
        throw new Error('Authentication failed');
      }
      this.markTrail('done');

      // Steward handoff — doorway never owns a steward's login portal.
      // When the auth response carries a reachable portalHostUrl, the human
      // is a graduated steward and their peer-native portal is the
      // authoritative identity provider. Doorway becomes the relying party
      // here: hand the session to the portal host, which will complete the
      // OAuth code dance back at the original client_id.
      //
      // The redirect carries a single-use transfer code minted from the
      // existing GET /auth/session-token endpoint — the doorway JWT must
      // NEVER ride a URL (history/referrer/log leakage). The portal's
      // storage redeems the code server-to-server via
      // GET {doorway_url}/auth/exchange-session, so doorway_url rides along.
      // If the mint fails, login falls through to the local path below
      // rather than blocking the human.
      if (authResult.portalHostUrl) {
        this.state.set('authorizing');
        const minted = await this.mintSessionToken(authResult.token);
        if (minted) {
          const params = this.oauthParams();
          const handoff = new URL(authResult.portalHostUrl);
          handoff.searchParams.set('session_token', minted);
          handoff.searchParams.set('doorway_url', window.location.origin);
          if (params) {
            handoff.searchParams.set('client_id', params.clientId);
            handoff.searchParams.set('redirect_uri', params.redirectUri);
            handoff.searchParams.set('response_type', params.responseType);
            handoff.searchParams.set('state', params.state);
            if (params.scope) handoff.searchParams.set('scope', params.scope);
          }
          window.location.href = handoff.toString();
          return;
        }
        console.warn('[threshold-login] session-token mint failed; continuing with local auth');
      }

      this.authState.storeToken(authResult.token);

      const params = this.oauthParams();
      if (params) {
        this.state.set('authorizing');
        await this.authorizeOAuth(authResult.token, params);
      } else {
        await this.authState.refresh();
        const back = this.returnUrl();
        if (back) {
          this.router.navigateByUrl(back);
        } else {
          this.router.navigate(['/dashboard']);
        }
      }
    } catch (err) {
      this.markTrail('failed');
      this.state.set('form');
      if (err instanceof HttpErrorResponse) {
        this.error.set(err.error?.error ?? 'Authentication failed');
      } else {
        this.error.set(err instanceof Error ? err.message : 'An error occurred');
      }
    }
  }

  private async authenticate(): Promise<AuthResponse | null> {
    const response = await firstValueFrom(
      this.http.post<AuthResponse>('/auth/login', {
        identifier: this.form.identifier,
        password: this.form.password,
      })
    );
    return response;
  }

  /**
   * Mint a single-use session-transfer code from the doorway's existing
   * GET /auth/session-token endpoint (Bearer-authenticated, 60s TTL,
   * consumed exactly once by the portal host's back-channel redeem).
   * Returns null on any failure — the caller falls through to local auth
   * rather than ever putting the JWT itself in a redirect URL.
   */
  private async mintSessionToken(token: string): Promise<string | null> {
    try {
      const response = await firstValueFrom(
        this.http.get<SessionTokenResponse>('/auth/session-token', {
          headers: { Authorization: `Bearer ${token}` },
        })
      );
      return response.sessionToken || null;
    } catch {
      return null;
    }
  }

  private async authorizeOAuth(token: string, params: OAuthParams): Promise<void> {
    // Call /auth/authorize with the token to get the authorization code
    // The backend will redirect us to the client's redirect_uri
    const authorizeUrl = new URL('/auth/authorize', window.location.origin);
    authorizeUrl.searchParams.set('client_id', params.clientId);
    authorizeUrl.searchParams.set('redirect_uri', params.redirectUri);
    authorizeUrl.searchParams.set('response_type', params.responseType);
    authorizeUrl.searchParams.set('state', params.state);
    if (params.scope) {
      authorizeUrl.searchParams.set('scope', params.scope);
    }

    // Request authorization code. Backend returns JSON { redirect_uri }
    // when it sees a Bearer token (SPA flow), avoiding cross-origin 302.
    const response = await fetch(authorizeUrl.toString(), {
      method: 'GET',
      headers: {
        Authorization: `Bearer ${token}`,
      },
    });

    if (response.ok) {
      const data = await response.json();
      if (data.redirect_uri) {
        window.location.href = data.redirect_uri;
      } else {
        this.error.set('Authorization completed but no redirect received');
        this.state.set('error');
      }
    } else {
      throw new Error('Authorization failed');
    }
  }

  /**
   * Replace the waiting indicator with the witness trail: one step, the
   * doorway checking the password. The element loads on demand; if it is not
   * registered before the request finishes, nothing is shown.
   */
  private startTrail(): void {
    this.trailSteps.set([
      {
        id: DOORWAY_CHECK,
        act: 'checked',
        relation: 'your-doorway',
        label: this.gatewayDomain(),
        state: 'working',
      },
    ]);
    if (this.trailElementReady()) return;
    void this.loadElements().then(loaded => {
      if (loaded && this.busy()) this.trailElementReady.set(true);
    });
  }

  private markTrail(state: WitnessStep['state']): void {
    const steps = this.trailSteps();
    if (steps) this.trailSteps.set(withStepState(steps, DOORWAY_CHECK, state));
  }

  clearError(): void {
    this.error.set('');
  }

  retry(): void {
    this.state.set('form');
    this.error.set('');
  }
}
