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
// Client
// ---------------------------------------------------------------------------

export class DoorwayClient {
  constructor(
    private baseUrl: string,
    private token?: string,
  ) {}

  get url(): string {
    return this.baseUrl;
  }

  setToken(token: string): void {
    this.token = token;
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

  // -- Content CRUD ---------------------------------------------------------

  async createContent(content: Record<string, unknown>): Promise<Record<string, unknown>> {
    return this.post<Record<string, unknown>>('/db/content', content);
  }

  async getContent(id: string): Promise<Record<string, unknown>> {
    return this.get<Record<string, unknown>>(`/api/v1/cache/content/${id}`);
  }

  async searchContent(tags: string[]): Promise<Record<string, unknown>[]> {
    const params = new URLSearchParams();
    for (const tag of tags) params.append('tags', tag);
    return this.get<Record<string, unknown>[]>(`/api/v1/cache/content?${params}`);
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

  private headers(): Record<string, string> {
    const h: Record<string, string> = {};
    if (this.token) h['authorization'] = `Bearer ${this.token}`;
    return h;
  }
}
