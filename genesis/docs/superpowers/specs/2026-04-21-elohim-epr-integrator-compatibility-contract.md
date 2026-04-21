# elohim-epr Integrator Compatibility Contract

**Status:** Contract — authoritative for Phase 2+ planning
**Date:** 2026-04-21
**Authors:** Matthew Dowell + Opus 4.7
**Applies to:** every phase of the elohim-core graph substrate that ships wire shapes, storage schemas, REST endpoints, or GraphQL surfaces
**Parent spec:** `genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md`
**Phase 1 companion:** `genesis/docs/superpowers/plans/2026-04-21-elohim-epr-codec-crate-plan.md`

---

## Why this document exists

The protocol has two classes of consumers:

- **REST integrators** — apps that fetch EPRs by CID, read pillar projections, and expect stable wire shapes across versions. Today this is elohim-app, genesis-seeder, and every future hApp that talks to elohim-storage over HTTP.
- **Graph integrators** — apps that traverse relationships, subscribe to changes, and compose subgraphs. Today this surface is unrealized; Phase 4+ delivers it.

Both classes must coexist. A change that benefits one must not break the other. A new feature that ships on one must be discoverable on the other when it makes sense. And both classes must enjoy **compiler-level confidence** that the wire shape their code expects is the wire shape the protocol actually delivers.

This contract defines that guarantee.

---

## 1. Schema-first IoC enforcement chain

**Rule:** for any wire contract, a JSON schema is authored FIRST. Rust structs, TypeScript interfaces, diesel migrations, GraphQL schemas, and manifest entries all COMPLY with that schema. Nothing generates the schema; the schema generates everything.

This matches the memory rule `feedback_schema_first_ioc.md` and extends it to the six enforcement layers below.

### The six layers

| # | Layer | What it catches | Failure surface |
|---|---|---|---|
| 1 | **JSON schema** (authoritative) | structural drift, missing fields, type mismatch | `jsonschema` / `ajv` validation at build time and runtime |
| 2 | **Rust struct + serde** | field-name/type mismatch at Rust compile time | `cargo check` |
| 3 | **`schema_contract.rs` test** | Rust struct serializes to JSON that the schema rejects | `cargo test` |
| 4 | **TypeScript interface (generated)** | TS consumer uses a field name the Rust side doesn't produce | `tsc --strict` |
| 5 | **Golden vector / fixture** | canonical bytes change across library upgrades | `cargo test` + `pnpm test` |
| 6 | **Cross-language interop test** | Rust↔TS verification diverges despite all of the above passing | `pnpm test` (reads Rust-generated fixtures) |

### What each layer proves

- **Layers 1–3** guarantee the Rust side matches the schema contract.
- **Layers 4–5** guarantee the TS side matches the Rust side.
- **Layer 6** guarantees the full stack delivers the same atom end-to-end.

Any single layer missing means implementation confidence collapses to "the tests we happened to write." All six together mean every new integrator gets the wire shape for free, verified by the toolchain, with zero guessing.

### How this maps to existing elohim SDK

The SDK already instantiates layers 1–3 for elohim-storage views:

- `elohim/sdk/schemas/v1/views/*.schema.json` — layer 1
- `elohim/elohim-storage/src/views.rs` with `#[serde(rename_all = "camelCase")]` — layer 2
- `elohim/elohim-storage/tests/schema_contract.rs` — layer 3

TS codegen via `elohim/sdk/storage-client-ts/` derives from ts-rs output of the Rust structs — layer 4 is present but as a *consumer* of the Rust layer, not a direct consumer of the JSON schema. Layers 5–6 exist implicitly in integration tests but are not formalized per-view.

### What Phase 1 (elohim-epr) DID ship

| Layer | Status | Evidence |
|---|:---:|---|
| 1 | ❌ | No JSON schemas for Envelope / Coupling / Signature / EprKind / Reach / CouplingLeg / Epr |
| 2 | ✅ | `elohim/epr/src/{envelope,coupling,signature,kind,reach}.rs` with serde + ts-rs |
| 3 | ❌ | No `schema_contract.rs` — no schema to contract against |
| 4 | ✅ (by proxy) | Generated TS interfaces at `elohim/sdk/epr-ts/src/generated/` |
| 5 | ✅ | Golden vector in `elohim/epr/tests/canonical_bytes.rs` + `elohim/epr/tests/vectors/signed_eprs.json` |
| 6 | ✅ | `elohim/sdk/epr-ts/tests/interop.test.ts` |

