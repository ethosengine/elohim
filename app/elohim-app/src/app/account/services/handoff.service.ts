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

// Wire shapes (/auth/exchange-session, /auth/session-token) are the
// schema-contract-pinned generated contracts re-exported by @elohim/identity
// (auth-wire plan Task 4 — local hand-written duplicates retired).
import type { ExchangeSessionResponse, SessionTokenResponse } from '@elohim/identity';

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

  /**
   * Mint a single-use session-transfer code from the doorway's existing
   * GET /auth/session-token endpoint (Bearer-authenticated, 60s TTL, consumed
   * exactly once by the receiving portal's back-channel redeem).
   *
   * Used by the "Manage from your steward →" redirect: the code rides the
   * redirect URL instead of the JWT — the JWT must NEVER appear in a URL
   * (history/referrer/log leakage). Returns null on any failure; callers must
   * not fall back to placing the JWT on the URL.
   */
  async mintHandoffToken(): Promise<string | null> {
    const bearer = this.auth.token();
    if (!bearer) return null;
    try {
      const resp = await firstValueFrom(
        this.http.get<SessionTokenResponse>('/auth/session-token', {
          headers: { Authorization: `Bearer ${bearer}` },
        })
      );
      return resp.sessionToken || null;
    } catch {
      return null;
    }
  }
}
