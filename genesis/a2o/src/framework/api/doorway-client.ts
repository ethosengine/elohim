/**
 * Typed HTTP client for the Doorway API.
 *
 * Types derived from:
 *   - doorway/src/routes/auth_routes.rs (RegisterRequest, AuthResponse)
 *   - doorway/src/routes/health.rs (HealthResponse)
 */

import { request } from 'undici';

// ---------------------------------------------------------------------------
// Request / Response types (mirrors Rust structs, camelCase on the wire)
// ---------------------------------------------------------------------------

export interface RegisterRequest {
  identifier: string;
  password: string;
  displayName: string;
  identifierType?: string;
  bio?: string;
  affinities?: string[];
  profileReach?: string;
  adminBootstrapKey?: string;
}

export interface LoginRequest {
  identifier: string;
  password: string;
}

export interface AuthResponse {
  token: string;
  humanId: string;
  agentPubKey: string;
  identifier: string;
  expiresAt: number;
  doorwayId?: string;
  doorwayUrl?: string;
  installedAppId?: string;
  profile?: HumanProfileResponse;
}

export interface HumanProfileResponse {
  id: string;
  displayName: string;
  bio?: string;
  affinities: string[];
  profileReach: string;
  location?: string;
  createdAt: string;
  updatedAt: string;
}

export interface MeResponse {
  humanId: string;
  agentPubKey: string;
  identifier: string;
  permissionLevel: string;
  doorwayId?: string;
  doorwayUrl?: string;
}

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
// Session Transfer types (mirrors Rust session handoff endpoints)
// ---------------------------------------------------------------------------

export interface SessionTokenResponse {
  sessionToken: string;
  expiresAt: number;
}

export interface SessionExchangeResponse {
  token: string;
  humanId: string;
  agentPubKey: string;
  identifier: string;
  expiresAt: number;
}

// ---------------------------------------------------------------------------
// Account types (mirrors doorway admin AccountResponse)
// ---------------------------------------------------------------------------

export interface AccountResponse {
  identifier: string;
  humanId: string;
  agentPubKey: string;
  permissionLevel: string;
  isActive: boolean;
  isSteward: boolean;
  doorwayId?: string;
  doorwayName?: string;
  doorwayRegion?: string;
  memberSince?: string;
  lastLogin?: string;
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
  inventory?: {
    cpuCores: number;
    memoryGb: number;
    storageTb: number;
    bandwidthMbps: number;
    region: string;
  };
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
// Client
// ---------------------------------------------------------------------------

export class DoorwayClient {
  constructor(
    private readonly baseUrl: string,
    private token?: string
  ) {}

  get url(): string {
    return this.baseUrl;
  }

  setToken(token: string): void {
    this.token = token;
  }

  clearToken(): void {
    this.token = undefined;
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

  // -- Auth -----------------------------------------------------------------

  async register(req: RegisterRequest): Promise<AuthResponse> {
    return this.post<AuthResponse>('/auth/register', req);
  }

  async login(req: LoginRequest): Promise<AuthResponse> {
    return this.post<AuthResponse>('/auth/login', req);
  }

  async logout(): Promise<{ success: boolean; message: string }> {
    return this.post<{ success: boolean; message: string }>('/auth/logout', {});
  }

  async me(): Promise<MeResponse> {
    return this.get<MeResponse>('/auth/me');
  }

  // -- Stewardship Allocations -----------------------------------------------

  async listAllocations(): Promise<AllocationView[]> {
    return this.get<AllocationView[]>('/db/allocations?active_only=true&limit=10000');
  }

  async getAllocationsForContent(contentId: string): Promise<AllocationView[]> {
    return this.get<AllocationView[]>(`/db/allocations/content/${encodeURIComponent(contentId)}`);
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
    const envelope = await this.get<{ items: Record<string, unknown>[] }>(
      `/db/content?tags=${tagsCsv}`
    );
    return envelope.items;
  }

  // -- Session Handoff ------------------------------------------------------

  async sessionToken(): Promise<SessionTokenResponse> {
    return this.get<SessionTokenResponse>('/auth/session-token');
  }

  async exchangeSession(sessionToken: string): Promise<SessionExchangeResponse> {
    return this.get<SessionExchangeResponse>(
      `/auth/exchange-session?session_token=${encodeURIComponent(sessionToken)}`
    );
  }

  // -- Account (doorway-app) ------------------------------------------------

  async account(): Promise<AccountResponse> {
    return this.get<AccountResponse>('/auth/account');
  }

  // -- Status (comprehensive) -----------------------------------------------

  async status(): Promise<StatusResponse> {
    return this.get<StatusResponse>('/status');
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

  async adminFederationPeers(): Promise<FederationPeersResponse> {
    return this.get<FederationPeersResponse>('/admin/federation/peers');
  }

  async adminAddFederationPeer(url: string): Promise<AdminMutationResponse> {
    return this.post<AdminMutationResponse>('/admin/federation/peers', { url });
  }

  async adminRemoveFederationPeer(url: string): Promise<AdminMutationResponse> {
    return this.delete<AdminMutationResponse>('/admin/federation/peers', { url });
  }

  async adminRefreshFederationPeers(): Promise<AdminMutationResponse> {
    return this.post<AdminMutationResponse>('/admin/federation/peers/refresh', {});
  }

  // -- Admin: Pipeline (agency funnel) --------------------------------------

  async adminPipeline(): Promise<PipelineResponse> {
    return this.get<PipelineResponse>('/admin/pipeline');
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
    if (this.token) h['authorization'] = `Bearer ${this.token}`;
    return h;
  }
}