**Gap:** layers 1 and 3 are missing for the EPR types. The Phase 1 code-first approach produced a working codec but deviated from the schema-first IoC chain the SDK commits to elsewhere.

### Backfill required before Phase 2

As the first task of Phase 2 (or as a standalone precondition), write JSON schemas for the Phase 1 types and add a `schema_contract.rs` test:

**Schema files (create in `elohim/sdk/schemas/v1/objects/`):**
- `envelope.schema.json`
- `coupling.schema.json`
- `signature.schema.json`
- `epr.schema.json`

**Enum schemas (create in `elohim/sdk/schemas/v1/enums/`):**
- `epr-kind.schema.json`
- `coupling-leg.schema.json`
- (Reach is a new envelope-level enum — add `reach.schema.json` here too)

**Contract test (create in `elohim/epr/tests/`):**
- `schema_contract.rs` — for each Rust struct (Envelope, Coupling, Signature, Epr) and each enum (EprKind, CouplingLeg, Reach), generate a valid instance, serialize to JSON, validate against the schema. Fail loudly on any mismatch.

**Source-of-truth declaration** (per `elohim/sdk/schemas/v1/views/CONVENTIONS.md`):

EPRs are a new p2p-design-gate archetype — **self-notarized via content-address + signature**, stored on elohim-storage as a projection. The schemas must declare this. Suggested language:

```json
{
  "description": "Source of truth: EPR atom is self-notarized via content-address (CIDv1 dag-cbor sha256) + Ed25519 signature. Canonical bytes + signature is the authoritative form; elohim-storage persists it as a projection. Category: A-prime (self-notarized atom, not DHT-notarized)."
}
```

If the p2p-design-gate skill needs a new category `A'` (A-prime) to cover self-notarized content-addressed atoms, propose the addition as part of the backfill task.

---

## 2. Phase 2 compatibility guarantee

Phase 2 ships: `epr_atoms` + `epr_coupling` + `epr_claims` + `epr_supersedence` diesel tables, the 4-stage validator (stages 1–3; stage 4 deferred to Phase 3), and new REST endpoints for EPR access.

**Contract:** every existing REST consumer MUST continue to work unchanged.

### 2.1 Stable wire-shape endpoints (must not change)

The following endpoints and their response shapes are locked for Phase 2. Contract tests MUST prove pre-vs-post byte-identical responses for a captured fixture per endpoint. Any change fails the Phase 2 acceptance gate.

| Endpoint group | Why stable | Contract test approach |
|---|---|---|
| `GET /content/...` endpoints | elohim-app depends on `ContentView` shape | capture response fixture before Phase 2, replay after, diff byte-for-byte |
| `GET /economic-events/...` | Signal Harness produces these; genesis-seeder reads them | same |
| `GET /learning-paths/...` | lamad pillar services | same |
| `GET /humans/...`, `/presences/...` | imagodei services | same |
| `GET /manifests/...` (existing) | domain manifests live here | same |
| `GET /elohim-reputation-profile/...`, `/peer-status/...`, `/peer-info/...`, etc. | 20+ existing views | same — iterate over every schema in `elohim/sdk/schemas/v1/views/` |

**Mechanism:** Phase 2 introduces a kind-aware projector that writes from `epr_atoms` into the existing pillar tables. The projection MUST produce JSON byte-identical to what the current direct-write path produces. Until this is proven, Phase 2 runs in write-through mode: both old write path AND EPR path populate the pillar table, with the old path remaining authoritative.

**Phase 2 Done-When:** for every pillar-view schema, a `schema_contract_pre_post.rs` test generates a fixture under pre-Phase-2 behavior, captures bytes, switches to projector-only mode, regenerates, and asserts byte equality.

### 2.2 New REST endpoints (additive)

Phase 2 adds the following. All are net-new; no existing endpoint URI collides.

