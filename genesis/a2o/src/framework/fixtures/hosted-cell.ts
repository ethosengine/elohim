/**
 * Reading a `hosted-cell` delegates-compute commitment the way the S1 plan's
 * Chief decisions require (doorway-federation-three-reds-to-green-plan.md,
 * Task 4 "Chief decisions on the S1 blind-reader findings", 2026-09-11):
 *
 *   Decision 1 (notary-side read): a hosted-cell commitment is read back BY
 *   CID from a household peer that is NOT the hosting doorway's pool — on the
 *   household mesh, jessica's storage (`:8091`) — via that peer's own
 *   `GET /api/v1/commitments/{id}` (elohim-storage/src/http.rs, route table in
 *   `build_manifest()`). Never through the doorway; the doorway only ever
 *   supplies the cid (`hostedCellGrantCid` on register / `GET /auth/account`,
 *   S2 Task 13).
 *
 *   Decision 2 (steward-key resolution): "the agent key that the pool
 *   conductor's own peer names as its steward" resolves from the POOL PEER's
 *   own storage self-identity (the household fixture's `storagePeers` table,
 *   `agentPubKey` stamped by `hc-mesh.sh refresh_fixture_pids` from that
 *   peer's own running AGENT_PUBKEY) — never from the doorway's own claim
 *   about who is hosting. `storagePeerForOrigin` (household-mesh.ts) does the
 *   origin match; this module wraps it for the two callers that need it.
 *
 * Field names below are read defensively (camelCase first, snake_case
 * fallback) because the exact `GET /api/v1/commitments/{id}` wire shape for a
 * `delegates-compute` grant is an S2/S3 concern this plan slice does not
 * implement — see `mishpat_projection.rs::DelegatesComputeProjection`
 * (cid/provider/recipient/scope columns) and `compute_grants.rs::grant_input`
 * (the raw payload: action, scope, provider, recipient, bounds, valid_from,
 * valid_until) for the two candidate shapes this defends against. This runs
 * RED until S2 Task 13/14 land the grant issuance and the wire contract
 * settles — that is the correct state for a story-first check.
 */

import { getRaw } from '../dataplane/surfaces.js';

import {
  loadHouseholdMeshFixture,
  requireFixtureStoragePeer,
  storagePeerForOrigin,
  type HouseholdMeshFixture,
} from './household-mesh.js';

export type CommitmentBody = Record<string, unknown>;

export interface CommitmentReadResult {
  status: number;
  body: CommitmentBody;
}

/** The household peer Decision 1 names as "not the hosting doorway's pool". */
export const NON_POOL_PEER_NAME = 'jessica';

/** jessica's storage origin — the notary read-back peer for every 07/humans-served step. */
export function nonPoolPeerUrl(fixture: HouseholdMeshFixture = loadHouseholdMeshFixture()): string {
  return requireFixtureStoragePeer(fixture, NON_POOL_PEER_NAME).url;
}

/**
 * `GET /api/v1/commitments/{cid}` on the non-pool peer. Never throws on a
 * non-2xx status (a 404 is itself an answer — "the notary has nothing under
 * this cid" — a caller may need to assert, e.g. after a revoke); callers
 * decide what a given status means for the scenario at hand.
 */
export async function readHostedCellCommitment(
  cid: string,
  fixture: HouseholdMeshFixture = loadHouseholdMeshFixture()
): Promise<CommitmentReadResult> {
  const peer = nonPoolPeerUrl(fixture);
  const { status, text } = await getRaw(`${peer}/api/v1/commitments/${encodeURIComponent(cid)}`);
  let body: CommitmentBody = {};
  if (text) {
    try {
      const parsed: unknown = JSON.parse(text);
      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
        body = parsed as CommitmentBody;
      }
    } catch {
      // Non-JSON body (e.g. a plain 404 page) — callers assert on `status` first.
    }
  }
  return { status, body };
}

/** First present field among camelCase/snake_case candidates — see module doc. */
export function commitmentField(commitment: CommitmentBody, ...keys: string[]): unknown {
  for (const key of keys) {
    if (commitment[key] !== undefined) return commitment[key];
  }
  return undefined;
}

const WITHDRAWN_STATES = new Set(['revoked', 'cancelled', 'sunset', 'withdrawn']);

/**
 * "Live" per the 07 story's own vocabulary: unwithdrawn AND not past its end
 * date. Mirrors `compute_grants.rs::WITHDRAWN_STATES` for the state check.
 */
export function commitmentIsLive(commitment: CommitmentBody): boolean {
  if (commitmentField(commitment, 'revokedAt', 'revoked_at')) return false;
  const state = commitmentField(commitment, 'state', 'status');
  if (typeof state === 'string' && WITHDRAWN_STATES.has(state)) return false;
  const until = commitmentField(commitment, 'validUntil', 'valid_until', 'hasEnd', 'has_end');
  if (typeof until === 'string') {
    const parsed = Date.parse(until);
    if (!Number.isNaN(parsed) && parsed <= Date.now()) return false;
  }
  return true;
}

/**
 * Decision 2's steward-key resolution: the agent key the POOL PEER at
 * `conductorOrigin` names as its own — never the doorway's opinion.
 *
 * `conductorOrigin` arrives from the doorway's conductor registry as a ws://
 * app-interface URL, so it resolves against the household fixture peer's
 * `conductorAppUrl` (stamped by `hc-mesh.sh refresh_fixture_pids`), NOT against
 * its HTTP storage `url`. The key is still read off that peer's own
 * `agentPubKey` — the registry only says WHICH peer, never who it is.
 */
export function stewardAgentPubKeyForConductorOrigin(
  conductorOrigin: string,
  fixture: HouseholdMeshFixture = loadHouseholdMeshFixture()
): string | undefined {
  return storagePeerForOrigin(fixture, conductorOrigin)?.peer.agentPubKey;
}

/**
 * The doorway's own service identity — the agent key of the storage peer the
 * household fixture names as that doorway's `primaryStorageUrl` (distinct from
 * its `poolStorageUrls`, which lend compute for humans it hosts).
 */
export function doorwayServiceIdentityAgentPubKey(
  doorwayId: string,
  fixture: HouseholdMeshFixture = loadHouseholdMeshFixture()
): string | undefined {
  const primary = fixture.doorways?.[doorwayId]?.primaryStorageUrl;
  if (!primary) return undefined;
  return storagePeerForOrigin(fixture, primary)?.peer.agentPubKey;
}
