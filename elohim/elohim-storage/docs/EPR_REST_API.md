# EPR REST API — elohim-storage

**Phase 2a (current).** Seven HTTP routes under `/api/v1/epr`. All routes are
served by `src/api/epr.rs` via the `EprStore` trait, so P2P federation (Phase
2c) can be wired in without route changes.

## Route Table

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| `GET` | `/api/v1/epr` | `list_epr` | Paged list of atoms held locally |
| `PUT` | `/api/v1/epr/:cid` | `put_epr` | Idempotent ingest |
| `GET` | `/api/v1/epr/:cid` | `get_epr` | Full atom (envelope + payload) |
| `GET` | `/api/v1/epr/:cid/envelope` | `get_envelope` | Envelope only (no payload) |
| `GET` | `/api/v1/epr/:cid/payload` | `get_payload` | Raw payload bytes (hex) |
| `GET` | `/api/v1/epr/:cid/verify` | `get_verify` | Cryptographic verification |
| `GET` | `/api/v1/epr/:cid/providers` | `get_providers` | Peers advertising this CID |

---

## Common Response Header

All GET responses include:

```
X-Epr-Source: local
```

| Value | Meaning |
|-------|---------|
| `local` | Atom served from this node's local SQLite store |
| `peer:<PeerId>` | Atom fetched from a remote peer and cached locally (Phase 2c) |

Phase 2a always returns `local`. Phase 2c wires the libp2p swarm handle into
`FederatedEprStore`, at which point remote fetches return `peer:<PeerId>`.

---

## Route Details

### GET /api/v1/epr

Paged list of atoms held locally, filtered by optional query parameters.

**Query parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `kind` | string | Filter by `EprKind` (e.g. `Content`, `Claim`) |
| `reach` | string | Filter by reach value (e.g. `commons`, `public`) |
| `schemaRef` | string | Filter by schema CID |
| `afterCid` | string | Cursor for next page (opaque — use `nextCursor` from previous response) |
| `limit` | integer | Page size (default 50) |

**Response:** `EprListView`

```json
{
  "items": [ /* array of EprEnvelopeView */ ],
  "nextCursor": "bafyrei..." // null when exhausted
}
```

---

### PUT /api/v1/epr/:cid

Idempotent content-addressed ingest. The `:cid` in the URL must match the CID
declared in the request body's `envelope.cid`.

**Why PUT (not POST)?** The CID is content-derived (SHA-256 of canonical CBOR
bytes), so the resource identity is known before it is stored. PUT is the
semantically correct method for idempotent creation of a named resource.

**Request body:** `EprPublishInput`

```json
{
  "envelope": {
    "cid": "bafyrei...",
    "kind": "Content",
    "schemaRef": "bafyrei...",
    "schemaKey": "concept",
    "reach": "commons",
    "coupling": {
      "knowledge": "bafyrei...",
      "value": "bafyrei...",
      "governance": null
    },
    "claims": [],
    "supersedes": null,
    "issuedAt": "2026-04-21T12:00:00Z",
    "proof": {
      "signer": "bafyrei...",
      "algorithm": "ed25519",
      "signature": "<128 hex chars>"
    }
  },
  "payload": "<hex-encoded bytes>"
}
```

**Idempotency behaviour:**

- If the CID does not exist: validate → persist → 201.
- If the CID exists with matching canonical bytes: 200 (no-op).
- If the CID exists with different canonical bytes: 400 Invalid Input (collision
  attempt detected — the same CID may not map to different content).

**Validation stages run at ingest:**

1. `canonicalization` — recompute canonical CBOR bytes and verify CID
2. `signature` — structural check (algorithm=ed25519, signature=64 bytes)
3. `coupling` — kind-required coupling legs are present
4. `payloadSchema` — **deferred to Phase 3** (needs manifest resolver)

**Response:** `{ "cid": "bafyrei..." }` on success.

---

### GET /api/v1/epr/:cid

Full atom: envelope + hex-encoded payload.

**Query parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `includeCanonical` | boolean | Include hex-encoded canonical bytes in response |

**Response:** `EprView`

```json
{
  "envelope": { /* EprEnvelopeView */ },
  "payload": "<hex>",
  "canonicalBytes": "<hex>" // only when ?includeCanonical=true
}
```

**Status codes:** 200 (found), 404 (not found).

---

### GET /api/v1/epr/:cid/envelope

Envelope-only view. No payload bytes. Useful for inspection, link following,
or reach/kind filtering before deciding whether to fetch the full payload.

**Response:** `EprEnvelopeView`

**Status codes:** 200, 404.

---

### GET /api/v1/epr/:cid/payload

Raw payload bytes as hex. Clients needing only the content (e.g. blob
renderers) can skip the envelope overhead.

