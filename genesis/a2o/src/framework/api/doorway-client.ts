/**
 * Typed HTTP client for the Doorway API.
 *
 * The auth surface (register/login/logout/me/session handoff) delegates to the
 * consolidated DoorwaySessionClient from @elohim/identity — the single home
 * for doorway auth wire types (source of truth:
 * doorway/doorway-service/src/routes/auth_routes.rs).
 *
 * Non-auth types below are derived from:
 *   - doorway/src/routes/health.rs (HealthResponse)
 */

import { DoorwaySessionClient, InMemorySessionTokenStore } from '@elohim/identity/core';
import { request } from 'undici';

import type {
  AccountResponse,
  AuthResponse,
  ExchangeSessionResponse,
  LoginRequest,
  MeResponse,
  RegisterRequest,
  SessionTokenResponse,
  StoredSession,
  SuccessResponse,
} from '@elohim/identity/core';

// Re-export the consolidated auth wire types — the identity package owns the
// definitions; a2o consumers keep importing them from this module.
export { DoorwaySessionError } from '@elohim/identity/core';
export type {
  AccountResponse,
  AuthResponse,
  ExchangeSessionResponse,
  HumanProfileResponse,
  LoginRequest,
  MeResponse,
  RegisterRequest,
  SessionTokenResponse,
  StoredSession,
  SuccessResponse,
} from '@elohim/identity/core';

export interface ConductorHealth {
  connected: boolean;
  connectedWorkers: number;
  totalWorkers: number;
  poolSize: number;
  poolsHealthy: number;
  poolsTotal: number;
}

export interface P2PHealth {
  enabled: boolean;
  peerCount: number;
  peerId?: string;
}

export interface HealthResponse {
  healthy: boolean;
  status: 'online' | 'degraded' | 'offline' | 'maintenance';
  registrationOpen: boolean;
  version: string;
  uptime: number;
  cacheEnabled: boolean;
  conductor: ConductorHealth;
  p2p?: P2PHealth;
  error?: string;
}

// ---------------------------------------------------------------------------
// Stewardship Allocation types (mirrors StewardshipAllocationView)
// ---------------------------------------------------------------------------

export interface AllocationView {
  id: string;
  contentId: string;
  stewardPresenceId: string;
  allocationRatio: number;
  allocationMethod: string;
  contributionType: string;
  governanceState: string;
  note?: string;
}

export interface ContentStewardshipView {
  contentId: string;
  allocations: AllocationView[];
  totalAllocation: number;
  hasDisputes: boolean;
  primarySteward: AllocationView | null;
}

// ---------------------------------------------------------------------------
// Contributor Presence types (mirrors ContributorPresenceView)
// ---------------------------------------------------------------------------

export interface PresenceView {
  id: string;
  displayName: string;
  presenceState: string;
  metadata?: Record<string, unknown> | null;
}

// ---------------------------------------------------------------------------
// Path types (mirrors PathView / PathWithDetailsView)
// ---------------------------------------------------------------------------

export interface PathIndexEntry {
  id: string;
  title: string;
  visibility?: string;
  participantIds?: string[];
  estimatedDuration?: string;
  tags?: string[];
}

export interface ChapterView {
  id: string;
  title: string;
  description?: string;
  order: number;
}

export interface PathWithDetailsView extends PathIndexEntry {
  description?: string;
  chapters?: ChapterView[];
  pathType?: string;
  difficulty?: string;
}

// ---------------------------------------------------------------------------
// Account types: AccountResponse is the schema-contract-pinned generated wire
// shape, imported and re-exported from @elohim/identity/core above (the
// hand-written mirror was retired — auth-wire plan Task 4).
// ---------------------------------------------------------------------------
// Status types (mirrors doorway /status endpoint)
// ---------------------------------------------------------------------------

export interface StatusResponse {
  version: string;
  uptime: number;
  conductor?: Record<string, unknown>;
  orchestrator?: Record<string, unknown>;
  [key: string]: unknown;
}

// ---------------------------------------------------------------------------
// Admin Node types (mirrors doorway /admin/nodes endpoint)
// ---------------------------------------------------------------------------

export interface AdminNodeView {
  nodeId: string;
  status: string;
  combinedScore: number;
  trustScore?: number;
  stewardTier?: string;
  lastHeartbeat?: string;
  // Hardware capacity is returned at the TOP LEVEL by GET /admin/nodes
  // (doorway-service NodeDetails) — not nested under `inventory`.
  cpuCores?: number;
  memoryGb?: number;
  storageTb?: number;
  bandwidthMbps?: number;
  region?: string;
}

