/**
 * DoorwaySessionClient — framework-free session client for the doorway auth surface.
 *
 * Consolidates the hand-rolled auth walking previously duplicated across
 * elohim-app's AuthService, a2o's DoorwayClient, and a2o's BrowserDevice
 * (arc plan 2026-06-10 Phase 1 Task 1.1; SDK home per elohim-sdk.md §3.4).
 *
 * Wire contract — source of truth is
 * doorway/doorway-service/src/routes/auth_routes.rs:
 * - POST /auth/register          → AuthResponse (201)
 * - POST /auth/login             → AuthResponse
 * - POST /auth/logout            → SuccessResponse
 * - POST /auth/refresh           → AuthResponse        (bearer)
 * - GET  /auth/me                → MeResponse          (bearer)
 * - GET  /auth/session-token     → SessionTokenResponse (bearer)
 * - GET  /auth/exchange-session  → ExchangeSessionResponse (?session_token=…)
 *
 * KEY RULE: the Rust structs serialize `#[serde(rename_all = "camelCase")]`,
 * so camelCase arrives on the wire. This client performs NO case conversion,
 * NO toWire/fromWire mapping, and NO JSON.parse of nested fields — the parsed
 * response body IS the typed value.
 *
 * Framework-free: no Angular imports, no DI decorators. Node 22 and browsers
 * both provide `globalThis.fetch`; tests inject a mock `fetchImpl` and the
 * default token store is in-memory, so no localStorage is required.
 */
import type { AuthResponse } from '../generated/auth-response.js';
import type { ExchangeSessionResponse } from '../generated/exchange-session-response.js';
import type { MeResponse } from '../generated/me-response.js';
import type { SessionTokenResponse } from '../generated/session-token-response.js';

// =============================================================================
// Request types (mirror auth_routes.rs deserialize structs)
// =============================================================================

/**
 * POST /auth/register body (auth_routes.rs `RegisterRequest`, camelCase wire).
 * Only `identifier` and `password` are required; everything else is
 * serde-defaulted server-side.
 */
export interface RegisterRequest {
  identifier: string;
  password: string;
  /** Holochain human ID (optional for doorway-hosted registration). */
  humanId?: string;
  /** Holochain agent public key (optional for doorway-hosted registration). */
  agentPubKey?: string;
  /** Defaults to 'email' server-side. */
  identifierType?: string;
  /** Display name for doorway-hosted registration (used to create identity). */
  displayName?: string;
  bio?: string;
  affinities?: string[];
  /** Profile visibility (public, connections, private); defaults to 'public'. */
  profileReach?: string;
  location?: string;
  /** Bootstrap key to grant Admin permission on registration. */
  adminBootstrapKey?: string;
  /** Agency phase: doorway, node, device, hosted, visitor. */
  agencyPhase?: string;
}

/** POST /auth/login body (auth_routes.rs `LoginRequest`). */
export interface LoginRequest {
  identifier: string;
  password: string;
}

// =============================================================================
// Response types — re-exported from the schema-generated wire shapes.
//
// The JSON Schemas in elohim/sdk/schemas/v1/views/ ({auth,me,exchange-session,
// session-token,account}-response.schema.json) are the contract; the Rust
// structs in auth_routes.rs are validated against them by
// doorway/doorway-service/tests/schema_contract.rs, and `pnpm run
// schema:codegen:ts` distributes the generated interfaces to ../generated/.
// Hand-matching the shapes here is therefore no longer possible to drift:
// these are the SAME types the schema declares.
// =============================================================================

export type { AuthResponse, HumanProfileResponse } from '../generated/auth-response.js';
export type { AuthorityRef, MeResponse } from '../generated/me-response.js';
export type { SessionTokenResponse } from '../generated/session-token-response.js';
export type { ExchangeSessionResponse } from '../generated/exchange-session-response.js';
export type { AccountResponse } from '../generated/account-response.js';

