import {
  AfterViewInit,
  ChangeDetectionStrategy,
  Component,
  CUSTOM_ELEMENTS_SCHEMA,
  ElementRef,
  OnInit,
  Type,
  ViewChild,
  computed,
  signal,
} from '@angular/core';
import { CommonModule } from '@angular/common';
import {
  IdentityStandingController,
  createIdentityStandingClient,
  type IdentityPageState,
} from 'elohim-imagodei/identity-standing';
import {
  SignInController,
  createNodeSignInClient,
  type SignInPageState,
} from 'elohim-imagodei/node-sign-in';
import { createPendingAsksClient, type PendingAsksClient } from 'elohim-imagodei/pending-asks';
import { sessionKeys } from 'elohim-imagodei/session-key';
import { DeviceApprovalComponent } from './device-approval/device-approval.component';
import { IdentityViewComponent } from './identity/identity-view.component';
import { PendingAsksComponent } from './pending/pending-asks.component';
import { SignInViewComponent } from './sign-in/sign-in-view.component';
import { portalBase } from './device-approval/device-approval-port';
import { RETURN_TO_PARAM, SIGN_IN_PARAM, safePortalReturn } from './services/portal-return';
import { StandaloneResolver, type ConsentContext } from './services/standalone-resolver';
import {
  readStewardHandoff,
  runStewardLogin,
  stripHandoffFromSearch,
  type StewardLoginEffects,
} from './services/steward-login-controller';
import type { AuthorityResolution } from 'elohim-imagodei';

type PortalMode =
  | 'login'
  | 'consent'
  | 'steward-login'
  | 'device-consent'
  | 'identity'
  | 'sign-in'
  | 'secret-unset';

/** What the root shows, from what the node said about the identity here. */
export type RootPage = 'identity' | 'sign-in' | 'secret-unset' | 'legacy';

/**
 * - an identity and a live session → what it rests on (unless the person
 *   was sent to sign in again, or this browser lost the key the session is
 *   bound to — then sign in);
 * - no identity here → begin it;
 * - an identity, no one signed in → sign in, or, with no sign-in secret
 *   set, where one is set;
 * - a host that does not answer → the sign-in this bundle always had.
 */
export function rootPageFor(
  identity: IdentityPageState,
  { signInAsked, keyLost }: { signInAsked: boolean; keyLost: boolean }
): RootPage {
  switch (identity.phase) {
    case 'standing':
      return signInAsked || keyLost ? 'sign-in' : 'identity';
    case 'begin':
    case 'beginning':
      return 'identity';
    case 'not-signed-in':
      return identity.signedOut?.signInSecretSet === false ? 'secret-unset' : 'sign-in';
    default:
      return 'legacy';
  }
}
// 'steward-login' is the transient "Connecting to your steward portal…" step;
// the shell only knows 'resolve' | 'login' | 'consent' | 'callback', so while
// mode is 'steward-login' we hold the shell on 'resolve' and render our own
// connecting / failure copy in the primary slot.
type PortalStep = 'resolve' | 'login' | 'consent' | 'callback';
type StewardPhase = 'connecting' | 'failed' | 'unreachable';
type DevicePage = 'live' | 'preview';

/**
 * What the shell's header may say, from what this page actually knows:
 * - `own`: the node answering holds the person's key (it answered in its own
 *   identity words, or the session is a peer-conductor one);
 * - `will-hold`: no identity here yet; beginning makes the key on this device;
 * - `default`: a doorway hosts this person (discovered from /auth/me) — the
 *   shell's own header says so;
 * - `none`: nothing is known — say nothing rather than something false.
 */
type HeaderKind = 'own' | 'will-hold' | 'default' | 'none';

/** This host's header words. */
const HEADER_WORDS = {
  own: { ownNodeLabel: 'Your own device holds your key' },
  // True even when the page is open on another machine: the node serving it
  // is the one that makes and keeps the key.
  'will-hold': { ownNodeLabel: 'Your own device will make and keep your key' },
} as const;

/**
 * Which device approval page a path names, if any: `consent/device` under the
 * portal's base, or its development-only `consent/device/preview`.
 */
