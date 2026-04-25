/**
 * Portal Host Model — re-exports from local schema-generated types.
 *
 * These types are generated from protocol schemas via schema:codegen:ts.
 * Source of truth: elohim/sdk/schemas/v1/views/portal-host-view.schema.json
 */

import type { PortalHostView } from '@app/generated/portal-host-view';

export type { PortalHostView } from '@app/generated/portal-host-view';

// ---------------------------------------------------------------------------
// AddPortalHostInputView — input shape for POST /api/v1/account/portal-hosts.
// Mirrors the Rust AddPortalHostInputView in elohim-storage/src/views.rs.
// Will be replaced by @elohim/storage-client re-export once Task 11 codegen
// is distributed through the generated index.
// ---------------------------------------------------------------------------

export interface AddPortalHostInputView {
  hostUrl: string;
  label: string | null;
  /** Reach classification: "public" | "trusted" | "private". Defaults to "trusted". */
  reach: string | null;
}

// ---------------------------------------------------------------------------
// UI-layer state shape (Angular-only — not a wire type)
// ---------------------------------------------------------------------------

export interface PortalHostServiceState {
  hosts: PortalHostView[];
  loading: boolean;
  error: string | null;
}