export interface AdminNodesResponse {
  total: number;
  nodes: AdminNodeView[];
}

// ---------------------------------------------------------------------------
// Admin Conductor types (mirrors doorway /admin/conductors endpoints)
// ---------------------------------------------------------------------------

export interface ConductorSummary {
  conductorId: string;
  conductorUrl: string;
  adminUrl: string;
  capacityUsed: number;
  capacityMax: number;
  capacityAvailable: number;
  agentCount: number;
}

export interface AdminConductorsResponse {
  total: number;
  totalAgents: number;
  totalCapacity: number;
  conductors: ConductorSummary[];
}

export interface AgentSummary {
  agentPubKey: string;
  appId: string;
  assignedAt: string;
}

export interface ConductorAgentsResponse {
  conductorId: string;
  total: number;
  agents: AgentSummary[];
}

export interface AgentConductorResponse {
  agentPubKey: string;
  conductorId: string;
  conductorUrl: string;
  appId: string;
  assignedAt: string;
}

// ---------------------------------------------------------------------------
// Admin Federation types (mirrors doorway /admin/federation endpoints)
// ---------------------------------------------------------------------------

export interface FederationPeer {
  url: string;
  reachable: boolean;
  doorwayId: string | null;
  region: string | null;
  capabilities: string[];
}

export interface FederationPeersResponse {
  peers: FederationPeer[];
  total: number;
  selfId: string | null;
}

export interface AdminMutationResponse {
  success: boolean;
  message: string;
}

// ---------------------------------------------------------------------------
// Admin Pipeline types (mirrors doorway /admin/pipeline endpoint)
// ---------------------------------------------------------------------------

export interface PipelineResponse {
  registeredTotal: number;
  registeredActive: number;
  hostedTotal: number;
  graduatingCount: number;
  stewardCount: number;
}

// ---------------------------------------------------------------------------
// Admin User types (mirrors doorway /admin/users endpoints)
// ---------------------------------------------------------------------------

export interface AdminUserSummary {
  id: string;
  identifier: string;
  permissionLevel: string;
  isActive: boolean;
  storagePercent?: number;
  createdAt?: string;
}

export interface AdminUsersResponse {
  users: AdminUserSummary[];
  total: number;
  page: number;
  totalPages: number;
}

export interface AdminUserDetailsResponse {
  id: string;
  identifier: string;
  permissionLevel: string;
  isActive: boolean;
  usage?: {
    storageBytes: number;
    projectionQueries: number;
    bandwidthBytes: number;
  };
  quota?: {
    storageLimit: number;
    dailyQueryLimit: number;
    dailyBandwidthLimit: number;
  };
}

export interface AdminListUsersParams {
  search?: string;
  page?: number;
  limit?: number;
  isActive?: boolean;
  permissionLevel?: string;
}

// ---------------------------------------------------------------------------
// Observation Session types (mirrors doorway /api/v1/observations endpoints)
// ---------------------------------------------------------------------------

export interface ObservationReport {
  contentId: string;
  sessionId: string;
  source: string;
  metadata?: Record<string, unknown>;
  duration: {
    startedAt: string;
    endedAt: string;
    durationMs: number;
  };
  summary: {
    totalEntries: number;
    byOrigin: Record<string, number>;
    bySeverity: Record<string, number>;
    byCategory: Record<string, number>;
  };
  issues: {
    id: string;
    category: string;
    severity: string;
    title: string;
    entryCount: number;
    relatedContentIds: string[];
    suggestedCause: string;
  }[];
  systemState: {
    storageHealthy: boolean;
    conductorConnected: boolean;
    p2pPeerCount: number;
  };
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

export class DoorwayClient {
  /**
   * Single token home: auth flows (via the composed session client) and raw
   * setToken() both write here; every request reads its bearer from here.
   */
  private readonly tokenStore = new InMemorySessionTokenStore();
  private readonly sessionClient: DoorwaySessionClient;

  constructor(
    private readonly baseUrl: string,
    token?: string
  ) {
    this.sessionClient = new DoorwaySessionClient({
      baseUrl,
      tokenStore: this.tokenStore,
      // Thread the active observation session through auth requests too.
      fetchImpl: async (input, init) => fetch(input, this.withObservation(init)),
    });
    if (token) this.setToken(token);
  }

