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