/** POST /auth/logout response (auth_routes.rs `SuccessResponse`). */
export interface SuccessResponse {
  success: boolean;
  message: string;
}

// =============================================================================
// Session persistence
// =============================================================================

/** The session identity this client persists between calls. */
export interface StoredSession {
  token: string;
  humanId: string;
  agentPubKey: string;
  identifier: string;
  /** Unix timestamp in seconds (the JWT `exp` claim). */
  expiresAt: number;
  doorwayId?: string;
  doorwayUrl?: string;
  installedAppId?: string;
}

/**
 * Minimal injected persistence seam. The default is in-memory (Node tests need
 * no localStorage); an Angular adapter can wrap localStorage later.
 */
export interface SessionTokenStore {
  get(): StoredSession | null;
  set(session: StoredSession): void;
  clear(): void;
}

/** Default per-instance store — nothing shared, nothing persisted. */
export class InMemorySessionTokenStore implements SessionTokenStore {
  private session: StoredSession | null = null;

  get(): StoredSession | null {
    return this.session;
  }

  set(session: StoredSession): void {
    this.session = session;
  }

  clear(): void {
    this.session = null;
  }
}

// =============================================================================
// Error type
// =============================================================================

/**
 * Typed error for non-2xx doorway responses. `message` carries the wire
 * `error` field when the body is the doorway JSON envelope
 * (`{ error, code? }`), otherwise the raw body text.
 */
export class DoorwaySessionError extends Error {
  constructor(
    /** HTTP status code of the failed response. */
    readonly status: number,
    message: string,
    /** Doorway machine-readable error code (e.g. 'INVALID_CREDENTIALS'). */
    readonly code?: string
  ) {
    super(message);
    this.name = 'DoorwaySessionError';
  }
}

async function errorFromResponse(res: Response): Promise<DoorwaySessionError> {
  const text = await res.text();
  const trimmed = text.trim();
  let message = trimmed || `HTTP ${res.status}`;
  let code: string | undefined;
  try {
    const envelope = JSON.parse(trimmed) as { error?: unknown; code?: unknown };
    if (typeof envelope?.error === 'string') {
      message = envelope.error;
      code = typeof envelope.code === 'string' ? envelope.code : undefined;
    }
  } catch {
    // Non-JSON body (proxy error page, etc.) — keep the raw text message.
  }
  return new DoorwaySessionError(res.status, message, code);
}

// =============================================================================
// Client
// =============================================================================

export interface DoorwaySessionClientOptions {
  /** Doorway origin, e.g. 'http://localhost:8888' or 'https://doorway.elohim.host'. */
  baseUrl: string;
  /** Defaults to `globalThis.fetch` (Node 22 + browsers). */
  fetchImpl?: typeof fetch;
  /** Defaults to a per-instance in-memory store. */
  tokenStore?: SessionTokenStore;
}

/**
 * Doorway session client.
 *
 * Non-2xx responses throw `DoorwaySessionError`. Network-level failures from
 * `fetchImpl` (DNS/TCP/abort) propagate as-is (typically `TypeError`) and are
 * NOT wrapped. `logout()` clears the stored session even when the fetch fails.
 */
export class DoorwaySessionClient {
  private readonly baseUrl: string;
  private readonly fetchImpl: typeof fetch;
  private readonly tokenStore: SessionTokenStore;

