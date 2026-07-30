/**
 * LoginComponent (Lit wrapper).
 *
 * The user-facing UI is rendered entirely by the Lit primitives in
 * elohim-imagodei. This component is a thin bridge that wires AuthService /
 * OAuthAuthProvider / DoorwayRegistryService into the Lit elements' callback
 * + event surfaces.
 *
 * The standalone EPR bundle (app/imagodei-portal/) renders the same UI with
 * the same UX — only the service wiring differs (plain fetch instead of
 * Angular DI).
 *
 * Per design spec §1.1, this is the cleanup that achieves "ZERO third portal"
 * — no parallel UI implementation in elohim-app.
 *
 * Steps:
 *   'resolve' — <elohim-imagodei-federated-resolver> collects user@host
 *   'login'   — <elohim-imagodei-login-card> handles password + OAuth entry
 */

import { CommonModule } from '@angular/common';
import {
  AfterViewInit,
  Component,
  CUSTOM_ELEMENTS_SCHEMA,
  ElementRef,
  OnInit,
  ViewChild,
  inject,
} from '@angular/core';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import type { AuthorityResolution } from 'elohim-imagodei';

import { AUTH_IDENTIFIER_KEY } from '../../models/auth.model';
import { parseFederatedIdentifier, resolveGatewayToDoorwayUrl } from '../../models/doorway.model';
import { type PasswordCredentials } from '../../models/auth.model';
import { AuthService } from '../../services/auth.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { IdentityService } from '../../services/identity.service';
import { OAuthAuthProvider } from '../../services/providers/oauth-auth.provider';
import { PasswordAuthProvider } from '../../services/providers/password-auth.provider';

type Step = 'resolve' | 'login';

@Component({
  selector: 'app-login',
  standalone: true,
  imports: [CommonModule, RouterModule],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  templateUrl: './login.component.html',
  styleUrls: ['./login.component.css'],
})
export class LoginComponent implements OnInit, AfterViewInit {
  private readonly authService = inject(AuthService);
  private readonly passwordProvider = inject(PasswordAuthProvider);
  private readonly oauthProvider = inject(OAuthAuthProvider);
  private readonly identityService = inject(IdentityService);
  private readonly doorwayRegistry = inject(DoorwayRegistryService);
  private readonly router = inject(Router);
  private readonly route = inject(ActivatedRoute);

  // Exposed to template for the remember-key attribute binding
  readonly authIdentifierKey = AUTH_IDENTIFIER_KEY;

  /** Pre-fetched authority resolution from `/auth/me`. Null until the fetch completes. */
  authority: AuthorityResolution | null = null;

  step: Step = 'resolve';
  identifier = '';
  flywheelHint = false;
  errorMessage = '';

  /** Return URL after successful login — read from query params */
  private returnUrl = '/';

  @ViewChild('resolver') resolverRef?: ElementRef<HTMLElement>;
  @ViewChild('loginCard') loginCardRef?: ElementRef<HTMLElement>;

  ngOnInit(): void {
    // Register password provider with auth service if not already registered
    if (!this.authService.hasProvider('password')) {
      this.authService.registerProvider(this.passwordProvider);
    }

    // Capture return URL from query params
    this.route.queryParams.subscribe(params => {
      this.returnUrl = (params['returnUrl'] as string) ?? '/';
    });

    // Already authenticated → redirect immediately
    if (this.authService.isAuthenticated()) {
      void this.router.navigate([this.returnUrl]);
      return;
    }

    // Pre-fill identifier if one was remembered
    try {
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside try/catch SSR fallback
      const stored = localStorage.getItem(AUTH_IDENTIFIER_KEY);
      if (stored) this.identifier = stored;
    } catch {
      // localStorage unavailable — degrade silently
    }

    // Pre-fetch authority from doorway so the shell element receives it as a
    // property rather than fetching itself. Failure is non-fatal — the shell
    // renders with placeholder chrome and emits authority-needed.
    void this._prefetchAuthority();
  }

  private async _prefetchAuthority(): Promise<void> {
    // Skip in non-browser environments (Vitest/Node, SSR). Node's native
    // fetch cannot resolve relative URLs and jsdom's default `about:blank`
    // origin is `'null'` — both surface as ERR_INVALID_URL.
    if (
      typeof window === 'undefined' ||
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside typeof window guard (short-circuited by the preceding typeof check)
      !window.location ||
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside typeof window guard (short-circuited by the preceding typeof check)
      !window.location.origin ||
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside typeof window guard (short-circuited by the preceding typeof check)
      window.location.origin === 'null' ||
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside typeof window guard (short-circuited by the preceding typeof check)
      !window.location.protocol.startsWith('http')
    ) {
      return;
    }
    try {
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside typeof window guard (early return above when window is undefined)
      const resp = await fetch(`${window.location.origin}/auth/me`, { credentials: 'include' });
      if (!resp.ok) return;
      const data = (await resp.json()) as Record<string, unknown>;
      const authorityData = (data['authority'] as Record<string, string> | undefined) ?? {};
      this.authority = {
        trustMode:
          (data['trustMode'] as AuthorityResolution['trustMode'] | undefined) ?? 'doorway-host',
        authority: {
          label: (authorityData['label'] as string | undefined) ?? '',
          id: authorityData['id'] as string | undefined,
        },
        flywheelHint: data['flywheelHint'] as boolean | undefined,
        attestors: data['attestors'] as AuthorityResolution['attestors'] | undefined,
      };
    } catch {
      // Network error — leave authority null; shell will emit authority-needed.
    }
  }