  /** Active observation session ID, if any. */
  observationId: string | null = null;

  get url(): string {
    return this.baseUrl;
  }

  /** Stored session identity (null when unauthenticated). */
  get session(): StoredSession | null {
    return this.tokenStore.get();
  }

  setToken(token: string): void {
    // Re-setting the bearer that is already stored keeps the full session
    // identity (device flows call setToken right after login). A NEW bare
    // token (steps injecting arbitrary JWTs) synthesizes a minimal session.
    if (this.tokenStore.get()?.token === token) return;
    this.tokenStore.set({
      token,
      humanId: '',
      agentPubKey: '',
      identifier: '',
      expiresAt: Number.MAX_SAFE_INTEGER,
    });
  }

  clearToken(): void {
    this.tokenStore.clear();
  }

  // -- Health ---------------------------------------------------------------

  async health(): Promise<HealthResponse> {
    return this.get<HealthResponse>('/health');
  }

  async isHealthy(): Promise<boolean> {
    try {
      const h = await this.health();
      return h.healthy;
    } catch {
      return false;
    }
  }

  // -- Auth (delegated to @elohim/identity DoorwaySessionClient) -------------

  async register(req: RegisterRequest): Promise<AuthResponse> {
    return this.sessionClient.register(req);
  }

  async login(req: LoginRequest): Promise<AuthResponse> {
    return this.sessionClient.login(req);
  }

  async logout(): Promise<SuccessResponse> {
    return this.sessionClient.logout();
  }

  async me(): Promise<MeResponse> {
    return this.sessionClient.me();
  }

  // -- Stewardship Allocations -----------------------------------------------

  async listAllocations(): Promise<AllocationView[]> {
    // Page through the FULL active set. The persistent alpha table holds
    // >10k legitimate rows (~3.4k content × ~3.4 stewards), so any single
    // bounded GET silently truncates — content items straddling the page
    // boundary lose stewards and their ratio sums break (genesis #1105).
    // A bigger bare limit only moves the cliff; paging until a short page
    // cannot truncate.
    const pageSize = 2000;
    const all: AllocationView[] = [];
    for (let offset = 0; ; offset += pageSize) {
      const page = await this.get<AllocationView[]>(
        `/db/allocations?active_only=true&limit=${pageSize}&offset=${offset}`
      );
      all.push(...page);
      if (page.length < pageSize) break;
    }
    return all;
  }

  async getAllocationsForContent(contentId: string): Promise<AllocationView[]> {
    // GET /db/allocations/content/{id} returns the ContentStewardshipView
    // AGGREGATE (http.rs handle_allocations_for_content), not a bare array —
    // unwrap the envelope so callers keep the AllocationView[] contract.
    const view = await this.get<ContentStewardshipView>(
      `/db/allocations/content/${encodeURIComponent(contentId)}`
    );
    return view.allocations;
  }

  async getAllocationsForSteward(stewardId: string): Promise<AllocationView[]> {
    return this.get<AllocationView[]>(`/db/allocations/steward/${encodeURIComponent(stewardId)}`);
  }

  // -- Paths ----------------------------------------------------------------

  async listPaths(): Promise<PathIndexEntry[]> {
    return this.get<PathIndexEntry[]>('/db/paths');
  }

  async getPath(id: string): Promise<PathWithDetailsView> {
    return this.get<PathWithDetailsView>(`/db/paths/${encodeURIComponent(id)}`);
  }

  // -- Presences ------------------------------------------------------------

  async listPresences(): Promise<PresenceView[]> {
    return this.get<PresenceView[]>('/db/presences');
  }

  async getPresence(id: string): Promise<PresenceView> {
    return this.get<PresenceView>(`/db/presences/${encodeURIComponent(id)}`);
  }

  // -- Content CRUD ---------------------------------------------------------

  async createContent(content: Record<string, unknown>): Promise<Record<string, unknown>> {
    return this.post<Record<string, unknown>>('/db/content', content);
  }

  async getContent(id: string): Promise<Record<string, unknown>> {
    return this.get<Record<string, unknown>>(`/db/content/${id}`);
  }