**Response:** `{ "cid": "bafyrei...", "payload": "<hex>" }`

**Status codes:** 200, 404.

---

### GET /api/v1/epr/:cid/verify

Cryptographic verification against a caller-supplied Ed25519 public key.

**Query parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `pubkey` | string | Yes | Hex-encoded 32-byte Ed25519 public key |

**Verification stages:**

| Stage | Phase 2a | Notes |
|-------|----------|-------|
| `canonicalization` | Run | CID must match SHA-256 of canonical CBOR |
| `signature` | Run | Ed25519 signature must verify under `pubkey` |
| `coupling` | Run | Kind-required legs must be present |
| `payloadSchema` | **Skipped** | Deferred to Phase 3 (needs manifest resolver) |

**Response:** `EprVerifyView`

```json
{
  "cid": "bafyrei...",
  "verified": true,
  "stagesRun": ["canonicalization", "signature", "coupling"],
  "stagesSkipped": ["payloadSchema"],
  "error": null
}
```

On failure, `verified: false` and `error.stage` identifies which stage failed.

**Status codes:** 200 (report delivered — `verified` field carries the result),
404 (CID not found).

---

### GET /api/v1/epr/:cid/providers

Returns the set of peers advertising that they hold the given CID.

**Response:** `EprProvidersView`

```json
{
  "cid": "bafyrei...",
  "providers": ["local"]
}
```

**Phase 2a behaviour:**

- `["local"]` when the atom is held by this node.
- `[]` when the atom is not known to this node.

**Phase 2c behaviour (planned):** extends with Kad DHT provider records from
the libp2p swarm. Each remote provider appears as `"peer:<libp2p-PeerId>"`.

**Status codes:** 200 always (empty array is a valid response).

---

## Response Types (TypeScript)

All response types are auto-generated by ts-rs from `elohim-storage/src/views.rs`
and distributed to `elohim/sdk/storage-client-ts/src/generated/`:

| Rust type | TS type | Description |
|-----------|---------|-------------|
| `EprView` | `EprView` | Full atom |
| `EprEnvelopeView` | `EprEnvelopeView` | Envelope only |
| `EprCouplingView` | `EprCouplingView` | Coupling legs |
| `EprSignatureView` | `EprSignatureView` | Ed25519 proof |
| `EprVerifyView` | `EprVerifyView` | Verify report |
| `EprVerifyErrorView` | `EprVerifyErrorView` | Verify failure detail |
| `EprListView` | `EprListView` | Paged list |
| `EprPublishInput` | `EprPublishInput` | PUT request body |
| `EprProvidersView` | `EprProvidersView` | Provider list |

All fields are camelCase. snake_case never leaves the Rust boundary.

---

## Phase Roadmap

| Phase | Milestone |
|-------|-----------|
| **2a** (current) | LocalEprStore — SQLite, 7 routes, Ed25519 structural validation |
| **2b** | Auth middleware — JWT-gated 404 for `reach: private/intimate` without session |
| **2c** | FederatedEprStore — libp2p bridge, `EprRequest::Resolve`, Kad `get_providers`, `X-Epr-Source: peer:<PeerId>` |
| **3** | Payload schema validation via manifest resolver (4th verification stage) |

### Phase 2c libp2p bridge

`FederatedEprStore` in `src/services/epr_store.rs` already carries the
`TODO(phase-2c)` markers:

- `fetch`: on local miss, issue `EprRequest::Resolve { id: cid }` to N peers
  via the `/elohim/epr/1.0.0` libp2p protocol. On `EprResponse::Head`,
  validate + put locally + return `EprSource::Peer(peer_id)`.
- `put`: after local persist, call `kad_start_providing(cid)` so the DHT can
  route future `Resolve` requests to this node.
- `providers`: extend local result with `kad_get_providers(cid)`.

Wire format: 4-byte BE length prefix + MessagePack (matching the shard and sync
protocols in `src/p2p/`).

---

## Source Files

| File | Purpose |
|------|---------|
| `src/api/epr.rs` | Route dispatcher and handler functions |
| `src/services/epr_store.rs` | `EprStore` trait, `LocalEprStore`, `FederatedEprStore` |
| `src/services/epr_service.rs` | Domain logic: ingest, fetch, list, verify |
| `src/db/epr_atoms.rs` | Diesel models and queries for EPR tables |
| `src/views.rs` | EPR view types (Rust → TS boundary) |
| `migrations/*add_epr_tables*` | SQLite schema for epr_atoms, epr_coupling, epr_claims, epr_supersedence |
| `tests/epr_ingest_integration.rs` | Integration: ingest/fetch/verify/idempotency |
| `tests/epr_reach_enforcement.rs` | Integration: reach persistence across all variants |
| `tests/schema_contract_diesel_epr.rs` | Diesel column ↔ JSON schema alignment |