  ngAfterViewInit(): void {
    // Wire the resolver's async resolveIdentifier callback so the Lit element
    // can validate a federated identifier against the doorway registry without
    // knowing about Angular DI.
    this.wireResolverCallback();
  }

  // ==========================================================================
  // Lit element event handlers
  // ==========================================================================

  onResolved(e: Event): void {
    const detail = (e as CustomEvent<{ identifier: string; doorwayUrl: string }>).detail;
    this.identifier = detail.identifier;
    this.errorMessage = '';

    // Inform the doorway registry so the rest of the app knows which doorway
    // was selected before we advance to the login card.
    const parsed = parseFederatedIdentifier(detail.identifier);
    if (parsed) {
      const doorwayUrl = detail.doorwayUrl || resolveGatewayToDoorwayUrl(parsed.gatewayDomain);
      this.doorwayRegistry.selectDoorwayByUrl(doorwayUrl);
    }

    this.step = 'login';
  }

  onResolveError(e: Event): void {
    const detail = (e as CustomEvent<{ reason: string }>).detail;
    this.errorMessage = `Could not resolve: ${detail.reason}`;
  }

  async onPasswordSubmit(e: Event): Promise<void> {
    const detail = (e as CustomEvent<{ identifier: string; password: string; remember: boolean }>)
      .detail;
    const identifier = (detail.identifier || this.identifier).trim();
    this.errorMessage = '';

    const credentials: PasswordCredentials = {
      type: 'password',
      identifier,
      password: detail.password,
    };

    try {
      const result = await this.authService.login('password', credentials);

      if (result.success) {
        // Persist or clear remembered identifier
        try {
          if (detail.remember) {
            // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside try/catch SSR fallback
            localStorage.setItem(AUTH_IDENTIFIER_KEY, identifier);
          } else {
            // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside try/catch SSR fallback
            localStorage.removeItem(AUTH_IDENTIFIER_KEY);
          }
        } catch {
          // localStorage unavailable — degrade silently
        }

        // Wait for identity state before navigating so the UI reflects
        // authenticated state immediately after redirect.
        await this.identityService.waitForAuthenticatedState(3000);
        void this.router.navigate([this.returnUrl]);
      } else {
        const msg = result.error ?? 'Sign-in failed';
        this.errorMessage = msg;
        this.forwardErrorToCard(msg);
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Sign-in failed';
      this.errorMessage = msg;
      this.forwardErrorToCard(msg);
    }
  }

  async onOAuthStart(e: Event): Promise<void> {
    // The login-card emits the doorwayUrl it resolved during the resolve step.
    // Fall back to the registry's current selection when not present.
    const detail = (e as CustomEvent<{ providerId: string; doorwayUrl?: string }>).detail;
    this.errorMessage = '';

    try {
      const doorwayUrl =
        detail.doorwayUrl ?? this.doorwayRegistry.selectedUrl() ?? this.resolveCurrentDoorwayUrl();
      if (!doorwayUrl) {
        this.errorMessage = 'No doorway selected. Please resolve your identity first.';
        return;
      }

      this.oauthProvider.storeReturnUrl(this.returnUrl);
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: browser-only oauth surface, never SSR-rendered (invoked only from a user click handler)
      const callbackUrl = `${globalThis.location.origin}/auth/callback`;
      this.oauthProvider.initiateLogin(doorwayUrl, callbackUrl);
    } catch (err) {
      this.errorMessage = err instanceof Error ? err.message : 'OAuth flow failed to start';
    }
  }

  // ==========================================================================
  // Private helpers
  // ==========================================================================

  /**
   * Wire the Lit resolver element's `resolveIdentifier` imperative callback.
   * This callback is called by the element when it needs to validate a
   * federated identifier (user@host) and discover the doorway URL.
   */
  private wireResolverCallback(): void {
    if (!this.resolverRef) return;

    const el = this.resolverRef.nativeElement as unknown as Record<string, unknown>;
    el['resolveIdentifier'] = async (id: string) => {
      try {
        const parsed = parseFederatedIdentifier(id);
        if (!parsed) {
          return { ok: false, reason: 'invalid-identifier' };
        }
        const doorwayUrl = resolveGatewayToDoorwayUrl(parsed.gatewayDomain);
        return { ok: true, doorwayUrl };
      } catch (err) {
        return { ok: false, reason: err instanceof Error ? err.message : 'resolve-failed' };
      }
    };
  }

  /**
   * Derive doorway URL from the current identifier as a last resort.
   * Returns null when the identifier hasn't been resolved yet.
   */
  private resolveCurrentDoorwayUrl(): string | null {
    if (!this.identifier) return null;
    const parsed = parseFederatedIdentifier(this.identifier);
    return parsed ? resolveGatewayToDoorwayUrl(parsed.gatewayDomain) : null;
  }

  /**
   * Forward an error message to the login-card Lit element's `setError`
   * imperative method, if the element exposes it.
   */
  private forwardErrorToCard(msg: string): void {
    const card = this.loginCardRef?.nativeElement as unknown as { setError?: (m: string) => void };
    card?.setError?.(msg);
  }
}
