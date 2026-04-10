# Peer Status Schema Contract — Design

**Date:** 2026-04-09
**Status:** Design approved, awaiting implementation plan
**Author:** Brainstorming session between Matthew and Claude
**Supersedes/extends:** `feat(p2p): export P2PStatusInfo and DrainStatusInfo via ts-rs` (547bfefc)

## P2P Design Gate Classification

Every entity in this plan is **Category C — Operational**. This is important and deliberate:

| Entity | Category | Source of Truth |
|---|---|---|
| `P2PStatusView` | C — Operational | libp2p Swarm state + in-memory replication tracker + SQLite `p2p_published_at` column (for drain). Reconstructed per request. Not persisted. |
| `PeerInfoView` | C — Operational | libp2p Swarm state + identify protocol + per-peer runtime tracking (RTT ring buffer, last-seen map, remote NAT dial-back results, BandwidthSinks). All ephemeral. |
| `PeerListView` | C — Operational | Same as `PeerInfoView` (pagination envelope). |
| `DrainStatusView` | C — Operational | SQLite aggregate query over the existing `p2p_published_at` column on the content table. |
| `ReplicationStatusView` | C — Operational | In-memory `ReplicationInner` struct. Rebuilt from discovery on every process start. |

**None of these entities are notarized on the DHT. None has a Holochain entry type. None has a `dht_anchor_hash` column. None has a coordinator zome function.** This is correct — they are observability projections of runtime state, not persisted records. The P2P design gate was invoked during this design to confirm the classification.

**Implication for the CONVENTIONS.md rule set:** Every view schema MUST declare its source of truth explicitly in the top-level `description` field. This is added to CONVENTIONS.md as an enforced rule (missing declaration = harness failure). The omission is what caused the P2P design gate hook to fire during the initial draft of this doc, and the fix is to make the declaration mandatory at the schema level so future agents cannot skip it.

**Implication for `isReady` derivation:** Since `P2PStatusView` is operational and reconstructed per request, the `isReady` derivation is also computed per request — consumers run it client-side against whatever snapshot they just received. There is no persisted `isReady` state.

## Motivation

The current `/p2p/status` wire shape is defined by a hand-written Rust struct (`P2PStatusInfo` in `elohim/elohim-storage/src/p2p/mod.rs`) with `#[derive(TS)]` for TypeScript generation. This works well enough for single-consumer types, but P2P status has five known consumers in the monorepo (doorway federation/main/server, elohim-app connection-indicator, simulate.sh, genesis Jenkinsfile, seeder) and is about to acquire more as connection strategies grow peer-aware routing logic.

The ts-rs approach creates several long-term problems:

1. **No contract surface.** Consumers bind to whatever ts-rs happens to emit on the last build. Field-name changes, nullability changes, and enum-value changes can slip through without any explicit decision point.
2. **String-typed enums.** `nat_status` and `relay_mode` are plain `String` on the wire, populated by `{:?}` debug-formatting libp2p's internal enums. Consumers pattern-match on string literals with no schema to tell them which values are possible.
3. **Inconsistent casing.** The rest of `elohim/sdk/schemas/v1/views/` is camelCase (content-view, economic-event-view). P2PStatusInfo is snake_case for historical-compatibility reasons. Every future schema decision has to re-litigate which convention applies.
4. **No per-peer visibility.** `/p2p/status` only exposes `connected_peers: usize` — a count, no detail. Connection strategies that want peer-aware routing have nowhere to look.
5. **No enforced drift detection.** If a consumer reads `connected_peers` and someone renames the Rust field, the drift surfaces as a runtime bug in whichever consumer runs first.

The strategic cost is bigger than this one view: `views/` already has two schemas and will likely grow to a dozen or more as the storage HTTP API expands (governance views, REA views, identity views, presence views). Every one of those future migrations will re-litigate the same decisions unless this plan establishes a reusable pattern.

## Goals

1. **Establish JSON Schema as the source of truth** for HTTP wire shapes in `views/`. Rust structs generate from schema, not the other way around.
2. **Lock the P2P status wire shape** as a documented contract that consumers bind to explicitly, with enforced drift detection.
3. **Expose per-peer detail** via a new `GET /p2p/peers` endpoint (Category C operational — no DHT entry type, source of truth is libp2p Swarm state reconstructed per request), giving connection strategies the data they need to do peer-aware routing.
4. **Formalize loose strings** (`nat_status`, `relay_mode`, connection direction) as typed enums in the schema.
5. **Migrate to camelCase** on the `/p2p/*` surface to match the rest of `views/`.
6. **Leave behind a reusable pattern** — CONVENTIONS.md, a validation harness, pre-push enforcement — so that the next twelve view migrations inherit the answers for free instead of re-litigating them.

