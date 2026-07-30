/**
 * AuthCallbackComponent (Lit wrapper).
 *
 * Renders <elohim-imagodei-oauth-callback> with code + state extracted from
 * the URL query params and the exchangeCode callback wired to OAuthAuthProvider.
 *
 * All visual states (exchanging skeleton, done, error) are owned by the Lit
 * element. This Angular bridge owns:
 *   - Extracting code / state / provider from ActivatedRoute snapshot
 *   - Delegating the actual code exchange to OAuthAuthProvider.handleCallback
 *   - Calling AuthService.setAuthFromResult on success
 *   - Navigating to the stored return URL (or /lamad) after success
 *   - Logging errors surfaced by the element
 *
 * Per design spec §1.1, this is the cleanup that achieves "ZERO third portal"
 * — the Lit element is the sole visual surface; the Angular component is a
 * thin bridge.
 */

import {
  AfterViewInit,
  Component,
  CUSTOM_ELEMENTS_SCHEMA,
  ElementRef,
  OnInit,
  ViewChild,
  inject,
  signal,
  ChangeDetectionStrategy,
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import 'elohim-imagodei/register';

import { AuthService } from '../../services/auth.service';
import { OAuthAuthProvider } from '../../services/providers/oauth-auth.provider';
import { type AuthResult } from '../../models/auth.model';

@Component({
  selector: 'app-auth-callback',
  standalone: true,
  imports: [CommonModule, RouterModule],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  changeDetection: ChangeDetectionStrategy.Eager,
  template: `
    <elohim-imagodei-oauth-callback
      #cb
      [attr.code]="code()"
      [attr.state]="state()"
      [attr.provider-label]="providerLabel()"
      (success)="onSuccess($event)"
      (error)="onError($event)"
    >
      <a slot="recovery-cta" routerLink="/identity/recovery">Recover account</a>
    </elohim-imagodei-oauth-callback>
  `,
})
export class AuthCallbackComponent implements OnInit, AfterViewInit {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly oauth = inject(OAuthAuthProvider);
  private readonly authService = inject(AuthService);

  readonly code = signal('');
  readonly state = signal('');
  readonly providerLabel = signal('');

  @ViewChild('cb') cbRef?: ElementRef<HTMLElement>;

  ngOnInit(): void {
    const qp = this.route.snapshot.queryParamMap;
    this.code.set(qp.get('code') ?? '');
    this.state.set(qp.get('state') ?? '');
    this.providerLabel.set(qp.get('provider') ?? '');
  }

  ngAfterViewInit(): void {
    if (!this.cbRef) return;

    // Wire the Lit element's imperative exchangeCode callback. The element calls
    // this with (code, state) and expects Promise<{ session: unknown }>.
    (this.cbRef.nativeElement as unknown as Record<string, unknown>)['exchangeCode'] = async (
      code: string,
      state: string
    ): Promise<{ session: AuthResult }> => {
      const result = await this.oauth.handleCallback(code, state);

      if (!result.success) {
        // Clear URL params before surfacing the error through the element's
        // error state (the element catches the rejection).
        this.oauth.clearCallbackParams();
        throw new Error(result.error);
      }

      // Persist auth state before the success event fires so the rest of the
      // app is already authenticated when we navigate.
      this.authService.setAuthFromResult(result);
      this.oauth.clearCallbackParams();

      return { session: result };
    };
  }

  onSuccess(_e: Event): void {
    const returnUrl = this.oauth.consumeReturnUrl() ?? '/lamad'; // route-literal-ok: default post-auth return mount (bundle nav target), not a minted content link
    void this.router.navigate([returnUrl]);
  }

  onError(e: Event): void {
    const detail = (e as CustomEvent<{ reason: string; recoverable: boolean }>).detail;
    // eslint-disable-next-line no-console
    console.error('OAuth callback error:', detail.reason);
  }
}
