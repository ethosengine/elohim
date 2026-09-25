/**
 * SignalIntent and companion types — EPR write-through protocol (Task C.3).
 *
 * Migrated to @elohim/rea-runtime from @app/shefa/services/signal-emit.service
 * as part of the Wave 2 cross-pillar import cleanup (Slice 2.4 residual).
 *
 * These are pure types with no behavioral dependency. They mirror the wire
 * shape of `POST /api/v1/signal/emit` defined in
 * `elohim/sdk/schemas/v1/signal-intent.schema.json`.
 *
 * Design: these types belong in @elohim/rea-runtime (not @app/shefa) because
 * they are part of the REA signal-emit protocol contract that any pillar can
 * use — not shefa-specific. Lamad's signal harness, for example, constructs
 * SignalIntent values to emit EconomicEvent signals through the EPR
 * write-through path.
 */

/**
 * Body for `POST /api/v1/signal/emit`. Field names are camelCase per the
 * elohim-storage HTTP boundary contract.
 */
export interface SignalIntent {
  /** Pillar that authored this signal. */
  pillar: 'lamad' | 'shefa' | 'imagodei' | 'mishpat' | 'qahal';
  /** Pillar-defined signal name. Today must equal an EprKind value. */
  signalType: string;
  /** Content address of the agent issuing this signal. */
  agentCid: string;
  /** Pillar-shaped payload object — encoded as canonical CBOR by storage. */
  payload: Record<string, unknown>;
  /** Three-leg coupling refs (CIDs of related EPRs). */
  couplingRefs: {
    knowledge?: string;
    value?: string;
    governance?: string;
  };
  /** Optional reach override. Defaults to the manifest-declared reach. */
  reach?: string;
}

/** Response from `POST /api/v1/signal/emit` on success (HTTP 201). */
export interface SignalEmitSuccessResponse {
  /** CIDv1 of the persisted EPR Envelope. */
  eprCid: string;
  /** Application event id. Today equal to `eprCid`. */
  eventCid: string;
}

/** Result envelope returned by `tryEmit()` so callers branch on intent. */
export type SignalEmitResult =
  | { status: 'emitted'; response: SignalEmitSuccessResponse }
  | { status: 'fallback'; reason: string }
  | { status: 'error'; status_code: number; message: string };

/**
 * `GET /api/v1/status/write-through` — the node's effective write-through flag
 * per (pillar, kind), composed across its four override layers. Mirrors
 * elohim-storage `api::write_through_status::WriteThroughStatusView` (camelCase
 * on the wire). A pair the node does not list is not on.
 */
export interface WriteThroughStatusView {
  effective: WriteThroughEffectiveRow[];
  /** Integrity-bearing kinds: always written through, whatever the rows say. */
  integrityKinds: string[];
  /** Live layer-4 admin override, when one is set (opaque to the client). */
  adminOverride: unknown;
}

/** One effective (pillar, kind) row of {@link WriteThroughStatusView}. */
export interface WriteThroughEffectiveRow {
  pillar: string;
  kind: string;
  on: boolean;
  /** Which layer decided (`manifest-default`, `env-override`, …). */
  source: string;
}