  async searchContent(tags: string[]): Promise<Record<string, unknown>[]> {
    const tagsCsv = tags.map(t => encodeURIComponent(t)).join(',');
    // limit=10000: storage's tag filter is applied in memory AFTER the SQL
    // LIMIT (content_diesel.rs list_content — default 100, newest-first), so
    // without a high limit a tag query silently misses any tagged row older
    // than the newest 100 ("No content found with tag X" on a seeded corpus).
    const envelope = await this.get<{ items: Record<string, unknown>[] }>(
      `/db/content?tags=${tagsCsv}&limit=10000`
    );
    return envelope.items;
  }

  /**
   * DELETE /db/content/{id} → `delete_cascade` (removes the content row and
   * its relationships). Path-addressed; the handler ignores the request body.
   *
   * Used by the content-lifecycle E2E cleanup so a run deletes the
   * `e2e-<uuid>` content it created instead of leaking a NULL-anchor row on
   * every scenario. Those leaked rows are what the reanchor backfill
   * re-selects and re-thrashes on every boot, saturating the target
   * conductor's Holochain Cache-DB read pool (main-branch CI → adam).
   */
  async deleteContent(id: string): Promise<Record<string, unknown>> {
    return this.delete<Record<string, unknown>>(`/db/content/${encodeURIComponent(id)}`, {});
  }

  // -- Session Handoff (delegated to @elohim/identity) ------------------------

  async sessionToken(): Promise<SessionTokenResponse> {
    return this.sessionClient.requestSessionToken();
  }

  async exchangeSession(sessionToken: string): Promise<ExchangeSessionResponse> {
    return this.sessionClient.exchangeSessionToken(sessionToken);
  }

  // -- Account (doorway-app) ------------------------------------------------

  async account(): Promise<AccountResponse> {
    return this.get<AccountResponse>('/auth/account');
  }

  // -- Status (comprehensive) -----------------------------------------------

  async status(): Promise<StatusResponse> {
    return this.get<StatusResponse>('/status.json');
  }

  // -- Admin: Nodes ---------------------------------------------------------

  async adminNodes(): Promise<AdminNodesResponse> {
    return this.get<AdminNodesResponse>('/admin/nodes');
  }

  // -- Admin: Conductors ----------------------------------------------------

  async adminConductors(): Promise<AdminConductorsResponse> {
    return this.get<AdminConductorsResponse>('/admin/conductors');
  }

  async adminConductorAgents(conductorId: string): Promise<ConductorAgentsResponse> {
    return this.get<ConductorAgentsResponse>(
      `/admin/conductors/${encodeURIComponent(conductorId)}/agents`
    );
  }

  async adminAgentConductor(agentPubKey: string): Promise<AgentConductorResponse> {
    return this.get<AgentConductorResponse>(
      `/admin/agents/${encodeURIComponent(agentPubKey)}/conductor`
    );
  }

  // -- Admin: Federation ----------------------------------------------------

  private static readonly FEDERATION_PEERS_PATH = '/admin/federation/peers';

  async adminFederationPeers(): Promise<FederationPeersResponse> {
    return this.get<FederationPeersResponse>(DoorwayClient.FEDERATION_PEERS_PATH);
  }

  async adminAddFederationPeer(url: string): Promise<AdminMutationResponse> {
    return this.post<AdminMutationResponse>(DoorwayClient.FEDERATION_PEERS_PATH, { url });
  }

  async adminRemoveFederationPeer(url: string): Promise<AdminMutationResponse> {
    return this.delete<AdminMutationResponse>(DoorwayClient.FEDERATION_PEERS_PATH, { url });
  }

  async adminRefreshFederationPeers(): Promise<AdminMutationResponse> {
    return this.post<AdminMutationResponse>('/admin/federation/peers/refresh', {});
  }

  // -- Admin: Pipeline (agency funnel) --------------------------------------

  async adminPipeline(): Promise<PipelineResponse> {
    return this.get<PipelineResponse>('/admin/pipeline');
  }

  // -- Admin: Users ---------------------------------------------------------

  async adminListUsers(params?: AdminListUsersParams): Promise<AdminUsersResponse> {
    const qs = new URLSearchParams();
    if (params?.search) qs.set('search', params.search);
    if (params?.page !== undefined) qs.set('page', String(params.page));
    if (params?.limit !== undefined) qs.set('limit', String(params.limit));
    if (params?.isActive !== undefined) qs.set('is_active', String(params.isActive));
    if (params?.permissionLevel) qs.set('permission_level', params.permissionLevel);
    const query = qs.toString();
    const path = query ? `/admin/users?${query}` : '/admin/users';
    return this.get<AdminUsersResponse>(path);
  }

