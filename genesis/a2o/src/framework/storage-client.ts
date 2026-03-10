/**
 * Lightweight HTTP client for elohim-storage.
 * Used by testnet manager for REA Commitment + EconomicEvent persistence.
 */

const DEFAULT_BASE_URL = 'http://localhost:8090';

export interface Measure {
  hasNumericalValue: number;
  hasUnit: string;
}

export interface CreateCommitmentInput {
  id?: string;
  action: string;
  provider: string;
  receiver: string;
  resourceClassifiedAs?: string[];
  resourceQuantity?: Measure;
  effortQuantity?: Measure;
  hasBeginning?: string;
  hasEnd?: string;
  due?: string;
  clauseOf?: string;
  inScopeOf?: string[];
  mediumOfExchangeId?: string;
  note?: string;
  metadata?: Record<string, unknown>;
}

export interface CommitmentView {
  id: string;
  action: string;
  provider: string;
  receiver: string;
  resourceClassifiedAs?: string[];
  resourceQuantity?: Measure;
  effortQuantity?: Measure;
  mediumOfExchangeId?: string;
  state: string;
  finished: boolean;
  createdAt: string;
}

export interface CreateEconomicEventInput {
  id?: string;
  action: string;
  provider: string;
  receiver: string;
  resourceClassifiedAs?: string[];
  resourceQuantityValue?: number;
  resourceQuantityUnit?: string;
  effortQuantityValue?: number;
  effortQuantityUnit?: string;
  hasPointInTime?: string;
  fulfills?: string[];
  lamadEventType?: string;
  note?: string;
  metadataJson?: string;
}

export class StorageClient {
  constructor(private baseUrl: string = DEFAULT_BASE_URL) {}

  async isHealthy(): Promise<boolean> {
    try {
      const res = await fetch(`${this.baseUrl}/api/v1/health`, {
        signal: AbortSignal.timeout(5000),
      });
      return res.ok;
    } catch {
      return false;
    }
  }

  async createCommitment(input: CreateCommitmentInput): Promise<CommitmentView> {
    const res = await fetch(`${this.baseUrl}/api/v1/commitments`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(input),
    });
    if (!res.ok) {
      const text = await res.text();
      throw new Error(`Failed to create commitment: ${res.status} ${text}`);
    }
    return (await res.json()) as CommitmentView;
  }

  async updateCommitmentState(
    id: string,
    state: string,
    finished?: boolean,
  ): Promise<CommitmentView> {
    const res = await fetch(`${this.baseUrl}/api/v1/commitments/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ state, finished }),
    });
    if (!res.ok) {
      const text = await res.text();
      throw new Error(`Failed to update commitment: ${res.status} ${text}`);
    }
    return (await res.json()) as CommitmentView;
  }

  async getCommitment(id: string): Promise<CommitmentView | null> {
    const res = await fetch(`${this.baseUrl}/api/v1/commitments/${id}`);
    if (res.status === 404) return null;
    if (!res.ok) throw new Error(`Failed to get commitment: ${res.status}`);
    return (await res.json()) as CommitmentView;
  }

  async listCommitments(query?: Record<string, string>): Promise<CommitmentView[]> {
    const params = query ? '?' + new URLSearchParams(query).toString() : '';
    const res = await fetch(`${this.baseUrl}/api/v1/commitments${params}`);
    if (!res.ok) throw new Error(`Failed to list commitments: ${res.status}`);
    return (await res.json()) as CommitmentView[];
  }

  async createEconomicEvent(input: CreateEconomicEventInput): Promise<Record<string, unknown>> {
    const res = await fetch(`${this.baseUrl}/api/v1/economic-events`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(input),
    });
    if (!res.ok) {
      const text = await res.text();
      throw new Error(`Failed to create economic event: ${res.status} ${text}`);
    }
    return (await res.json()) as Record<string, unknown>;
  }

  async getAgentCommitments(agentId: string): Promise<CommitmentView[]> {
    const res = await fetch(`${this.baseUrl}/api/v1/commitments/agent/${agentId}`);
    if (!res.ok) throw new Error(`Failed to get agent commitments: ${res.status}`);
    return (await res.json()) as CommitmentView[];
  }
}
