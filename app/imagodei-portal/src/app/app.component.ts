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
import type { AuthorityResolution } from 'elohim-imagodei';

type PortalMode = 'login' | 'consent';
type PortalStep = 'resolve' | 'login' | 'consent' | 'callback';

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
  `],
})
export class AppComponent implements OnInit, AfterViewInit {
  private readonly resolverService = new StandaloneResolver();

  mode = signal<PortalMode>('login');
  step = signal<PortalStep>('resolve');
  identifier = signal<string>('');
  consentCtx = signal<ConsentContext | null>(null);
  errorMessage = signal<string>('');

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

    const params = new URLSearchParams(window.location.search);
    const isConsent = params.has('client_id') && params.has('claims');

    if (isConsent) {
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
  }

  private async _prefetchAuthority(): Promise<void> {
    try {
      const resp = await fetch('/auth/me', { credentials: 'include' });
      if (!resp.ok) return;
      const data = (await resp.json()) as Record<string, unknown>;
      const authorityData = (data['authority'] as Record<string, string> | undefined) ?? {};
      this.authority.set({
        trustMode: (data['trustMode'] as AuthorityResolution['trustMode'] | undefined) ?? 'doorway-host',
        authority: {
          label: (authorityData['label'] as string | undefined) ?? '',
          id: authorityData['id'] as string | undefined,
        },
        flywheelHint: data['flywheelHint'] as boolean | undefined,
        attestors: data['attestors'] as AuthorityResolution['attestors'] | undefined,
      });
    } catch {
      // Network error — leave authority null; shell will emit authority-needed.
    }
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