  async adminGetUser(userId: string): Promise<AdminUserDetailsResponse> {
    return this.get<AdminUserDetailsResponse>(`/admin/users/${encodeURIComponent(userId)}`);
  }

  async adminSetUserStatus(userId: string, isActive: boolean): Promise<AdminMutationResponse> {
    return this.put<AdminMutationResponse>(`/admin/users/${encodeURIComponent(userId)}/status`, {
      isActive,
    });
  }

  async adminDeleteUser(userId: string): Promise<AdminMutationResponse> {
    return this.delete<AdminMutationResponse>(`/admin/users/${encodeURIComponent(userId)}`, {});
  }

  async adminForceLogout(userId: string): Promise<AdminMutationResponse> {
    return this.post<AdminMutationResponse>(
      `/admin/users/${encodeURIComponent(userId)}/force-logout`,
      {}
    );
  }

  // -- HTTP helpers ---------------------------------------------------------

  private async get<T>(path: string): Promise<T> {
    const { statusCode, body } = await request(`${this.baseUrl}${path}`, {
      method: 'GET',
      headers: this.headers(),
    });
    const text = await body.text();
    if (statusCode < 200 || statusCode >= 300) {
      throw new Error(`GET ${path} returned ${statusCode}: ${text}`);
    }
    return JSON.parse(text) as T;
  }

  private async post<T>(path: string, payload: unknown): Promise<T> {
    const { statusCode, body } = await request(`${this.baseUrl}${path}`, {
      method: 'POST',
      headers: { ...this.headers(), 'content-type': 'application/json' },
      body: JSON.stringify(payload),
    });
    const text = await body.text();
    if (statusCode < 200 || statusCode >= 300) {
      throw new Error(`POST ${path} returned ${statusCode}: ${text}`);
    }
    return JSON.parse(text) as T;
  }

  private async put<T>(path: string, payload: unknown): Promise<T> {
    const { statusCode, body } = await request(`${this.baseUrl}${path}`, {
      method: 'PUT',
      headers: { ...this.headers(), 'content-type': 'application/json' },
      body: JSON.stringify(payload),
    });
    const text = await body.text();
    if (statusCode < 200 || statusCode >= 300) {
      throw new Error(`PUT ${path} returned ${statusCode}: ${text}`);
    }
    return JSON.parse(text) as T;
  }

  private async delete<T>(path: string, payload: unknown): Promise<T> {
    const { statusCode, body } = await request(`${this.baseUrl}${path}`, {
      method: 'DELETE',
      headers: { ...this.headers(), 'content-type': 'application/json' },
      body: JSON.stringify(payload),
    });
    const text = await body.text();
    if (statusCode < 200 || statusCode >= 300) {
      throw new Error(`DELETE ${path} returned ${statusCode}: ${text}`);
    }
    return JSON.parse(text) as T;
  }

  private headers(): Record<string, string> {
    const h: Record<string, string> = {};
    const token = this.tokenStore.get()?.token;
    if (token) h['authorization'] = `Bearer ${token}`;
    if (this.observationId) h['x-observation-id'] = this.observationId;
    return h;
  }

  /** Add the observation header to a session-client request when active. */
  private withObservation(init?: RequestInit): RequestInit | undefined {
    if (!this.observationId) return init;
    return {
      ...init,
      headers: {
        ...((init?.headers ?? {}) as Record<string, string>),
        'x-observation-id': this.observationId,
      },
    };
  }

  // -- Observations ---------------------------------------------------------

  /**
   * Begin an observation session. All subsequent requests will carry
   * X-Observation-Id and the infrastructure will auto-observe.
   */
  async beginObservation(metadata?: Record<string, unknown>): Promise<string> {
    const resp = await this.post<{ sessionId: string; expiresAt: string }>(
      '/api/v1/observations/begin',
      { source: 'a2o', ttlSeconds: 300, metadata }
    );
    this.observationId = resp.sessionId;
    return resp.sessionId;
  }

  /**
   * Fetch the composed observation report and clear the session.
   */
  async getObservationReport(): Promise<ObservationReport> {
    if (!this.observationId) {
      throw new Error('No active observation session');
    }
    const report = await this.get<ObservationReport>(
      `/api/v1/observations/${this.observationId}/report`
    );
    this.observationId = null;
    return report;
  }
}