| Endpoint | Response shape | Schema file |
|---|---|---|
| `GET /epr/{cid}` | full `Epr` (envelope + payload bytes + canonical bytes metadata) | `elohim/sdk/schemas/v1/views/epr-view.schema.json` |
| `GET /epr/{cid}/envelope` | `Envelope` only | `elohim/sdk/schemas/v1/views/epr-envelope-view.schema.json` |
| `GET /epr/{cid}/payload` | raw payload bytes (`Content-Type` from manifest schema lookup) | (binary) |
| `GET /epr/{cid}/verify` | `{ok: true}` or structured error | `elohim/sdk/schemas/v1/views/epr-verify-view.schema.json` |
| `POST /epr` | `{cid: "..."}` on success | `elohim/sdk/schemas/v1/inputs/epr-publish-input.schema.json` |
| `GET /epr?kind=&reach=&schemaRef=` | paged list of `Envelope` refs | `elohim/sdk/schemas/v1/views/epr-list-view.schema.json` |

Every new endpoint:
1. Has a JSON schema FIRST (layer 1 of the IoC chain)
2. Has a Rust view struct with `#[serde(rename_all = "camelCase")]` (layer 2)
3. Has a `schema_contract.rs` test (layer 3)
4. Has a ts-rs export flowing to `@elohim/storage-client` (layer 4)
5. Has a golden-response fixture (layer 5)
6. Has a TS integration test verifying the Rust → TS wire (layer 6)

**No endpoint ships without all six layers.** This is the non-negotiable IoC floor for Phase 2.

### 2.3 Schema-first for diesel

Diesel migrations for `epr_atoms` / `epr_coupling` / `epr_claims` / `epr_supersedence` MUST derive from the JSON schemas:

- Column names match JSON schema property names in snake_case (diesel convention) with `#[serde(rename_all = "camelCase")]` on the Rust projection model to convert back to camelCase at the wire layer.
- Column types derive from JSON schema types via a lookup table (`string → TEXT`, `integer → BIGINT`, `array → JSONB`, etc.) documented in the Phase 2 plan.
- A new `schema_contract_diesel.rs` test asserts that the `diesel::schema::epr_atoms::columns` exactly match the JSON schema properties — additions, removals, or renames on either side fail the build.

### 2.4 Signal Harness migration

The existing Signal Harness emits `EconomicEvent` records directly. Phase 2 flips it to emit `EconomicEvent` EPRs that are then projected into the existing `economic_events` table.

**Contract:** the `EconomicEventView` JSON schema does NOT change. The downstream `economic_events` table columns do NOT change. Only the WRITE path changes, and only behind a feature flag (`VITE_EPR_WRITE_THROUGH=true`) until contract tests prove parity. Default state during Phase 2: flag OFF, old path authoritative, EPR path shadows.

### 2.5 Reach backfill

Existing content rows don't have a `reach` field. Phase 2 must define:

- **Backfill policy:** default value for existing rows. Suggested: `collective` (conservative: existing content was implicitly group-scoped, not broadcast). NOT `commons` (that would widen visibility without consent).
- **Schema requirement:** `reach` becomes required on NEW writes via Phase 2's validator. Existing rows are grandfathered.

The Phase 2 plan MUST call out this decision explicitly and document it in a new ADR (`genesis/docs/superpowers/specs/decisions/`).

---

## 3. Forbidden patterns (code-review flags)

Phase 2 plans and PRs should be rejected if they contain any of these:

| Anti-pattern | Why forbidden | Correct approach |
|---|---|---|
| Hand-writing a Rust struct without a corresponding JSON schema | Breaks layer 1 of IoC chain | Write schema first in `elohim/sdk/schemas/v1/`, then Rust struct |
| Hand-writing a TypeScript interface | Breaks layer 4 — TS must be generated | Use ts-rs on Rust, OR JSON schema → `json-schema-to-typescript`, never both |
| Defining a new view without `schema_contract.rs` entry | Breaks layer 3 | Add to the contract test suite as part of the same PR |
| Modifying existing view field names/shapes | Breaks Phase 2 compatibility contract | Add NEW view; old view stays |
| Changing response shape in write-through mode | Breaks byte-identical contract | Capture pre-fixture, assert equality, never mutate |
| Removing source-of-truth declaration from schema `description` | Breaks `CONVENTIONS.md` rule 2 | Every schema declares its p2p-design-gate category |
| Skipping the golden vector / interop test for a new wire shape | Breaks layers 5–6 | No endpoint ships without both |
| Introducing `additionalProperties: true` on a view schema | Breaks `CONVENTIONS.md` rule 3 | Tight contracts only; add fields explicitly |

---

## 4. Graph-side readiness (Phase 3+ pre-commitment)

Phase 2 doesn't ship GraphQL, but the EPR storage it builds must be ready for graph access without retrofitting. Contract:

