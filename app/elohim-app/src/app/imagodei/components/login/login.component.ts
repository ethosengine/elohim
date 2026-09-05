/**
 * LoginComponent (Lit wrapper) — the app's REDIRECTOR to its doorway's portal.
 *
 * There are exactly two sign-in portals in the protocol: the doorway portal
 * (`/threshold/*`, for hosted humans) and the p2p-native portal (for stewards
 * whose own runtime holds their key). elohim-app is neither — it is a relying
 * party. So this route (a) discovers which doorway to send the human to, and
 * (b) redirects them there through OAuth. It renders NO password field and
 * posts NO credential: the one place a hosted human's password is ever seen is
 * the doorway's own origin.
 *
 * Steps:
 *   'resolve' — <elohim-imagodei-federated-resolver> collects user@host
 *   'login'   — the hand-off notice while the browser leaves for the portal
 *
 * When a doorway is ALREADY proven (a workspace origin, a configured
 * environment, a registry entry that answered for itself), there is nothing to
 * resolve and the redirect happens on init.
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

import { AUTH_IDENTIFIER_KEY } from '../../models/auth.model';
import { parseFederatedIdentifier, resolveGatewayToDoorwayUrl } from '../../models/doorway.model';
import { AuthService } from '../../services/auth.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { OAuthAuthProvider } from '../../services/providers/oauth-auth.provider';

// SIDE-EFFECT import: registers <elohim-imagodei-federated-resolver> and
// <elohim-imagodei-portal-shell>. Without it this page renders the custom
// elements' fallback text and NO identifier input, so login is impossible on a
// direct navigation to /identity/login. The registration used to arrive only
// via auth-callback.component.ts, which a first-time visitor never passes
// through, and the type-only import below is erased at compile time so it
// pulls in nothing.
import 'elohim-imagodei/register';

import type { AuthorityResolution } from 'elohim-imagodei';

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
  private readonly oauthProvider = inject(OAuthAuthProvider);
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

  ngOnInit(): void {
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

    // A doorway that has already PROVED itself (workspace origin, configured
    // environment, registry entry that answered for itself) leaves nothing to
    // resolve — send the human straight to its portal.
    const provenDoorwayUrl = this.doorwayRegistry.selectedUrl();
    if (provenDoorwayUrl) {
      this.redirectToPortal(provenDoorwayUrl, this.identifier);
      return;
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
      // eslint-disable-next-line no-restricted-syntax, @typescript-eslint/prefer-optional-chain -- SSR-safe: short-circuited by the preceding typeof check; and each clause is a separate defensive check, so collapsing them to `?.` drops one
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
    // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside typeof window guard (early return above when window is undefined)
    const origin = window.location.origin;

    try {
      const resp = await fetch(`${origin}/auth/me`, { credentials: 'include' });
      if (resp.ok) {
        const data = (await resp.json()) as Record<string, unknown>;
        const authorityData = (data['authority'] as Record<string, string> | undefined) ?? {};
        const label = (authorityData['label'] as string | undefined) ?? '';
        if (label) {
          this.authority = {
            trustMode:
              (data['trustMode'] as AuthorityResolution['trustMode'] | undefined) ?? 'doorway-host',
            authority: {
              label,
              id: authorityData['id'] as string | undefined,
            },
            flywheelHint: data['flywheelHint'] as boolean | undefined,
            attestors: data['attestors'] as AuthorityResolution['attestors'] | undefined,
          };
          return;
        }
      }
    } catch {
      // Network error — fall through to the anonymous discovery document.
    }

    // /auth/me is 401 for EVERY anonymous visitor, which is everyone reading a
    // SIGN-IN page — so the trust chip rendered "Hosted via" followed by
    // nothing. The doorway publishes who it is anonymously at
    // /.well-known/elohim-auth (doorway-service/src/routes/auth_discovery.rs);
    // that is the honest answer to "whose porch am I standing on" before a
    // session exists.
    await this._prefetchAuthorityFromDiscovery(origin);
  }

  /**
   * Anonymous fallback: name the doorway from its own discovery document.
   * The document is origin-relative by construction, so the only identity it
   * can assert is its own; if it omits `doorwayId` we fall back to the hostname
   * the human already typed or clicked.
   */
  private async _prefetchAuthorityFromDiscovery(origin: string): Promise<void> {
    // eslint-disable-next-line no-restricted-syntax -- SSR-safe: caller returns early when window is undefined
    const hostname = window.location.hostname;
    try {
      const resp = await fetch(`${origin}/.well-known/elohim-auth`);
      if (!resp.ok) return;
      const doc = (await resp.json()) as Record<string, unknown>;
      const doorwayId = (doc['doorwayId'] as string | undefined) ?? '';
      const label = doorwayId || hostname;
      if (!label) return;
      this.authority = {
        trustMode: 'doorway-host',
        authority: { label, id: doorwayId || undefined },
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
    // was selected before we hand the human over to it.
    const parsed = parseFederatedIdentifier(detail.identifier);
    // `resolveGatewayToDoorwayUrl` is a LOOKUP now and returns null for a host
    // no doorway has declared — there is nothing to select in that case, and
    // inventing one is the defect this whole path was fixed for.
    const doorwayUrl = parsed
      ? detail.doorwayUrl || resolveGatewayToDoorwayUrl(parsed.gatewayDomain)
      : null;

    if (!doorwayUrl) {
      this.errorMessage = 'No doorway serves that identifier. Check the host after the @.';
      return;
    }

    // `selectProbedDoorwayUrl`, not `selectDoorwayByUrl`: the synchronous
    // setter only accepts a doorway the app already trusts, so a host the
    // resolver element legitimately probed would be refused by it. This
    // path adopts a new doorway only after it answers for itself.
    void this.doorwayRegistry.selectProbedDoorwayUrl(doorwayUrl, true);

    this.redirectToPortal(doorwayUrl, detail.identifier);
  }

  onResolveError(e: Event): void {
    const detail = (e as CustomEvent<{ reason: string }>).detail;
    this.errorMessage = `Could not resolve: ${detail.reason}`;
  }

  // ==========================================================================
  // The hand-off
  // ==========================================================================

  /**
   * Leave for the doorway's own portal through OAuth, carrying the identifier
   * as `login_hint` so the human does not retype it.
   *
   * This is the ONLY way this route signs anyone in. There is no in-app
   * credential path to fall back to.
   */
  redirectToPortal(doorwayUrl: string, identifier = ''): void {
    const origin = this.browserOrigin();
    if (!origin) return; // SSR / non-browser render — nowhere to send anyone

    try {
      this.step = 'login';
      this.oauthProvider.storeReturnUrl(this.returnUrl);
      this.oauthProvider.initiateLogin(
        doorwayUrl,
        `${origin}/auth/callback`,
        identifier.trim() || undefined
      );
    } catch (err) {
      this.step = 'resolve';
      this.errorMessage = err instanceof Error ? err.message : 'Sign-in could not be started';
    }
  }

  // ==========================================================================
  // Private helpers
  // ==========================================================================

  /** The page origin, or null when there is no browser (SSR, prerender). */
  private browserOrigin(): string | null {
    if (typeof window === 'undefined') return null;
    // eslint-disable-next-line no-restricted-syntax -- SSR-safe: guarded by the typeof check above
    const origin = window.location?.origin;
    return origin && origin !== 'null' ? origin : null;
  }

  /**
   * Wire the Lit resolver element's `resolveIdentifier` imperative callback.
   * This callback is called by the element when it needs to validate a
   * federated identifier (user@host) and discover the doorway URL.
   */
  private wireResolverCallback(): void {
    if (!this.resolverRef) return;

    const el = this.resolverRef.nativeElement as unknown as Record<string, unknown>;
    // The Lit element awaits this imperative callback, so the promise IS the
    // contract; today's implementation happens to resolve synchronously.
    // eslint-disable-next-line @typescript-eslint/require-await -- see above
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
}
