/**
 * StandaloneResolver — plain-fetch HTTP service for the standalone EPR bundle.
 *
 * The Angular-DI counterpart (AuthService, OAuthAuthProvider, etc.) does the
 * same thing inside elohim-app via injected services. The standalone portal
 * bundle can't depend on Angular services, so it uses this directly. The Lit
 * elements consume identical callback shapes either way.
 *
 * Service gravity: This is a UX-surface service — it mediates between URL
 * state and Lit element callbacks. The doorway owns actual auth truth (session
 * creation, token issuance, consent decisions). This class is a thin fetch
 * adapter, not a business-logic layer.
 */

import type { AuthorityResolution } from 'elohim-imagodei';

/**
 * Structural mirror of `LocalSessionView`
 * (elohim/sdk/storage-client-ts/src/generated/LocalSessionView.ts) — the 201
 * body of `POST /session/exchange`. Declared locally because the standalone
 * portal does not depend on `@elohim/storage-client`; the shape is owned by
 * Rust (ts-rs) and is read-only here. Session identity (`id`, `humanId`,
 * `agentPubKey`) is assigned by the backend — the portal never generates it.
 */
export interface LocalSessionView {
  id: string;
  humanId: string;
  agentPubKey: string;
  doorwayUrl: string;
  doorwayId: string | null;
  identifier: string;
  displayName: string | null;
  profileImageHash: string | null;
  isActive: boolean;
  createdAt: string;
  updatedAt: string;
  lastSyncedAt: string | null;
  bootstrapUrl: string | null;
}

export interface SessionTokenOutcome {
  ok: boolean;
  /** HTTP status when the server answered (absent on a network/throw). */
  status?: number;
  /** The redeemed local session, present only on a 201. */
  session?: LocalSessionView;
}

export interface ResolveOutcome {
  ok: boolean;
  doorwayUrl?: string;
  reason?: string;
}

export interface LoginInput {
  identifier: string;
  password: string;
  remember: boolean;
}

export interface LoginOutcome {
  redirect?: string;
  error?: string;
}

export interface ConsentRequest {
  clientId: string;
  claims: string[];
  redirectUri: string;
  state: string;
}

export interface ConsentContext {
  requestingClient: { id: string; displayName: string; brandMark?: string };
  requestedClaims: { id: string; label: string; description?: string }[];
  requiredClaims: string[];
}

export class StandaloneResolver {
  /**
   * Resolves a federated identifier (user@gateway-host) by probing the
   * gateway's /healthz endpoint. If reachable, returns the doorway base URL.
   *
   * The doorway owns the identity resolution truth — this is a connectivity
   * probe, not an authoritative lookup.
   */
  async resolveIdentifier(identifier: string): Promise<ResolveOutcome> {
    const at = identifier.indexOf('@');
    if (at < 1 || at === identifier.length - 1) {
      return { ok: false, reason: 'format' };
    }
    const gatewayHost = identifier.slice(at + 1);
    if (!gatewayHost) {
      return { ok: false, reason: 'format' };
    }
    const probeUrl = `https://${gatewayHost}/healthz`;
    try {
      const resp = await fetch(probeUrl, { credentials: 'omit' });
      if (!resp.ok) {
        return { ok: false, reason: `http-${resp.status}` };
      }
      return { ok: true, doorwayUrl: `https://${gatewayHost}` };
    } catch (e) {
      return { ok: false, reason: e instanceof Error ? e.message : 'fetch-failed' };
    }
  }

  /**
   * Submits credentials to /auth/login. The doorway validates and returns a
   * redirect path on success, or an error message on failure.
   */
  async loginWithPassword(input: LoginInput): Promise<LoginOutcome> {
    const resp = await fetch('/auth/login', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify(input),
    });
    if (!resp.ok) {
      let error = `http-${resp.status}`;
      try {
        const body = await resp.json() as { error?: string };
        if (body.error) error = body.error;
      } catch {
        // body may not be JSON — keep the http-status fallback
      }
      return { error };
    }
    return await resp.json() as LoginOutcome;
  }

  /**
   * Consumes a doorway→steward portal handoff. The doorway issued an opaque,
   * single-use `sessionToken` and redirected the browser here carrying it plus
   * the issuer's origin (`doorwayUrl`). We redeem it at this steward's OWN
   * storage backend (same-origin relative URL — never the issuer), which
   * verifies the token against the issuer, mints a local peer-native session,
   * and sets an httpOnly session cookie.
   *
   * The doorway owns token issuance; this steward's storage owns redemption and
   * trust adjudication (403 when the issuer isn't trusted). This method only
   * carries the values across the wire and reports the outcome — it computes no
   * trust truth of its own.
   *
   * Returns `{ ok: true, session }` on 201; `{ ok: false, status }` on
   * 401 (redeem failed / expired / replayed) or 403 (issuer not trusted);
   * `{ ok: false }` with no status on a network error.
   */
  async loginWithSessionToken(
    sessionToken: string,
    doorwayUrl: string
  ): Promise<SessionTokenOutcome> {
    try {
      const resp = await fetch('/session/exchange', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({ sessionToken, doorwayUrl }),
      });
      if (resp.status !== 201) {
        return { ok: false, status: resp.status };
      }
      const session = (await resp.json()) as LocalSessionView;
      return { ok: true, status: resp.status, session };
    } catch {
      // Network error — conductor/storage unreachable. No status to report.
      return { ok: false };
    }
  }

  /**
   * Discovers the active session's authority by calling `GET /auth/me`.
   * `trustMode` is READ from the response, never configured — the same bundle
   * runs in both doorway-host and peer-conductor modes and learns which from
   * the wire. Returns null when unauthenticated (401) or unreachable, leaving
   * the shell to render placeholder chrome / emit `authority-needed`.
   */
  async fetchAuthority(): Promise<AuthorityResolution | null> {
    try {
      const resp = await fetch('/auth/me', { credentials: 'include' });
      if (!resp.ok) return null;
      const data = (await resp.json()) as Record<string, unknown>;
      const authorityData = (data['authority'] as Record<string, string> | undefined) ?? {};
      return {
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
      return null;
    }
  }

  /**
   * Exchanges an OAuth authorization code for a session. Throws on failure
   * so callers can handle with a single try/catch.
   */
  async exchangeCode(code: string, state: string): Promise<{ session: unknown }> {
    const resp = await fetch('/auth/token', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify({ code, state }),
    });
    if (!resp.ok) {
      throw new Error(`exchange failed: ${resp.status}`);
    }
    return await resp.json() as { session: unknown };
  }

  /**
   * Prepares the consent context for an OAuth authorize request. The doorway
   * resolves the client registration and computes which claims are required vs
   * optional; this method surfaces that for the consent-card Lit element.
   */
  async prepareConsent(req: ConsentRequest): Promise<ConsentContext> {
    const resp = await fetch('/auth/authorize/prepare', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify(req),
    });
    if (!resp.ok) {
      throw new Error(`prepare failed: ${resp.status}`);
    }
    return await resp.json() as ConsentContext;
  }
}