export function devicePageFor(pathname: string): DevicePage | null {
  const path = pathname.replace(/\/+$/, '');
  if (path.endsWith('/consent/device')) return 'live';
  if (path.endsWith('/consent/device/preview')) return 'preview';
  return null;
}

/** The development-only identity preview: `identity/preview` under the portal's base. */
export function identityPreviewFor(pathname: string): boolean {
  return pathname.replace(/\/+$/, '').endsWith('/identity/preview');
}

@Component({
  selector: 'imagodei-portal-root',
  standalone: true,
  imports: [
    CommonModule,
    DeviceApprovalComponent,
    IdentityViewComponent,
    PendingAsksComponent,
    SignInViewComponent,
  ],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  // OnPush-unsafe: ROOT view — an OnPush root is never marked dirty, so a global
  // ApplicationRef tick skips it and freezes change detection for the whole
  // non-signal tree. See backlog-onpush-eager-debt-inventory.
  changeDetection: ChangeDetectionStrategy.Eager,
  template: `
    <main>
      <h1 class="visually-hidden">Elohim Portal</h1>
      <elohim-imagodei-portal-shell #shell [attr.step]="step()" [authority]="authority()">
        <!-- The header says only what this page knows about who holds the
             key, in this host's words; with nothing known it says nothing,
             and it never shows a witness line the page below would contradict. -->
        <ng-container *ngIf="headerWords() as words">
          <elohim-imagodei-trust-indicator
            slot="header"
            data-testid="portal-trust"
            trust-mode="peer-conductor"
            [strings]="words"
          ></elohim-imagodei-trust-indicator>
        </ng-container>
        <span
          *ngIf="headerKind() === 'none'"
          slot="header"
          data-testid="portal-header-empty"
        ></span>

        <ng-container *ngIf="mode() === 'identity'">
          <imagodei-portal-identity-view
            slot="primary"
            [state]="identityState()"
            [returnNote]="identityReturnNote()"
            [canSignOut]="identityState().phase === 'standing'"
            [signingOut]="signInState().phase === 'signing-out'"
            (begin)="onIdentityBegin($event)"
            (signOut)="onSignOut()"
          ></imagodei-portal-identity-view>
          <imagodei-portal-pending-asks
            *ngIf="identityState().phase === 'standing'"
            slot="primary"
            class="portal-pending"
            [client]="pendingClient()"
            [identity]="identityState()"
            (signInNeeded)="onSignInNeeded()"
          ></imagodei-portal-pending-asks>
        </ng-container>

        <ng-container *ngIf="mode() === 'sign-in' || mode() === 'secret-unset'">
          <imagodei-portal-sign-in-view
            slot="primary"
            [state]="signInState()"
            [page]="mode() === 'secret-unset' ? 'secret-unset' : 'form'"
            [returnNote]="identityReturnNote()"
            (signIn)="onSignIn($event)"
          ></imagodei-portal-sign-in-view>
        </ng-container>

        <ng-container *ngIf="mode() === 'login' && step() === 'resolve' && !identityPending()">
          <elohim-imagodei-federated-resolver
            #resolver
            slot="primary"
            remember-key="elohim_auth_identifier"
            (resolved)="onResolved($event)"
            (resolve-error)="onResolveError($event)"
          >
            <span slot="help-text">
              Sign in with your federated identifier — for example, matthew&#64;alpha.elohim.host.
            </span>
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
              >
                Return to your doorway
              </a>
            </ng-container>
          </div>
        </ng-container>

        <ng-container *ngIf="mode() === 'device-consent'">
          <imagodei-portal-device-approval
            *ngIf="devicePage() === 'live'"
            slot="primary"
            (identityChange)="identityState.set($event)"
          ></imagodei-portal-device-approval>
          <div *ngIf="devicePreview() as preview" slot="primary">
            <ng-container *ngComponentOutlet="preview"></ng-container>
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
  styles: [
    `
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

      /* The pending-asks panel follows the identity panel, set apart from it. */
      .portal-pending {
        display: block;
        margin-block-start: 1.25rem;
        padding-block-start: 1rem;
        border-block-start: 1px solid color-mix(in srgb, currentColor 18%, transparent);
      }

      .steward-login__return {
        align-self: center;
        color: LinkText;
      }
    `,
  ],
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

  /** What the person's identity rests on, at the node that served this page. */
  private readonly identityController = new IdentityStandingController({
    // Beginning makes this node sign: it carries this browser's session proof.
    client: createIdentityStandingClient({ prove: () => sessionKeys().current() }),
    onChange: state => this.identityState.set(state),
  });
  identityState = signal<IdentityPageState>(this.identityController.state);
  /** Signing in to this node from this browser, and signing out. */
  private readonly signInController = new SignInController({
    client: createNodeSignInClient(),
    onChange: state => this.signInState.set(state),
  });
  signInState = signal<SignInPageState>(this.signInController.state);
  /** Devices asking over the private network; deciding one carries the session proof. */
  pendingClient = signal<PendingAsksClient>(
    createPendingAsksClient({ prove: () => sessionKeys().current() })
  );

  /** True while the root is still asking the node about the identity. */
  identityPending = signal(false);
  /** Where a begin at the root will bring the person back to, if anywhere. */
  identityReturnNote = signal<string | undefined>(undefined);

  /** What the header may say; see {@link HeaderKind}. */
  readonly headerKind = computed<HeaderKind>(() => {
    const mode = this.mode();
    const identity = this.identityState();
    if (mode === 'sign-in' || mode === 'secret-unset') return 'own';
    if (mode === 'device-consent') {
      return identity.phase === 'begin' || identity.phase === 'beginning' ? 'will-hold' : 'own';
    }
    if (mode === 'identity') {
      if (identity.phase === 'begin' || identity.phase === 'beginning') return 'will-hold';
      return identity.standing?.thisNodeIsController ? 'own' : 'none';
    }
    // The node answered in its own identity words: it is a person's own node.
    if (['standing', 'begin', 'not-signed-in', 'refused'].includes(identity.phase)) return 'own';
    const trustMode = this.authority()?.trustMode;
    if (trustMode === 'peer-conductor') return 'own';
    if (trustMode === 'doorway-host') return 'default';
    return 'none';
  });
  readonly headerWords = computed(() => {
    const kind = this.headerKind();
    return kind === 'own' || kind === 'will-hold' ? HEADER_WORDS[kind] : null;
  });

  /** Which device approval page this is, when the path names one. */
  devicePage = signal<DevicePage | null>(null);
  /** The development-only preview, loaded on demand (absent from production builds). */
  devicePreview = signal<Type<unknown> | null>(null);

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

    // 0) Device approval. The page talks only to this node; no hand-off or
    //    OAuth consent rides on its URL.
    const devicePage = readStewardHandoff(search) ? null : devicePageFor(window.location.pathname);
    if (devicePage === 'live') {
      this.mode.set('device-consent');
      this.step.set('consent');
      this.devicePage.set('live');
      return;
    }
    if (devicePage === 'preview' && (typeof ngDevMode === 'undefined' || ngDevMode)) {
      // Optimized builds replace ngDevMode with false: this branch, its import
      // and the preview chunk are removed, so no sample-data page exists on a
      // live node (the path then falls through to the ordinary flow).
      this.mode.set('device-consent');
      this.step.set('consent');
      const preview = await import('./device-approval/device-approval-preview.component');
      this.devicePreview.set(preview.DeviceApprovalPreviewComponent);
      return;
    }

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
        this._stewardEffects()
      );
      if (outcome.status === 'authenticated') {
        // Signed in on the way to another page of this portal: go straight back.
        if (this._returnAfterSignIn(search)) return;
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
    if (this.mode() !== 'login') return;

    // 3) Everything else: what does the person's identity rest on here?
    if (
      identityPreviewFor(window.location.pathname) &&
      (typeof ngDevMode === 'undefined' || ngDevMode)
    ) {
      // Development only, removed from optimized builds like the approval preview.
      const preview = await import('./identity/identity-preview');
      this.identityState.set(preview.previewIdentityState(search));
      this.signInState.set(preview.previewSignInState(search));
      this.pendingClient.set(preview.previewPendingClient(search));
      this._showRoot(rootPageFor(this.identityState(), preview.previewRootFlags(search)));
      return;
    }
    await this._readIdentity(search);
  }

  /**
   * Ask the node what the person's identity rests on. With none yet, offer
   * to begin it here; with one, say what it rests on. Anything else — no one
   * signed in, a host that does not answer — leaves the sign-in as it was.
   */
  private async _readIdentity(search: string): Promise<void> {
    this.identityPending.set(true);
    let keyLost = false;
    try {
      await this.identityController.read();
      keyLost = await sessionKeys()
        .lost()
        .catch(() => false);
    } finally {
      this.identityPending.set(false);
    }
    const params = new URLSearchParams(search);
    if (params.has(RETURN_TO_PARAM)) {
      this.identityReturnNote.set('Then you’ll go straight back to where you were.');
    }
    this._showRoot(
      rootPageFor(this.identityState(), { signInAsked: params.has(SIGN_IN_PARAM), keyLost })
    );
  }

  /** Put the root on the page the node's answer calls for; the old sign-in otherwise. */
  private _showRoot(page: RootPage): void {
    if (page === 'legacy') {
      this.mode.set('login');
      return;
    }
    this.mode.set(page);
  }

  /** Begin the identity on this device, then go back to where the person came from. */
  async onIdentityBegin({
    displayName,
    secret,
  }: {
    displayName: string;
    secret?: string;
  }): Promise<void> {
    if (await this.identityController.begin(displayName, secret)) {
      this._returnAfterSignIn(window.location.search);
    }
  }

  /**
   * Sign in to this node with the word and secret; then straight back to
   * where the person came from, or the signed-in root. Nothing they were
   * approving is resent: they approve again themselves.
   */
  async onSignIn({
    identifier,
    password,
  }: {
    identifier: string;
    password: string;
  }): Promise<void> {
    const search = window.location.search;
    const asked = new URLSearchParams(search).get(RETURN_TO_PARAM);
    const returnTo = asked ? (safePortalReturn(asked, portalBase()) ?? undefined) : undefined;
    const ok = await this.signInController.signIn({
      identifier,
      password,
      remember: true,
      ...(returnTo ? { returnTo } : {}),
    });
    if (!ok || this._returnAfterSignIn(search)) return;
    // Signed in at the root: show what the identity rests on, without the sign-in ask.
    const params = new URLSearchParams(search);
    params.delete(SIGN_IN_PARAM);
    const rest = params.toString();
    window.history.replaceState(
      window.history.state,
      '',
      window.location.pathname + (rest ? `?${rest}` : '')
    );
    await this._readIdentity(rest ? `?${rest}` : '');
  }

  /** Sign out of this node; this browser's key is forgotten with it. */
  async onSignOut(): Promise<void> {
    await this.signInController.signOut();
    await this._readIdentity('');
  }

  /** A pending ask's decision asked for sign-in: the root's form, then back here. */
  onSignInNeeded(): void {
    this.mode.set('sign-in');
  }

  /**
   * After a sign-in here, go back to the page of this portal that sent the
   * person (`?return_to=`), if any. Returns true when it navigated.
   */
  private _returnAfterSignIn(search: string): boolean {
    const asked = new URLSearchParams(search).get(RETURN_TO_PARAM);
    if (!asked) return false;
    const back = safePortalReturn(asked, portalBase());
    if (!back) return false;
    window.location.assign(back);
    return true;
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
    const detail = (
      e as CustomEvent<{
        identifier: string;
        password: string;
        remember: boolean;
      }>
    ).detail;
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
      const card = this.loginCardRef?.nativeElement as unknown as
        | Record<string, unknown>
        | undefined;
      if (typeof card?.['setError'] === 'function') {
        (card['setError'] as (msg: string) => void)(out.error);
      }
    } else if (out.redirect) {
      window.location.href = out.redirect;
    } else {
      this._returnAfterSignIn(window.location.search);
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
      const result = (await resp.json()) as { redirect: string };
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
      const result = (await resp.json()) as { redirect: string };
      window.location.href = result.redirect;
    } catch (err) {
      this.errorMessage.set(err instanceof Error ? err.message : 'consent decline failed');
    }
  }
}
