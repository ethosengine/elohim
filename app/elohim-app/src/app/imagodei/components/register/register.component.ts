/**
 * RegisterComponent — the app's REDIRECTOR to its doorway's registration page.
 *
 * Apps are relying parties, never portals. Creating an account happens at the
 * doorway's own portal (`/threshold/register`), reached by asking that
 * doorway's OAuth `authorize` endpoint with `prompt=create`; the human returns
 * here through the ordinary `/auth/callback`, already signed in.
 *
 * So this route renders no form, collects no password, and posts no
 * registration. It exists only so that every existing "Join Network" /
 * "create an account" link keeps working.
 *
 * Profile is not auth: bio, interests, location and reach are set on the
 * profile surface AFTER sign-in — a visitor's session is applied to the fresh
 * profile by the auth callback, not collected here.
 */

import { Component, OnInit, inject, signal } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';

import { AUTH_IDENTIFIER_KEY } from '../../models/auth.model';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { OAuthAuthProvider } from '../../services/providers/oauth-auth.provider';

@Component({
  selector: 'app-register',
  standalone: true,
  imports: [],
  template: `
    <p class="handoff-note" role="status" data-testid="register-handoff">
      {{ message() }}
    </p>
  `,
  styles: [
    `
      :host {
        display: block;
        padding: 2rem;
      }

      .handoff-note {
        margin: 0;
        text-align: center;
      }
    `,
  ],
})
export class RegisterComponent implements OnInit {
  private readonly doorwayRegistry = inject(DoorwayRegistryService);
  private readonly oauthProvider = inject(OAuthAuthProvider);
  private readonly router = inject(Router);
  private readonly route = inject(ActivatedRoute);

  /** What the human reads during the hand-off (or if it cannot happen). */
  readonly message = signal('Taking you to your doorway to create your account…');

  /** Where to send the human back to once the doorway returns them. */
  private returnUrl = '/';

  ngOnInit(): void {
    this.route.queryParams.subscribe(params => {
      this.returnUrl = (params['returnUrl'] as string) ?? '/';
    });

    const doorwayUrl = this.resolveDoorwayUrl();
    if (!doorwayUrl) {
      // No doorway has proved itself yet — the login route owns discovery, and
      // it will bring the human straight back here once a doorway is chosen.
      this.message.set('Choose your doorway first…');
      void this.router.navigate(['/identity/login'], {
        queryParams: { returnUrl: this.returnUrl },
      });
      return;
    }

    this.redirectToPortalRegistration(doorwayUrl);
  }

  /**
   * The doorway to register with — one that has already PROVED itself, exactly
   * as `/identity/login` resolves it (a configured origin, a registry entry, or
   * a host that answered its own `/.well-known/elohim-auth`). Never a host
   * derived from typed input: probing that is the login resolver's job, and
   * a second, weaker resolution path here is how the two would drift apart.
   */
  private resolveDoorwayUrl(): string | null {
    return this.doorwayRegistry.selectedUrl();
  }

  /** Leave for the doorway's registration page through OAuth `prompt=create`. */
  private redirectToPortalRegistration(doorwayUrl: string): void {
    const origin = this.browserOrigin();
    if (!origin) return; // SSR / non-browser render — nowhere to send anyone

    try {
      this.oauthProvider.storeReturnUrl(this.returnUrl);
      this.oauthProvider.initiateRegistration(
        doorwayUrl,
        `${origin}/auth/callback`,
        this.rememberedIdentifier()
      );
    } catch (err) {
      this.message.set(
        err instanceof Error
          ? `Could not reach your doorway: ${err.message}`
          : 'Could not reach your doorway.'
      );
    }
  }

  /** A previously typed identifier, offered to the portal as `login_hint`. */
  private rememberedIdentifier(): string | undefined {
    try {
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside try/catch SSR fallback
      return localStorage.getItem(AUTH_IDENTIFIER_KEY) ?? undefined;
    } catch {
      return undefined;
    }
  }

  /** The page origin, or null when there is no browser (SSR, prerender). */
  private browserOrigin(): string | null {
    if (typeof window === 'undefined') return null;
    // eslint-disable-next-line no-restricted-syntax -- SSR-safe: guarded by the typeof check above
    const origin = window.location?.origin;
    return origin && origin !== 'null' ? origin : null;
  }
}