  constructor(options: DoorwaySessionClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/+$/, '');
    this.fetchImpl = options.fetchImpl ?? globalThis.fetch.bind(globalThis);
    this.tokenStore = options.tokenStore ?? new InMemorySessionTokenStore();
  }

  /** Current bearer token, or null when no session is stored. */
  get token(): string | null {
    return this.tokenStore.get()?.token ?? null;
  }

  /** POST /auth/login — authenticate and store the resulting session. */
  async login(req: LoginRequest): Promise<AuthResponse> {
    const res = await this.request<AuthResponse>('POST', '/auth/login', { body: req });
    this.storeAuth(res);
    return res;
  }

  /** POST /auth/register — create credentials and store the resulting session. */
  async register(req: RegisterRequest): Promise<AuthResponse> {
    const res = await this.request<AuthResponse>('POST', '/auth/register', { body: req });
    this.storeAuth(res);
    return res;
  }

  /**
   * POST /auth/logout — server-side no-op today (stateless JWTs); the local
   * session is cleared regardless of the server outcome.
   */
  async logout(): Promise<SuccessResponse> {
    try {
      return await this.request<SuccessResponse>('POST', '/auth/logout', { auth: true });
    } finally {
      this.tokenStore.clear();
    }
  }

  /** GET /auth/me — current user info from the stored bearer token. */
  async me(): Promise<MeResponse> {
    return this.request<MeResponse>('GET', '/auth/me', { auth: true });
  }

  /** POST /auth/refresh — mint a fresh JWT from the stored one and re-store it. */
  async refresh(): Promise<AuthResponse> {
    const res = await this.request<AuthResponse>('POST', '/auth/refresh', { auth: true });
    this.storeAuth(res);
    return res;
  }

  /**
   * GET /auth/session-token — issue a short-lived (60 s) single-use transfer
   * token for cross-app session handoff (consumed by `exchangeSessionToken`).
   */
  async requestSessionToken(): Promise<SessionTokenResponse> {
    return this.request<SessionTokenResponse>('GET', '/auth/session-token', { auth: true });
  }

  /**
   * GET /auth/exchange-session?session_token=… — exchange a transfer token for
   * a full JWT and store the resulting session. The query parameter name is
   * snake_case on the wire (auth_routes.rs handle_exchange_session).
   */
  async exchangeSessionToken(sessionToken: string): Promise<ExchangeSessionResponse> {
    const res = await this.request<ExchangeSessionResponse>(
      'GET',
      `/auth/exchange-session?session_token=${encodeURIComponent(sessionToken)}`
    );
    // installedAppId intentionally absent — ExchangeSessionResponse (auth_routes.rs:88) does not carry it.
    this.tokenStore.set({
      token: res.token,
      humanId: res.humanId,
      agentPubKey: res.agentPubKey,
      identifier: res.identifier,
      expiresAt: res.expiresAt,
      doorwayId: res.doorwayId,
      doorwayUrl: res.doorwayUrl,
    });
    return res;
  }

  /**
   * Return the stored session when unexpired, else clear it and return null.
   * Purely local — no network round-trip.
   */
  restoreSession(): StoredSession | null {
    const stored = this.tokenStore.get();
    if (!stored) {
      return null;
    }
    const nowSeconds = Math.floor(Date.now() / 1000);
    if (stored.expiresAt <= nowSeconds) {
      this.tokenStore.clear();
      return null;
    }
    return stored;
  }

  // ===========================================================================
  // Internals
  // ===========================================================================

  private storeAuth(res: AuthResponse): void {
    this.tokenStore.set({
      token: res.token,
      humanId: res.humanId,
      agentPubKey: res.agentPubKey,
      identifier: res.identifier,
      expiresAt: res.expiresAt,
      doorwayId: res.doorwayId,
      doorwayUrl: res.doorwayUrl,
      installedAppId: res.installedAppId,
    });
  }

  private async request<T>(
    method: 'GET' | 'POST',
    path: string,
    opts: { body?: unknown; auth?: boolean } = {}
  ): Promise<T> {
    const headers: Record<string, string> = {};
    if (opts.auth) {
      const token = this.token;
      if (token) {
        headers['Authorization'] = `Bearer ${token}`;
      }
    }
    let body: string | undefined;
    if (opts.body !== undefined) {
      headers['Content-Type'] = 'application/json';
      body = JSON.stringify(opts.body);
    }

    const res = await this.fetchImpl(`${this.baseUrl}${path}`, { method, headers, body });
    if (!res.ok) {
      throw await errorFromResponse(res);
    }
    return (await res.json()) as T;
  }
}
