/**
 * Handoff Service — browser-side session token exchange.
 *
 * When doorway redirects the browser to /account?session_token=xxx&doorway_url=yyy
 * this service exchanges the short-lived session token for a full JWT by calling
 * {doorwayUrl}/auth/exchange-session, then sets auth state via AuthService.
 *
 * This is the browser equivalent of TauriAuthService's OAuth handoff flow.
 * It lives in the account pillar because it is triggered by the accountGuard
 * during route activation, not during the login flow itself.
 *
 * Sense-and-respond: the browser detects the handoff token in the URL (sensing);
 * AuthService updates auth state (responding). No domain logic lives here.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import { AuthService } from '@app/imagodei';

// ---------------------------------------------------------------------------
// Wire shape returned by /auth/exchange-session
// ---------------------------------------------------------------------------

interface ExchangeSessionResponse {
  /** Full JWT replacing the short-lived session token. */
  token: string;
  humanId: string;
  agentPubKey: string;
  /** Human-readable identifier (email or username). */
  identifier: string;
  /** Unix timestamp (seconds) or ISO string. */
  expiresAt: number | string;
  doorwayUrl?: string;
  portalHostUrl?: string;
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

@Injectable({ providedIn: 'root' })
export class HandoffService {
  private readonly http = inject(HttpClient);
  private readonly auth = inject(AuthService);

  /**
   * Exchange a doorway-issued session token for a full auth session.
   *
   * @param token      Short-lived session_token from query param.
   * @param doorwayUrl Base URL of the doorway that issued the token.
   * @returns          True on success, false if exchange failed.
   */
  async consumeHandoffToken(token: string, doorwayUrl: string): Promise<boolean> {
    try {
      const resp = await firstValueFrom(
        this.http.get<ExchangeSessionResponse>(
          `${doorwayUrl}/auth/exchange-session?session_token=${encodeURIComponent(token)}`
        )
      );

      // setAuthFromResult is the existing AuthService API for externally-sourced
      // auth results (used by the OAuth callback flow). Provider type 'oauth'
      // is correct for hosted doorway sessions.
      this.auth.setAuthFromResult(
        {
          success: true,
          token: resp.token,
          humanId: resp.humanId,
          agentPubKey: resp.agentPubKey,
          identifier: resp.identifier,
          expiresAt: resp.expiresAt,
        },
        'oauth'
      );

      return true;
    } catch {
      // Exchange failure is non-fatal — the guard will redirect to /identity/login.
      return false;
    }
  }
}
