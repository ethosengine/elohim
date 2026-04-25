/**
 * Account Model — re-exports from local schema-generated types.
 *
 * These types are generated from protocol schemas via schema:codegen:ts.
 * Source of truth: elohim/sdk/schemas/v1/views/account-view.schema.json
 *
 * NOTE: When @elohim/storage-client's generated/index.ts includes AccountView
 * (after the storage-client build is updated), migrate these imports to
 * @elohim/storage-client to centralise the boundary.
 */

import type { AccountView } from '@app/generated/account-view';

export type {
  AccountView,
  KeyRotationView,
  KeyRevocationView,
  RecoveryRequestView,
  HumanRelationshipView,
  HumanView,
} from '@app/generated/account-view';

// AgentPeerBindingView is in its own schema-generated file
export type { AgentPeerBindingView } from '@app/generated/agent-peer-binding-view';

// ---------------------------------------------------------------------------
// UI-layer state shape (Angular-only — not a wire type)
// ---------------------------------------------------------------------------

export interface AccountServiceState {
  account: AccountView | null;
  loading: boolean;
  error: string | null;
}
