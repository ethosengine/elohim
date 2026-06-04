import {
  AfterViewInit,
  Component,
  CUSTOM_ELEMENTS_SCHEMA,
  ElementRef,
  OnInit,
  ViewChild,
  signal,
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { StandaloneResolver, type ConsentContext } from './services/standalone-resolver';
import {
  readStewardHandoff,
  runStewardLogin,
  stripHandoffFromSearch,
  type StewardLoginEffects,
} from './services/steward-login-controller';
import type { AuthorityResolution } from 'elohim-imagodei';

type PortalMode = 'login' | 'consent' | 'steward-login';
// 'steward-login' is the transient "Connecting to your steward portal…" step;
// the shell only knows 'resolve' | 'login' | 'consent' | 'callback', so while
// mode is 'steward-login' we hold the shell on 'resolve' and render our own
// connecting / failure copy in the primary slot.
type PortalStep = 'resolve' | 'login' | 'consent' | 'callback';
type StewardPhase = 'connecting' | 'failed' | 'unreachable';

@Component({
  selector: 'imagodei-portal-root',
  standalone: true,
  imports: [CommonModule],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  template: `
    <main>
      <h1 class="visually-hidden">Elohim Portal</h1>
      <elohim-imagodei-portal-shell #shell [attr.step]="step()" [authority]="authority()">

        <ng-container *ngIf="mode() === 'login' && step() === 'resolve'">
          <elohim-imagodei-federated-resolver
            #resolver
            slot="primary"
            remember-key="elohim_auth_identifier"
            (resolved)="onResolved($event)"
            (resolve-error)="onResolveError($event)"
          >
            <span slot="help-text">Sign in with your federated identifier — for example, matthew&#64;alpha.elohim.host.</span>
          </elohim-imagodei-federated-resolver>
        </ng-container>

        <ng-container *ngIf="mode() === 'login' && step() === 'login'">
          <elohim-imagodei-login-card
            #loginCard
            slot="primary"
            [attr.remembered-identifier]="identifier()"
            allow-password
            (password-submit)="onPasswordSubmit($event)"
            (oauth-start)="onOAuthStart($event)"
          ></elohim-imagodei-login-card>
        </ng-container>

        <ng-container *ngIf="mode() === 'steward-login'">
          <div slot="primary" class="steward-login" role="status" aria-live="polite">
            <ng-container *ngIf="stewardPhase() === 'connecting'">
              <p class="steward-login__lead">Connecting to your steward portal…</p>
              <p class="steward-login__detail">
                We're handing your sign-in over to the conductor that holds your identity.
              </p>
            </ng-container>

            <ng-container *ngIf="stewardPhase() !== 'connecting'">
              <p class="steward-login__lead">{{ stewardMessage() }}</p>
              <a
                *ngIf="stewardReturnUrl()"
                class="steward-login__return"
                [href]="stewardReturnUrl()"
                rel="noopener"
              >Return to your doorway</a>
            </ng-container>
          </div>
        </ng-container>

        <ng-container *ngIf="mode() === 'consent' && consentCtx() !== null">
          <elohim-imagodei-consent-card
            #consentCard
            slot="primary"
            [requestingClient]="consentCtx()!.requestingClient"
            [requestedClaims]="consentCtx()!.requestedClaims"
            [requiredClaims]="consentCtx()!.requiredClaims"
            (approve)="onConsentApprove($event)"
            (decline)="onConsentDecline($event)"
          ></elohim-imagodei-consent-card>
        </ng-container>

        <div slot="error-region" *ngIf="errorMessage()" role="alert">
          {{ errorMessage() }}
        </div>

      </elohim-imagodei-portal-shell>
    </main>
  `,
  styles: [`
    .visually-hidden {
      position: absolute;
      inline-size: 1px;
      block-size: 1px;
      padding: 0;
      margin: -1px;
      overflow: hidden;
      clip: rect(0, 0, 0, 0);
      white-space: nowrap;
      border: 0;
    }

    .steward-login {
      display: flex;
      flex-direction: column;
      gap: 0.75rem;
      text-align: center;
    }

    .steward-login__lead {
      margin: 0;
      font-weight: 600;
    }

    .steward-login__detail {
      margin: 0;
      opacity: 0.75;
      font-size: 0.875rem;
    }

    .steward-login__return {
      align-self: center;
      color: LinkText;
    }
  `],
})
export class AppComponent implements OnInit, AfterViewInit {
  private readonly resolverService = new StandaloneResolver();

  mode = signal<PortalMode>('login');
  step = signal<PortalStep>('resolve');
  identifier = signal<string>('');
  consentCtx = signal<ConsentContext | null>(null);
  errorMessage = signal<string>('');

  /** Steward-handoff display phase; only meaningful while mode === 'steward-login'. */
  stewardPhase = signal<StewardPhase>('connecting');
  /** Non-scary copy shown on a failed/unreachable steward handoff. */
  stewardMessage = signal<string>('');
  /** The doorway origin to offer as a "return to your doorway" action. */
  stewardReturnUrl = signal<string>('');

  /** Pre-fetched authority resolution from `GET /auth/me`. Null until fetch completes. */
  authority = signal<AuthorityResolution | null>(null);

  /** The federated-resolver Lit element — receives the resolveIdentifier callback. */
  @ViewChild('resolver') resolverRef?: ElementRef<HTMLElement>;
  /** The login-card Lit element — receives setError calls on auth failure. */
  @ViewChild('loginCard') loginCardRef?: ElementRef<HTMLElement>;

  async ngOnInit(): Promise<void> {
    // Pre-fetch authority so the shell receives it as a property (not via its own fetch).
    // Failure is non-fatal — shell renders placeholder chrome and emits authority-needed.
    void this._prefetchAuthority();

    const search = window.location.search;

    // 1) Doorway→steward handoff. The doorway redirected us here with an opaque
    //    session_token + the issuer origin. Redeem it before the normal flow.
    const handoff = readStewardHandoff(search);
    if (handoff) {
      this.mode.set('steward-login');
      this.stewardPhase.set('connecting');
      const outcome = await runStewardLogin(
        this.resolverService,
        handoff,
        search,
        this._stewardEffects(),
      );
      if (outcome.status === 'authenticated') {
        // The session landed; refreshAuthority + consent re-check already ran in
        // the effects. If no consent was requested, fall back to the login chrome
        // (the shell now reflects the authenticated peer-conductor authority).
        if (this.mode() === 'steward-login') {
          this.mode.set('login');
          this.step.set('resolve');
        }
      } else {
        this.stewardPhase.set(outcome.status === 'unreachable' ? 'unreachable' : 'failed');
        this.stewardMessage.set(outcome.message);
        this.stewardReturnUrl.set(outcome.returnToDoorwayUrl ?? handoff.doorwayUrl);
      }
      return;
    }

    // 2) Direct OAuth consent request (no handoff).
    await this._enterConsentIfRequested(search);
  }

  /** Effects seam handed to the steward-login controller. */
  private _stewardEffects(): StewardLoginEffects {
    return {
      stripHandoffParams: (search: string) => {
        // Strip ONLY the handoff params; preserve OAuth params for the consent
        // flow. The opaque code must never linger in the address bar / history.
        const cleaned = stripHandoffFromSearch(search);
        const url = window.location.pathname + cleaned + window.location.hash;
        window.history.replaceState(window.history.state, '', url);
      },
      refreshAuthority: () => this._prefetchAuthority(),
      enterConsentIfRequested: (search: string) => this._enterConsentIfRequested(search),
    };
  }

  /**
   * If the URL carries an OAuth consent request (client_id + claims), prepare
   * and enter the consent step. Shared by the direct-consent path and the
   * post-handoff path so the OAuth params survive the steward hop.
   */
  private async _enterConsentIfRequested(search: string): Promise<void> {
    const params = new URLSearchParams(search);
    if (!(params.has('client_id') && params.has('claims'))) return;

    this.mode.set('consent');
    this.step.set('consent');
    try {
      const ctx = await this.resolverService.prepareConsent({
        clientId: params.get('client_id')!,
        claims: (params.get('claims') ?? '').split(',').filter(Boolean),
        redirectUri: params.get('redirect_uri') ?? '',
        state: params.get('state') ?? '',
      });
      this.consentCtx.set(ctx);
    } catch (e) {
      this.errorMessage.set(e instanceof Error ? e.message : 'consent preparation failed');
    }
  }

  private async _prefetchAuthority(): Promise<void> {
    // trustMode is DISCOVERED here from /auth/me, never configured — the same
    // bundle runs in doorway-host and peer-conductor modes and learns which
    // from the wire (peer-conductor after a successful steward handoff).
    const authority = await this.resolverService.fetchAuthority();
    if (authority) {
      this.authority.set(authority);
    }
    // Null ⇒ leave authority null; the shell emits authority-needed.
  }

  ngAfterViewInit(): void {
    // Wire the resolveIdentifier callback so the Lit element can call it
    // without depending on Angular DI. The federated-resolver element calls
    // this function and fires (resolved) or (resolve-error) events in response.
    if (this.resolverRef) {
      (this.resolverRef.nativeElement as unknown as Record<string, unknown>)['resolveIdentifier'] =
        this.resolverService.resolveIdentifier.bind(this.resolverService);
    }
  }

  onResolved(e: Event): void {
    const detail = (e as CustomEvent<{ identifier: string; doorwayUrl: string }>).detail;
    this.identifier.set(detail.identifier);
    this.errorMessage.set('');
    this.step.set('login');
    // The login-card attribute binding (remembered-identifier) is handled
    // declaratively via signal; no imperative wiring needed.
  }

  onResolveError(e: Event): void {
    const detail = (e as CustomEvent<{ reason: string }>).detail;
    this.errorMessage.set(`Could not resolve: ${detail.reason}`);
  }

  async onPasswordSubmit(e: Event): Promise<void> {
    const detail = (e as CustomEvent<{ identifier: string; password: string; remember: boolean }>).detail;
    const ident = detail.identifier || this.identifier();
    this.errorMessage.set('');

    const out = await this.resolverService.loginWithPassword({
      identifier: ident,
      password: detail.password,
      remember: detail.remember,
    });

    if (out.error) {
      this.errorMessage.set(out.error);
      // Surface the error directly into the login-card via its setError method,
      // so the card can display it inline without relying on the slot region.
      const card = this.loginCardRef?.nativeElement as unknown as Record<string, unknown> | undefined;
      if (typeof card?.['setError'] === 'function') {
        (card['setError'] as (msg: string) => void)(out.error);
      }
    } else if (out.redirect) {
      window.location.href = out.redirect;
    }
  }

  onOAuthStart(e: Event): void {
    const detail = (e as CustomEvent<{ providerId: string }>).detail;
    // Redirect to doorway's OAuth provider initiation route.
    window.location.href = `/auth/oauth/${detail.providerId}`;
  }

  async onConsentApprove(e: Event): Promise<void> {
    const detail = (e as CustomEvent<{ grantedClaims: string[] }>).detail;
    const params = new URLSearchParams(window.location.search);
    const state = params.get('state') ?? '';
    try {
      const resp = await fetch('/auth/authorize/grant', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({ grantedClaims: detail.grantedClaims, state }),
      });
      if (!resp.ok) throw new Error(`grant failed: ${resp.status}`);
      const result = await resp.json() as { redirect: string };
      window.location.href = result.redirect;
    } catch (err) {
      this.errorMessage.set(err instanceof Error ? err.message : 'consent grant failed');
    }
  }

  async onConsentDecline(e: Event): Promise<void> {
    const detail = (e as CustomEvent<{ reason: string }>).detail;
    const params = new URLSearchParams(window.location.search);
    const state = params.get('state') ?? '';
    try {
      const resp = await fetch('/auth/authorize/decline', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({ state, reason: detail.reason }),
      });
      if (!resp.ok) throw new Error(`decline failed: ${resp.status}`);
      const result = await resp.json() as { redirect: string };
      window.location.href = result.redirect;
    } catch (err) {
      this.errorMessage.set(err instanceof Error ? err.message : 'consent decline failed');
    }
  }
}