## Non-goals

- **Not** deleting ts-rs entirely. ts-rs stays for every Rust type not migrated in this plan. Only the three P2P types (`P2PStatusInfo`, `DrainStatusInfo`, `ReplicationStatus`) switch to schema-generation. Future view migrations can reduce the ts-rs footprint further.
- **Not** extending the IoC pattern to non-view schemas. DNA entry schemas and protocol enum schemas in `elohim/sdk/schemas/v1/` follow their own conventions because they're DNA-notarized. This plan covers `views/` only.
- **Not** adding historical peer data, peer event logs, or disconnection tracking. `/p2p/peers` returns a snapshot of currently-connected peers. A future plan can add `/p2p/peer-events`.
- **Not** adding WebSocket/SSE push for peer changes. Consumers poll. Push is a separate plan when a consumer needs it.
- **Not** inventing new telemetry beyond what libp2p already provides. RTT, last-seen, remote NAT status, and bandwidth come from existing libp2p primitives (ping, identify, autonat dial-backs, BandwidthSinks).
- **Not** unifying with DNA entry schemas or the elohim-import pipeline. Out of scope.

## Architecture: IoC Pattern for Wire Contracts

**Scope reminder:** This architecture applies to `views/` only — Category C operational projections of runtime state. It does NOT apply to DNA entry schemas or protocol enum schemas, which have their own notarized source of truth on the DHT. Every view schema covered by this architecture MUST declare its source of truth in its top-level `description` field; the validation harness enforces this.

The long-term shape of wire contracts in this repo:

```
JSON Schema (elohim/sdk/schemas/v1/views/*.schema.json)
    │
    │  pnpm run schema:codegen:rs
    ├──────────────────────▶  Rust structs (elohim/elohim-storage/src/generated/views/)
    │                         │
    │                         │  serde serialization
    │                         ▼
    │                         HTTP response JSON
    │                         │
    │                         ▼
    │                         Validation harness (jsonschema crate)
    │                         validates against original schema
    │                         fails CI on drift
    │
    │  pnpm run schema:codegen:ts
    └──────────────────────▶  TypeScript types (elohim/sdk/storage-client-ts/src/generated/views/)
                              │
                              ▼
                              Consumers: elohim-app, doorway, seeder, etc.
```

Key invariants:

- **Schema is the only place where field names, types, nullability, enum values, and constraints are declared.**
- **Rust structs are generated**, not hand-written. The generated files are committed for visibility but never hand-edited.
- **TypeScript types are generated** from the same schema, replacing ts-rs for migrated types.
- **Validation harness closes the loop**: the Rust serializer's actual output is validated against the schema that generated it. This catches any codegen drift, serde attribute surprise, or manual override that would cause runtime behavior to diverge from the contract.
- **Pre-push enforcement** runs the harness on any branch touching `views/`. Drift cannot reach `dev`.

## Conventions (CONVENTIONS.md contents)

This is the contract-of-contracts. Every file under `elohim/sdk/schemas/v1/views/` follows these rules. Violations fail the validation harness.

### Field names

- All field names are camelCase. No exceptions.
- Boolean fields use positive phrasing: `caughtUp`, not `notCaughtUp`.
- Predicate naming: `isReady`, `hasDrain`, `viaRelay`. The `is`/`has` prefix is optional for clearly-boolean nouns.

### Enum values

- All enum values are lowercase string literals. No PascalCase. Kebab-case only if the value is genuinely compound, and even then prefer two enums over a compound.
- Enum schemas live in `elohim/sdk/schemas/v1/views/enums/` and are referenced via `$ref`. Reusable across views.

### Numeric types