- **Indexes on `epr_atoms`:** `(kind, schema_ref)`, `(reach)`, `(signer_cid)`, `(supersedes)`. These support both REST list-queries and future GraphQL traversal without additional migration.
- **`epr_coupling` supports traversal:** the Phase 2 schema MUST make coupling refs first-class rows (not JSON columns inside `epr_atoms`), so Phase 4 GraphQL resolvers can join on `(epr_cid, leg) → target_cid` in SQL.
- **CID as primary key:** `epr_atoms.cid` is TEXT PRIMARY KEY (CIDv1 string form). Every foreign key to an EPR is `TEXT` holding a CIDv1. This means Phase 4 can build GraphQL `@key(fields: "cid")` directives directly over the existing schema.
- **No ID column.** No `epr_atoms.id BIGSERIAL`. CIDs are the identity; introducing a surrogate integer ID would pollute the graph model with storage-local identifiers and break `epr-content-addressing` principles.

---

## 5. Integrator test charter

Every merge to `dev` that touches a wire-boundary file MUST pass the following contract suite. This is what "IoC confidence" means operationally.

### Rust side (invoked by `cargo test -p elohim-epr` and `cargo test -p elohim-storage`)

- `schema_contract.rs` — Rust struct ↔ JSON schema round-trip
- `schema_contract_pre_post.rs` — pre-vs-post Phase 2 byte-identical response fixtures (Phase 2 ships this)
- `schema_contract_diesel.rs` — diesel columns ↔ JSON schema properties alignment
- `canonical_bytes.rs` golden vector — locks CBOR encoding
- `vector_roundtrip.rs` — Rust re-verifies its own output
- RFC 8032 test vector — Ed25519 implementation drift guard

### TypeScript side (invoked by `pnpm test` at `elohim/sdk/epr-ts/` and `elohim/sdk/storage-client-ts/`)

- `cbor.test.ts` — canonical CBOR round-trip
- `cid.test.ts` — CID derivation matches Rust vectors
- `proof.test.ts` — Ed25519 verify matches Rust vectors
- `envelope.test.ts` — canonical envelope bytes match Rust byte-for-byte
- `interop.test.ts` — end-to-end: every Rust vector verifies via TS
- Per-view schema contract tests — `json-schema-to-typescript` output matches ts-rs output (where both exist)

### Pre-push hook + CI

Both layers run in the pre-push gate (`.husky/pre-push`) AND in the Jenkins pipeline (`elohim/epr/Jenkinsfile` + orchestrator). A wire-contract failure in either layer fails both.

---

## 6. Open questions for Phase 2 planning

These are NOT answered by this contract; the Phase 2 plan must address them:

1. **Who writes the backfill schemas?** Phase 2 plan Task 0 / Task 1 ships the JSON schemas for Phase 1 types. Should this be gated as a prerequisite separate from Phase 2's storage work?
2. **p2p-design-gate category for self-notarized atoms.** Does the gate skill need a new category (A', A+signature) or does existing Category A with modified language suffice?
3. **Diesel column ↔ schema property lookup table.** Which JSON schema primitive maps to which diesel type? This needs a one-time spec.
4. **Write-through feature flag format.** `VITE_EPR_WRITE_THROUGH` for frontend; what's the server-side equivalent? Env var, config.toml key, or agent-observed state (per the `project_elohim_active_observed_not_flagged.md` memory)?
5. **Reach backfill value.** Commit to `collective` default for existing rows, or defer to the ADR in Phase 2?
6. **Schema versioning.** When a schema evolves (e.g., new optional field), does the version bump live in the schema `$id`, the surrounding manifest, or both?

---

## 7. Summary

**Phase 2 is complementary to existing REST, not conflicting.** Every existing endpoint continues to work with byte-identical responses. New endpoints are additive. Storage grows an EPR layer underneath; pillar views become materialized projections. Graph-side readiness is pre-committed via index and FK choices so Phase 3–4 land cleanly.

**The IoC chain has six layers.** Phase 1 shipped 4 of 6 for elohim-epr. Phase 2 backfills layers 1 + 3 for EPR types and extends all six to every new REST endpoint. No endpoint ships without all six, ever.

**The protocol gains compile-time wire confidence.** Every integrator — REST, graph, P2P, Holochain hApp, external ValueFlows consumer — reads the same atom through different transports, verified by the same toolchain, with zero guessing at the implementation.

This is the foundation for the 20-year generational architecture the parent spec commits to.
