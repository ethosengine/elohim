import type { StorageClient } from '../client';
import type {
  CollabCollectiveView,
  CollabAgreementView,
  CollabQahalView,
  CollabMembershipView,
  CreateCollabCollectiveInputView,
  CreateCollabAgreementInputView,
  AttestCollabAgreementInputView,
  WithdrawMembershipInputView,
} from '../generated';

/**
 * SDK convenience methods for Collective + Collab agreement flows.
 *
 * Accessed via `client.qahal`:
 * ```typescript
 * const collective = await client.qahal.createCollective({ ... });
 * const agreement  = await client.qahal.createCollabAgreement({ ... });
 * ```
 */
export class QahalApi {
  constructor(private readonly client: StorageClient) {}

  /**
   * Create a new Collective (POST /api/v1/collective).
   */
  async createCollective(input: CreateCollabCollectiveInputView): Promise<CollabCollectiveView> {
    return this.client.postJson<CollabCollectiveView>('/api/v1/collective', input);
  }

  /**
   * Fetch a Collective by CID (GET /api/v1/collective/:cid).
   */
  async getCollective(cid: string): Promise<CollabCollectiveView> {
    return this.client.getJson<CollabCollectiveView>(
      `/api/v1/collective/${encodeURIComponent(cid)}`,
    );
  }

  /**
   * Create a new CollabAgreement between Collectives
   * (POST /api/v1/collab/agreement).
   */
  async createCollabAgreement(
    input: CreateCollabAgreementInputView,
  ): Promise<CollabAgreementView> {
    return this.client.postJson<CollabAgreementView>('/api/v1/collab/agreement', input);
  }

  /**
   * Attest a CollabAgreement on behalf of a Collective
   * (POST /api/v1/collab/agreement/:cid/attest).
   */
  async attestCollabAgreement(
    input: AttestCollabAgreementInputView,
  ): Promise<CollabAgreementView> {
    return this.client.postJson<CollabAgreementView>(
      `/api/v1/collab/agreement/${encodeURIComponent(input.agreementCid)}/attest`,
      input,
    );
  }

  /**
   * Fetch a CollabQahal (shared governance space) by CID
   * (GET /api/v1/collab/:cid).
   */
  async getCollab(cid: string): Promise<CollabQahalView> {
    return this.client.getJson<CollabQahalView>(`/api/v1/collab/${encodeURIComponent(cid)}`);
  }

  /**
   * Withdraw a Collective's membership from a CollabQahal
   * (POST /api/v1/collab/:cid/withdraw).
   */
  async withdrawFromCollab(input: WithdrawMembershipInputView): Promise<CollabMembershipView> {
    return this.client.postJson<CollabMembershipView>(
      `/api/v1/collab/${encodeURIComponent(input.collabQahalCid)}/withdraw`,
      input,
    );
  }
}