- **Counts and bounded sizes** are `"type": "number"` (Rust `usize`/`i32`, TypeScript `number`). Use when the value is guaranteed below 2^53.
- **Byte counts and monotonic counters** that could exceed 2^53 use the BigInt-as-string pattern: `{"type": "string", "pattern": "^[0-9]+$"}`. Rust `u64` with `serde_with::DisplayFromStr` or manual serializer. TypeScript `bigint` parsed at the adapter boundary.
- **Durations** are `"type": "number"` measured in milliseconds unless documented otherwise. Field name carries the unit suffix: `uptimeSeconds`, `observedRttMs`.
- **When in doubt, use BigInt.** The cost of over-conservatism is cheap; the cost of a silent truncation bug at 2^53 is a 2am debugging session.

### Timestamps

- All timestamps are `"type": "string"` with `"format": "date-time"` (ISO-8601, UTC). No Unix epoch numbers, no naive datetimes, no millisecond integers.
- Field name does NOT carry a unit suffix (the format implies it): `connectedAt`, `lastSeen`.

### Nullability

- Nullable fields use `"type": ["string", "null"]` (or equivalent). Rust generates `Option<T>`. TypeScript generates `T | null`.
- Nullable means "this data is unavailable for a known reason." A nullable field's `description` MUST explain what causes null.
- Default to non-nullable. Add nullability deliberately.

### Source of truth declaration (mandatory)

- Every view schema MUST include a top-level `description` field that explicitly declares its source of truth and its P2P design gate classification. Format:

  ```
  "description": "Category C — Operational. Source of truth: <where the real state lives>. Reconstructed per request. Not persisted, not notarized, no DHT entry type."
  ```

- The validation harness fails any schema missing this declaration.
- This rule exists because the P2P design gate hook caught this exact omission during the drafting of this design. Making the declaration mandatory at the schema level prevents future agents from skipping it.
- For the rare case of a Category A view (projection of notarized DHT state — not present in this plan but possible for future views), the declaration form is: `"description": "Category A — Notarized projection. Source of truth: DHT entry type <name> in <zome>. SQLite projection has dht_anchor_hash. Reconstructable from DHT."`

### File layout

- One schema file per view: `<entity>-view.schema.json` in `elohim/sdk/schemas/v1/views/`.
- Nested types that are reused get their own file, referenced via `$ref`.
- Enums live in `elohim/sdk/schemas/v1/views/enums/<name>.schema.json`.
- Generated Rust output: `elohim/elohim-storage/src/generated/views/` (committed for visibility).
- Generated TypeScript output: `elohim/sdk/storage-client-ts/src/generated/views/` (next to the existing ts-rs-generated directory; eventually replaces it for migrated types).

### Closed contracts

- All view schemas declare `"additionalProperties": false`. The contract is closed by default.
- Adding a new field requires updating the schema first.

### Versioning

- The path encodes the major version: `elohim/sdk/schemas/v1/`. Breaking changes go to `v2/`. Minor additions (new optional fields) happen in place.

### Descriptions

- Every field has a non-empty `description`. Schemas without descriptions fail the validation harness.
- The description is the human contract; the type is the machine contract.

### Pagination

- Paginated list responses use a standard envelope: `{ "items": [...], "pagination": { "nextCursor": string | null, "hasMore": boolean } }`.
- The `pagination` fragment is a shared schema at `elohim/sdk/schemas/v1/views/enums/pagination.schema.json` (even though it's not strictly an enum, it lives in the shared-fragments directory).
- Cursors are opaque. The contract documents "treat as string, pass back as received." Implementations can change the cursor format without breaking callers.
- Paginated endpoints accept `?after=<cursor>&limit=N`. Default limit and max limit are documented per endpoint.

## Schema File Inventory

Files added by this plan:

```
elohim/sdk/schemas/v1/views/
├── CONVENTIONS.md                              (new)
├── p2p-status-view.schema.json                 (new)
├── peer-info-view.schema.json                  (new)
├── peer-list-view.schema.json                  (new — paginated wrapper around PeerInfo)
├── drain-status-view.schema.json               (new)
├── replication-status-view.schema.json         (new)
└── enums/
    ├── nat-status.schema.json                  (new)
    ├── relay-mode.schema.json                  (new — values TBD in Phase 2)
    ├── connection-direction.schema.json        (new)
    └── pagination.schema.json                  (new — reusable fragment)
```

### p2p-status-view.schema.json — field inventory

Existing fields (migrated from snake to camel):
- `peerId: string`
- `listenAddresses: string[]`
- `connectedPeers: number`
- `bootstrapNodes: string[]`
- `syncDocuments: number`
- `natStatus: enums/nat-status` (was loose string)
- `relayReservations: number`
- `announceAddresses: string[]`
- `relayMode: enums/relay-mode` (was loose string)
- `replication: replication-status-view` (nested, non-nullable)
- `drain: drain-status-view | null` (nullable; null means DB pool unavailable — NOT "caught up")

New fields (added by this plan):
- `uptimeSeconds: number` — node uptime since process start
- `storageBytesAvailable: string (bigint)` — free disk on storage volume
- `storageBytesUsed: string (bigint)` — bytes consumed by blob store + DB
- `capabilities: string[]` — protocol IDs this node serves (e.g. `["epr", "shard", "feed", "dht"]`)
- `version: string` — CARGO_PKG_VERSION of elohim-storage
- `agentVersion: string` — libp2p identify agent_version string

Schema-level `description` documents the canonical `isReady` derivation:

```
isReady(status) =
  status.replication.caughtUp
  && (status.drain == null || status.drain.pending == 0)
```

Consumers compute this client-side; the server does not expose a precomputed field.

### peer-info-view.schema.json — field inventory

Tier 1 (cheap, from Swarm state):
- `peerId: string`
- `addresses: string[]`
- `direction: enums/connection-direction`
- `viaRelay: boolean`
- `connectedAt: string (date-time)`

Tier 2 (cheap, from libp2p identify protocol):
- `agentVersion: string`
- `protocols: string[]`
- `observedAddress: string | null` (null when peer hasn't reported one)

Tier 3 (requires new plumbing, nullable until populated):
- `observedRttMs: number | null` (null until ping plumbing lands in Phase 7)
- `lastSeen: string | null (date-time)` (null until last-seen hook lands in Phase 8)
- `remoteNatStatus: enums/nat-status | null` (null until dial-back tracking lands in Phase 9)
- `bytesIn: string (bigint) | null` (null until bandwidth counters land in Phase 10)
- `bytesOut: string (bigint) | null` (null until bandwidth counters land in Phase 10)

### peer-list-view.schema.json

```
{
  "items": peer-info-view[],
  "pagination": enums/pagination
}
```

## Validation Harness

Location: `elohim/elohim-storage/tests/schema_contract.rs` (integration test, runs on `cargo test`).

Mechanism:

1. Test discovery walks `elohim/sdk/schemas/v1/views/` and finds every `*.schema.json`.
2. For each schema, looks up a registered `Fixture` implementation via a trait.
3. Instantiates the fixture, serializes to JSON via serde, validates against the schema using the `jsonschema` crate.
4. Fails loudly on any drift: missing field, extra field, wrong type, nullable where non-nullable declared, enum value not in schema's allowed set, etc.
5. Also fails if a schema file has no registered fixture (ensures every view is covered).

Fixture trait:

```rust
trait SchemaFixture {
    const SCHEMA_PATH: &'static str;
    fn fixture() -> Self;
}
```

Each migrated view type implements `SchemaFixture`. Fixtures are deliberately non-default: populate nullable fields, set enums to non-default values, make arrays non-empty. The goal is to exercise every schema path.

Harness runs in:
- `cargo test` (local development, CI via existing storage pipeline)
- Pre-push hook on any branch touching `elohim/sdk/schemas/v1/views/` or `elohim/elohim-storage/src/generated/views/`

Starts local to `elohim-storage/tests/`. `TODO(shared-crate)` comment in the harness file flags eventual extraction to a shared `elohim-schemas` crate once a second consumer exists.

## Phase Breakdown

**Phases are logical execution units, not commit boundaries.** The entire sprint — this design doc plus all 12 phases of implementation — lands as one commit (or a small handful of logical commits if the total diff is genuinely too large to review as one) on `dev` at the end of the sprint. No per-phase commits, no feature branch unless explicitly requested. Each phase has exit criteria so the sprint can be paused between phases with confidence, but the git history reflects the sprint as a single unit of completed work.

### Phase 0 — Conventions + audit (no P2P code yet)

- Write `elohim/sdk/schemas/v1/views/CONVENTIONS.md` with the rules from this design.
- Audit `content-view.schema.json` and `economic-event-view.schema.json` for conformance. Fix any drift.
- Add the IoC pattern to `CLAUDE.md` (or a new `docs/contracts.md` linked from CLAUDE.md): "JSON Schema is the source of truth for HTTP wire shapes. Rust generates from schema. CI validates. Add a new schema → it's automatically discovered."
- Add pre-push hook: run schema validation tests on any branch that touches `elohim/sdk/schemas/v1/views/`.
- Feasibility spike: write one trivial toy schema, run `pnpm run schema:codegen:rs`, confirm the output Rust is usable. Exercises: nested `$ref` across files, enum-of-string-literals, `additionalProperties: false`, BigInt-as-string pattern.
- **Exit criteria:** CONVENTIONS.md committed, existing views conform, codegen handles required features, pre-push hook wired.

### Phase 1 — Build the reusable validation harness

- Add `elohim/elohim-storage/tests/schema_contract.rs`.
- Implement schema discovery, `SchemaFixture` trait, `jsonschema` validation integration.
- Register the existing two views (`content-view`, `economic-event-view`) as the harness's first test cases with fixtures.
- Harness must go green against existing views before proceeding.
- **Exit criteria:** `cargo test schema_contract` passes with two views covered.

### Phase 2 — P2P schema files (no Rust changes yet)

- Add all ten new schema files listed in the inventory.
- Schemas are written as the destination shape: camelCase, lowercase enums, all new self-status fields, full PeerInfo, BigInt patterns, closed contracts, descriptions.
- Check actual values of libp2p's `RelayMode` and populate `relay-mode.schema.json` accordingly (flagged as an explicit task here because the current `{:?}` stringification may not match what we'd pick as enum values).
- Validation harness picks up the new schemas and fails CI because there's no Rust counterpart yet. **This failure is the checklist for Phase 3.**
- **Exit criteria:** Schema files committed, harness fails with "no fixture registered for p2p-status-view" etc.

### Phase 3 — Rust regen + struct replacement + consumer migration

- Run `pnpm run schema:codegen:rs`, generate Rust types for P2P views.
- Delete hand-written `P2PStatusInfo`, `DrainStatusInfo`, `ReplicationStatus` from `elohim/elohim-storage/src/p2p/mod.rs` and `p2p/replication.rs`. Replace call sites with imports from the generated module. (This deletion removes the `#[derive(TS)]` and `#[ts(export)]` attributes along with the structs themselves — there is nothing separate to clean up afterward. Confirm the ts-rs export script no longer references these types.)
- Update the HTTP handler to construct the new generated types. Field names change from snake to camel in the same commit.
- Update seeder import (hc-rna-fixtures) from `@elohim/storage-client`'s ts-rs-generated type to the new schema-generated type in `elohim/sdk/storage-client-ts/src/generated/views/`.
- Update all 5 consumers in the same commit:
  - doorway-service federation module (Rust, type-checked)
  - doorway-service main module (Rust, type-checked)
  - doorway-service server module (Rust, type-checked)
  - elohim-app connection-indicator (TypeScript, type-checked via schema-generated types)
  - simulate.sh (shell script, `jq` path updates)
  - genesis Jenkinsfile (shell script, `jq` path updates)
- Register `SchemaFixture` for the three P2P view types in the harness.
- Harness goes green.
- **Exit criteria:** `cargo test` passes, `pnpm test` passes in app, pre-push hook passes, manual smoke against a running storage node confirms camelCase wire shape.

### Phase 4 — Enum formalization

- `NatStatus`, `RelayMode`, `ConnectionDirection` become proper Rust enums (generated from schema).
- Wire serialization switches from `{:?}` to `serde(rename = "lowercase")` or equivalent via the generated code.
- Consumer pattern-matches update to use the typed enums where possible.
- **Exit criteria:** No remaining `{:?}` formatting for these enums. Harness passes. Type-level enforcement of valid values.

### Phase 5 — Populate self-status field values

- The six new self-status fields were already declared in the schema in Phase 2. This phase adds the Rust implementation that populates them at request time.
- Rust handler populates: `uptimeSeconds` (from node start time captured at boot), `storageBytesAvailable`/`storageBytesUsed` (statvfs + blob store + DB query, serialized as BigInt-as-string per conventions), `capabilities` (derived from which protocol handlers are registered), `version` (`env!("CARGO_PKG_VERSION")`), `agentVersion` (libp2p identify agent_version string, mirrors what peers see about us).
- Schema-level `description` documents the `isReady` derivation for consumers.
- Update the `SchemaFixture` for `p2p-status-view` to populate all six new fields with non-default values.
- **Exit criteria:** `/p2p/status` response includes all six new fields populated with real values (not placeholders). Harness passes.

### Phase 6 — PeerInfo Tier 1+2 + new endpoint

- Add `GET /p2p/peers` handler in `elohim-storage/src/http.rs`.
- Handler queries Swarm state for Tier 1 fields: peerId, addresses, direction, viaRelay, connectedAt.
- Handler queries libp2p identify state for Tier 2 fields: agentVersion, protocols, observedAddress.
- Tier 3 fields are populated as `null` in the response.
- Accept `?after=<cursor>&limit=N` query params. Default limit = 500. Max limit = 500. Stable ordering by peerId ascending. Opaque cursor implementation (currently the last returned peerId, but contract doesn't guarantee that).
- Register `SchemaFixture` for peer-info-view and peer-list-view.
- **Exit criteria:** `GET /p2p/peers` returns a valid paginated response. Harness passes. Manual smoke confirms pagination behavior.

### Phase 7 — Tier 3 plumbing: RTT

- Enable libp2p-ping protocol if not already active (verify).
- Maintain per-peer ring buffer of ping samples.
- Surface latest sample (or rolling average — decided in this phase, not now) as `observedRttMs` on PeerInfo.
- Update fixture to populate the field.
- **Exit criteria:** `observedRttMs` is non-null for peers with at least one ping sample. Harness passes.

### Phase 8 — Tier 3 plumbing: lastSeen

- Hook a timestamp updater into inbound message paths (or use libp2p's connection events).
- Per-peer map from `PeerId` to most recent `SystemTime`.
- Surface as `lastSeen` on PeerInfo.
- **Exit criteria:** `lastSeen` is non-null for any peer we've received a message from. Harness passes.

### Phase 9 — Tier 3 plumbing: remoteNatStatus

- Track dial-back attempts per peer (success/failure).
- Derive per-peer NAT status from dial-back outcomes.
- Surface as `remoteNatStatus` on PeerInfo.
- **Exit criteria:** `remoteNatStatus` populated for peers with at least one dial-back attempt. Harness passes.

### Phase 10 — Tier 3 plumbing: bandwidth counters

- Wire libp2p `BandwidthSinks` per peer.
- Track cumulative `bytesIn` and `bytesOut` per `PeerId`.
- Serialize as BigInt-as-string per conventions.
- **Exit criteria:** `bytesIn`/`bytesOut` populated and monotonically increasing over connection lifetime. Harness passes.

### Phase 11 — Documentation + cleanup

- Update CLAUDE.md with the now-proven IoC pattern: "to add a new view, do these 4 things." Link to `elohim/sdk/schemas/v1/views/CONVENTIONS.md` as the canonical reference.
- Document the `GET /p2p/peers` endpoint in any existing HTTP API docs.
- Consider whether a memory entry is warranted to capture any non-obvious lessons discovered during implementation (surprises in the libp2p API, unexpected consumer migrations, harness quirks). Not required — only add if there's something worth preserving for future sessions.
- Close any `TODO(shared-crate)` items if a second consumer has emerged during implementation; otherwise leave them for a follow-up.
- **Exit criteria:** Docs updated, CLAUDE.md references CONVENTIONS.md, plan marked complete.

## Consumer Migration Detail

Phase 0 runs a full grep across the repo:

```bash
grep -rE "connected_peers|nat_status|relay_mode|relay_reservations|sync_documents|peer_id|listen_addresses|bootstrap_nodes|announce_addresses" \
  --include='*.ts' --include='*.rs' --include='*.sh' --include='Jenkinsfile*' --include='*.md'
```

Expected hits (from prior knowledge):

1. `doorway/doorway-service/src/federation/**/*.rs` — P2P status consumer
2. `doorway/doorway-service/src/main.rs` — P2P status consumer
3. `doorway/doorway-service/src/server/**/*.rs` — P2P status consumer
4. `app/elohim-app/src/app/**/connection-indicator*.ts` — green-dot UI
5. `simulate.sh` (or wherever it lives) — shell integration
6. `genesis/orchestrator/Jenkinsfile` and possibly other Jenkinsfiles — CI/CD
7. `elohim/elohim-storage/src/` and seeder code for the E1 integration

Any unexpected hit becomes either an additional migration target or an "out of scope — pre-existing debt" note.

## Testing Strategy

- **Validation harness (Phase 1):** Primary contract enforcement. Runs on `cargo test` and pre-push.
- **Unit tests for P2P handlers (Phase 3 onward):** Standard Rust unit tests for handler logic. No schema involvement — the harness covers schema conformance separately.
- **Integration tests against a running storage node:** For Phase 6+ (new endpoint), add integration tests that spin up storage and hit `/p2p/peers` over HTTP. Verify pagination, cursor behavior, enum values.
- **Smoke test against live dev stack (Phase 3):** Manual verification that the camelCase migration doesn't break any consumer on the actual wire. Run via the `hc-dev-orchestrator` skill once the branch is assembled.
- **Angular unit tests for connection-indicator (Phase 3):** Update existing Vitest tests to use the new field names. No new test cases — just a mechanical migration.
- **Jenkins pipeline smoke:** Let the genesis pipeline run its full build on the branch once before merging, to catch any Jenkinsfile `jq` path that was missed.

## Risks and Mitigations

1. **Schema→Rust codegen doesn't support all required features.** Mitigated by Phase 0 feasibility spike. If any feature is missing, Phase 0 extends to include codegen work.
2. **Existing content-view / economic-event-view don't conform to conventions.** Mitigated by Phase 0 audit. Non-conforming schemas get fixed in Phase 0.
3. **Tier 3 plumbing (Phases 7-10) uncovers libp2p API surprises.** Mitigated by making each Tier 3 phase independent. If RTT plumbing stalls, the rest of the plan still ships; the schema already declares those fields as nullable.
4. **Consumer migration misses a caller.** Mitigated by Phase 0 grep and harness runtime enforcement. Shell/jq consumers caught by smoke test and Jenkins pipeline run.
5. **Scope creep during CONVENTIONS.md writing.** Mitigated by the rule: "CONVENTIONS.md is limited to rules that have an automated enforcement mechanism." If a rule can't be enforced, it's a PR comment, not a line in the doc.
6. **The sprint grows too large across 12 phases.** Mitigated by each phase having independent exit criteria, so the sprint can be paused between phases if needed. If the total diff becomes genuinely unreviewable as one commit at sprint end, phases 7-11 (Tier 3 plumbing) can defer to a follow-up sprint without structural change — the schema contract and Phases 0-6 are the load-bearing win, and Tier 3 fields are schema-declared as nullable so deferring their population is a valid end state.
7. **BigInt-as-string pattern surprises TypeScript consumers.** Mitigated by documenting the pattern in CONVENTIONS.md and handling the parse at the adapter boundary. UI code sees `bigint` natively, never touches the string form.

## Open Questions Deferred to Implementation

These are deliberately left for the implementation plan or individual phases rather than being pinned in this design:

- **Exact enum values for `relay-mode`:** depends on what libp2p's `RelayMode` actually stringifies to today and what we want the canonical values to be. Resolved in Phase 2.
- **RTT representation:** single latest sample, rolling average, p50/p99? Resolved in Phase 7 with real data in hand.
- **Whether to publish Rust-generated view types to a shared crate:** deferred until a second consumer emerges. Flagged with `TODO(shared-crate)` in Phase 1.
- **Whether `observedAddress` should be an array** (libp2p may report multiple): checked in Phase 2 against the identify protocol's actual shape.

## Success Criteria

The plan is done when all of the following are true:

1. `elohim/sdk/schemas/v1/views/CONVENTIONS.md` exists and is referenced from CLAUDE.md.
2. `p2p-status-view`, `peer-info-view`, `peer-list-view`, `drain-status-view`, `replication-status-view`, and all four enum schemas are committed under `elohim/sdk/schemas/v1/views/`.
3. `P2PStatusInfo`, `DrainStatusInfo`, `ReplicationStatus` no longer exist as hand-written types. They are imported from the generated module.
4. `GET /p2p/peers?after=<cursor>&limit=N` returns a valid paginated `peer-list-view` response.
5. All Tier 1, Tier 2, and Tier 3 PeerInfo fields are populated and non-null for appropriate peers.
6. The validation harness (`cargo test schema_contract`) passes and is wired into the pre-push hook.
7. All 5+ known consumers of `/p2p/status` are migrated to camelCase and typed enums.
8. A future agent can add a new view by following CONVENTIONS.md and the CLAUDE.md pointer, with no need to re-derive any of the decisions in this design.
