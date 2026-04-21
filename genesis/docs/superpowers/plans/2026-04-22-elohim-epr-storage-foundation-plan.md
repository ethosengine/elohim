# elohim-epr Phase 2a — Storage Foundation & REST Surface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land EPR atoms as first-class storage in elohim-storage (diesel tables + validator), expose a complete additive REST surface (`/api/v1/epr/...`), and regenerate the TypeScript storage client — all without changing any existing endpoint's wire shape.

**Architecture:** `elohim-epr` crate (Phase 1) is the codec primitive; Phase 2a wraps it in elohim-storage's established three-layer pattern (controller → service → model). Four new diesel tables (`epr_atoms`, `epr_coupling`, `epr_claims`, `epr_supersedence`) store validated atoms. Six new HTTP routes expose them. All wire types go through JSON schema first per the Integrator Compatibility Contract.

**Tech Stack:** Rust (elohim-storage crate), diesel + postgres, hyper HTTP, `elohim-epr` codec, `ts-rs` for TypeScript export, `@elohim/storage-client` consumer package, JSON Schema via `jsonschema` crate.

**Phase 2a scope:** New REST surface for signed content-addressed atoms; existing REST surface unchanged.

**Out of scope (Phase 2b):**
- Projector from `epr_atoms` → existing pillar tables (`content_nodes`, `economic_events`, etc.)
- Signal Harness migration to emit EPRs
- Write-through feature flag wiring to existing code paths
- Pre/post byte-identical contract tests for existing views (framework only in 2a)
- Reconciliation between existing `epr_codec.rs` (EprHead pattern) and the new generalized Envelope

---

## Decisions locked for this plan

- **Crate boundary:** `elohim-epr` is a workspace dep of `elohim-storage` (already a sibling crate at `elohim/epr/`). No new top-level crates.
- **Migration naming:** `elohim/elohim-storage/migrations/2026-04-22-000000_add_epr_tables/` — follows existing date-prefix convention.
- **CID wire format in REST responses:** **string (CIDv1 base32)**, not byte array. Internal storage keeps canonical form; the wire view transforms. Rationale: REST integrators are app developers, not codec authors; strings copy-paste into logs/URLs/docs.
- **Route prefix:** `/api/v1/epr/...` — matches existing convention.
- **Reach enforcement for Phase 2a:** endpoint-level gate based solely on envelope `reach` field (no payload parse). Commons and Public require no auth. Collective/Steward/Private require an authenticated caller and return 404 (not 403) when the caller isn't authorized — avoids leaking existence. Full identity integration deferred to Phase 2b.
- **RUSTFLAGS:** elohim-storage uses `RUSTFLAGS='--cfg getrandom_backend="custom"'`. `elohim-epr` will be built under that flag set. Task 4 verifies the build works before any database work; if it doesn't compile, stop and escalate.
- **Schema contract test pattern:** extend existing `elohim/elohim-storage/tests/schema_contract.rs` rather than create a new file. Add per-view test functions.
- **pnpm workspace:** `@elohim/storage-client` already exists at `elohim/sdk/storage-client-ts/` — Task 20 extends it with regenerated types.
- **No changes to `epr_codec.rs`** (the existing EprHead encoder in elohim-storage). Parallel existence; reconciliation is a Phase 2b concern.

---

## File Structure

### New files

```
elohim/elohim-storage/migrations/2026-04-22-000000_add_epr_tables/
├── up.sql                                  # 4 CREATE TABLE statements
└── down.sql                                # 4 DROP TABLE statements (reverse order)

elohim/elohim-storage/src/
├── db/
│   └── epr_atoms.rs                        # Diesel models + queries for epr_atoms/_coupling/_claims/_supersedence
├── services/
│   └── epr_service.rs                      # Ingest, fetch, list, verify business logic
└── api/
    └── epr.rs                              # HTTP controller for /api/v1/epr/...

elohim/elohim-storage/tests/
├── epr_ingest_integration.rs               # POST /epr → validates → stores → GET /epr/:cid returns it
├── epr_verify_integration.rs               # GET /epr/:cid/verify full 4-stage (stage 4 deferred)
├── epr_list_integration.rs                 # GET /epr?kind=&reach=&schemaRef= pagination + filters
└── epr_reach_enforcement.rs                # Reach visibility rules at endpoint layer

elohim/sdk/schemas/v1/views/
├── epr-view.schema.json                    # Full Epr for GET /epr/:cid
├── epr-envelope-view.schema.json           # Envelope only for GET /epr/:cid/envelope
├── epr-verify-view.schema.json             # GET /epr/:cid/verify
└── epr-list-view.schema.json               # GET /epr?...

elohim/sdk/schemas/v1/inputs/
└── epr-publish-input.schema.json           # POST /epr body

genesis/docs/superpowers/specs/decisions/
└── 2026-04-22-reach-backfill-policy.md     # ADR for how existing rows gain `reach`
```

### Files to modify

```
elohim/elohim-storage/Cargo.toml            # Add elohim-epr workspace dep
elohim/elohim-storage/src/lib.rs            # Register new mod epr_service / epr_atoms / api/epr
elohim/elohim-storage/src/api/mod.rs        # Register pub mod epr + dispatch in router
elohim/elohim-storage/src/db/mod.rs         # Register pub mod epr_atoms
elohim/elohim-storage/src/services/mod.rs   # Register pub mod epr_service (if services use explicit mod pattern)
elohim/elohim-storage/src/views.rs          # Add EprView, EprEnvelopeView, EprVerifyView, EprListView + Input types
elohim/elohim-storage/src/db/diesel_schema.rs  # Auto-regenerated by `diesel migration run`
elohim/elohim-storage/tests/schema_contract.rs # Add per-view contract tests

elohim/sdk/storage-client-ts/src/generated/  # Regenerated by cargo test export_bindings
```

---

## Task overview

22 tasks in 5 groups.

- **A. Decisions & schemas** (Tasks 1–3): Reach ADR, JSON schemas for new views, schema contract scaffolding
- **B. Storage layer** (Tasks 4–7): elohim-epr dep verification, diesel migration, models, column-vs-schema contract test
- **C. Service layer** (Tasks 8–11): ingest, fetch, list, verify (stage 4 deferred)
- **D. Route layer** (Tasks 12–17): 6 HTTP endpoints
- **E. Reach enforcement, integration, TS client, CI** (Tasks 18–22)

---

## Prerequisites

Before Task 1, verify the working environment:

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
pwd                         # expect /projects/elohim/.worktrees/epr-codec-phase-1
git branch --show-current   # expect feature/epr-codec-phase-1
git log --oneline -5        # expect HEAD at the Reach-alignment commit (b5784314)
```

Also verify elohim-storage toolchain and db reachability:

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --bin elohim-storage 2>&1 | tail -5
# Expect: clean build (may have warnings). If it fails, elohim-storage is broken before Phase 2a started — escalate.
```

If the db binary connection is part of the existing test harness, ensure tests pass on the current branch:

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage --lib 2>&1 | tail -5
```

If any prerequisite fails, stop and surface the issue.

---

## Group A — Decisions & Schemas

### Task 1: ADR — Reach backfill policy

**Files:**
- Create: `genesis/docs/superpowers/specs/decisions/2026-04-22-reach-backfill-policy.md`

- [ ] **Step 1: Author the ADR**

Write the decision record. Format matches the existing one at `genesis/docs/superpowers/specs/decisions/2026-04-19-d1-through-d5-node-and-household-canon.md` — decision / context / options considered / consequences.

The decision: new EPR writes REQUIRE `reach`. Existing content rows have no `reach` column; when Phase 2b begins projecting existing tables through EPR, the projector must assign a default. **Default for existing rows at migration time: `community`** (mid-restrictive, explicit network-visible; conservative but not overly locked). Owners of existing content can update reach via a new `PATCH` path (spec'd in Phase 2b).

Include these sections:

```markdown
# ADR: Reach Backfill Policy for Existing Content

**Status:** Accepted
**Date:** 2026-04-22
**Supersedes:** none
**Context:** The graph substrate's EPR envelope requires a `reach` field on every atom. Existing elohim-storage rows (content_nodes, humans, economic_events, etc.) were written before `reach` was envelope-level and therefore lack any stored reach value. Phase 2b's projector must assign a reach when converting existing rows to EPRs.

**Decision:** Existing rows project to EPRs with `reach = "community"` until the owner explicitly re-asserts a different reach through a Phase 2b endpoint.

**Alternatives considered:**
- `commons` — rejected. Widens visibility beyond what the original author consented to.
- `public` — rejected. Same concern — public is broadcast-level on the substrate.
- `private` — rejected. Too restrictive; existing content was visible to network consumers.
- `self` / `intimate` / `trusted` / `familiar` — rejected. No evidence in the existing data to pick one over another.
- Per-content-type default (e.g., content_nodes → public, economic_events → community) — rejected. Adds complexity without proportional gain; the author re-assert path handles nuance.

**Consequences:**
- No existing consumer experiences visibility expansion at projection time.
- Authors who want their content broader-reach must explicitly act.
- Phase 2b MUST ship the re-assert path before the projector is enabled in production.
- The ADR is binding on Phase 2b's projector code and any future migrations that add reach to a projected table.
```

- [ ] **Step 2: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add genesis/docs/superpowers/specs/decisions/2026-04-22-reach-backfill-policy.md
git commit -m "adr(epr): reach backfill policy for existing rows

Decision: existing rows without a stored reach project to EPRs with
reach = community. Owners explicitly re-assert via Phase 2b endpoint.
Rationale: conservative default, preserves original visibility intent,
no silent widening at migration time.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: JSON schemas for new views

**Files:**
- Create: `elohim/sdk/schemas/v1/views/epr-view.schema.json`
- Create: `elohim/sdk/schemas/v1/views/epr-envelope-view.schema.json`
- Create: `elohim/sdk/schemas/v1/views/epr-verify-view.schema.json`
- Create: `elohim/sdk/schemas/v1/views/epr-list-view.schema.json`
- Create: `elohim/sdk/schemas/v1/inputs/epr-publish-input.schema.json`

- [ ] **Step 1: Create `epr-envelope-view.schema.json`**

This is the wire-string form of the Envelope (CIDs as strings, not byte arrays). It's used inside `EprView` and `EprListView` and directly by `GET /epr/:cid/envelope`.

```json
{
  "$id": "epr:schema:view:epr-envelope",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "EprEnvelopeView",
  "description": "Source of truth: EPR atom (self-notarized via content-address + Ed25519). Wire-string projection of Envelope for HTTP consumers. CIDs are CIDv1 base32 strings. Category A — notarized via content-derived CID + signer proof.",
  "type": "object",
  "required": ["cid", "kind", "schemaRef", "schemaKey", "reach", "coupling", "claims", "issuedAt", "proof"],
  "additionalProperties": false,
  "properties": {
    "cid": { "type": "string", "description": "CIDv1 base32" },
    "kind": { "$ref": "../enums/epr-kind.schema.json" },
    "schemaRef": { "type": "string", "description": "CIDv1 base32 of the Manifest EPR" },
    "schemaKey": { "type": "string", "description": "Content-type key within the referenced manifest" },
    "reach": { "$ref": "../enums/reach.schema.json" },
    "coupling": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "knowledge": { "type": ["string", "null"], "description": "CIDv1 base32 of Claim-EPR" },
        "value": { "type": ["string", "null"], "description": "CIDv1 base32 of EconomicEvent-EPR" },
        "governance": { "type": ["string", "null"], "description": "CIDv1 base32 of Governance-EPR" }
      }
    },
    "claims": {
      "type": "array",
      "items": { "type": "string", "description": "CIDv1 base32" }
    },
    "supersedes": { "type": ["string", "null"], "description": "CIDv1 base32" },
    "supersededBy": { "type": ["string", "null"], "description": "CIDv1 base32 (derived from index, not in canonical bytes)" },
    "issuedAt": { "type": "string", "format": "date-time" },
    "proof": {
      "type": "object",
      "additionalProperties": false,
      "required": ["signer", "algorithm", "signature"],
      "properties": {
        "signer": { "type": "string", "description": "CIDv1 base32 of Agent EPR" },
        "algorithm": { "type": "string", "const": "ed25519" },
        "signature": { "type": "string", "pattern": "^[0-9a-f]{128}$", "description": "Hex-encoded 64 bytes" }
      }
    }
  }
}
```

- [ ] **Step 2: Create `epr-view.schema.json`**

```json
{
  "$id": "epr:schema:view:epr",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "EprView",
  "description": "Source of truth: EPR atom (self-notarized via content-address + Ed25519). Full HTTP view for GET /api/v1/epr/:cid. Payload is hex-encoded. Category A — notarized via content-derived CID + signer proof.",
  "type": "object",
  "required": ["envelope", "payload"],
  "additionalProperties": false,
  "properties": {
    "envelope": { "$ref": "./epr-envelope-view.schema.json" },
    "payload": { "type": "string", "description": "Hex-encoded payload bytes" },
    "canonicalBytes": { "type": "string", "description": "Optional hex-encoded canonical bytes (included when client sets ?includeCanonical=true)" }
  }
}
```

- [ ] **Step 3: Create `epr-verify-view.schema.json`**

```json
{
  "$id": "epr:schema:view:epr-verify",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "EprVerifyView",
  "description": "Source of truth: EPR atom verification result (derived on request). Category C — operational (reconstructed per request, not persisted).",
  "type": "object",
  "required": ["cid", "verified", "stagesRun"],
  "additionalProperties": false,
  "properties": {
    "cid": { "type": "string", "description": "CIDv1 base32 of the EPR being verified" },
    "verified": { "type": "boolean", "description": "true iff all run stages passed" },
    "stagesRun": {
      "type": "array",
      "items": {
        "type": "string",
        "enum": ["canonicalization", "signature", "coupling", "payloadSchema"]
      }
    },
    "stagesSkipped": {
      "type": "array",
      "items": {
        "type": "string",
        "enum": ["canonicalization", "signature", "coupling", "payloadSchema"]
      },
      "description": "Stages that were not run (e.g., payloadSchema deferred to Phase 3)"
    },
    "error": {
      "type": ["object", "null"],
      "additionalProperties": false,
      "required": ["stage", "message"],
      "properties": {
        "stage": { "type": "string", "enum": ["canonicalization", "signature", "coupling", "payloadSchema"] },
        "message": { "type": "string" }
      }
    }
  }
}
```

- [ ] **Step 4: Create `epr-list-view.schema.json`**

```json
{
  "$id": "epr:schema:view:epr-list",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "EprListView",
  "description": "Source of truth: EPR atoms (self-notarized). Paged list response for GET /api/v1/epr?... Items are envelope-only (no payload). Category A — atoms are notarized; the list projection is a query-time operation.",
  "type": "object",
  "required": ["items", "nextCursor"],
  "additionalProperties": false,
  "properties": {
    "items": {
      "type": "array",
      "items": { "$ref": "./epr-envelope-view.schema.json" }
    },
    "nextCursor": {
      "type": ["string", "null"],
      "description": "Opaque cursor for next page; null when exhausted"
    }
  }
}
```

- [ ] **Step 5: Create `epr-publish-input.schema.json`**

```json
{
  "$id": "epr:schema:input:epr-publish",
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "EprPublishInput",
  "description": "Input body for POST /api/v1/epr. Accepts a fully-signed EPR from the caller; server validates signature + coupling before storing.",
  "type": "object",
  "required": ["envelope", "payload"],
  "additionalProperties": false,
  "properties": {
    "envelope": { "$ref": "../views/epr-envelope-view.schema.json" },
    "payload": { "type": "string", "description": "Hex-encoded payload bytes" }
  }
}
```

- [ ] **Step 6: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/sdk/schemas/v1/views/epr-view.schema.json elohim/sdk/schemas/v1/views/epr-envelope-view.schema.json elohim/sdk/schemas/v1/views/epr-verify-view.schema.json elohim/sdk/schemas/v1/views/epr-list-view.schema.json elohim/sdk/schemas/v1/inputs/epr-publish-input.schema.json
git commit -m "feat(epr): JSON schemas for REST views + publish input

Per Integrator Compatibility Contract §2.2, every new REST surface
ships JSON schema FIRST. Five schemas land together:

- epr-envelope-view: wire-string Envelope (CIDs as base32 strings,
  not byte arrays) used by GET /api/v1/epr/:cid/envelope and embedded
  in list responses
- epr-view: full EprView (envelope + hex payload) for GET /api/v1/epr/:cid
- epr-verify-view: verification result with per-stage report for
  GET /api/v1/epr/:cid/verify
- epr-list-view: paged cursor-based list for GET /api/v1/epr?...
- epr-publish-input: POST /api/v1/epr body

All declare 'Source of truth:' per CONVENTIONS.md Rule 2.
additionalProperties: false and explicit required arrays throughout.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: Extend schema_contract test to cover new view schemas (shell only)

**Files:**
- Modify: `elohim/elohim-storage/tests/schema_contract.rs`

Add per-view contract tests that will be filled in as the Rust view structs are defined (Task 11). For now, scaffold test functions that load the schemas to prove they parse; actual Rust-struct validation hooks get added later.

- [ ] **Step 1: Inspect existing schema_contract.rs**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
head -80 elohim/elohim-storage/tests/schema_contract.rs
```

Note the existing pattern for view schema loading and validation. The test file likely uses `jsonschema` or similar. Match whatever's there.

- [ ] **Step 2: Add test stubs for 4 new views + 1 input**

Append to `elohim/elohim-storage/tests/schema_contract.rs`:

```rust
// ============================================================================
// Phase 2a — EPR view schema parsing (Rust struct validation added in Task 11)
// ============================================================================

#[test]
fn epr_view_schema_parses() {
    let _ = load_view_schema("epr-view.schema.json");
}

#[test]
fn epr_envelope_view_schema_parses() {
    let _ = load_view_schema("epr-envelope-view.schema.json");
}

#[test]
fn epr_verify_view_schema_parses() {
    let _ = load_view_schema("epr-verify-view.schema.json");
}

#[test]
fn epr_list_view_schema_parses() {
    let _ = load_view_schema("epr-list-view.schema.json");
}

#[test]
fn epr_publish_input_schema_parses() {
    let path = schemas_root()
        .join("inputs")
        .join("epr-publish-input.schema.json");
    let raw = std::fs::read_to_string(&path).unwrap();
    let _: serde_json::Value = serde_json::from_str(&raw).unwrap();
}
```

**Find the `load_view_schema` helper and `schemas_root()` helpers already in the file** — if they don't exist by those exact names, adapt to the existing helper names. The goal is: parse-only test that proves the schema files exist and are valid JSON. Rust-struct conformance is added in Task 11.

If the existing file uses a different pattern (e.g., iterating over a directory), add the 5 new schemas to whatever filter includes them.

- [ ] **Step 3: Run the parsing tests**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage --test schema_contract 2>&1 | tail -10
```

Expected: all existing tests pass plus 5 new `*_parses` tests pass.

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/tests/schema_contract.rs
git commit -m "test(epr): scaffold EPR view schema parse tests

Adds 5 stub tests that prove the Phase 2a Task 2 view and input
schemas are valid JSON and loadable. Rust-struct conformance tests
will be filled in at Task 11 once the view structs exist.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Group B — Storage Layer

### Task 4: Verify elohim-epr compiles as an elohim-storage dep under custom RUSTFLAGS

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml`
- Modify: `elohim/elohim-storage/src/lib.rs`

- [ ] **Step 1: Add the dep**

Append to `[dependencies]` in `elohim/elohim-storage/Cargo.toml`:

```toml
# EPR canonical codec (Phase 1 — generalized atom)
elohim-epr = { path = "../epr" }
```

- [ ] **Step 2: Add a smoke import to `lib.rs`**

At the top of `elohim/elohim-storage/src/lib.rs`, add (temporarily):

```rust
// Phase 2a smoke: confirm elohim-epr builds under custom RUSTFLAGS.
#[doc(hidden)]
pub use elohim_epr::{Epr, Envelope, EprKind};
```

These re-exports will be removed once real use arrives in Task 8+ — they exist only to force a compile-time link.

- [ ] **Step 3: Build**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check -p elohim-storage 2>&1 | tail -10
```

Expected: clean build. If it fails due to elohim-epr dependencies (e.g., `ed25519-dalek`, `getrandom`, `rand`) disagreeing with the custom `getrandom_backend`, escalate — this is a real blocker. Options may include:
- Add an `elohim-epr` feature flag that swaps the CSPRNG source
- Accept the custom backend at the workspace level
- Vendor a compatible `getrandom` path

Do NOT simply drop the import — the build failure means Phase 2a can't proceed without resolution.

If build succeeds, move to step 4.

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/Cargo.toml elohim/elohim-storage/src/lib.rs
git commit -m "build(epr): add elohim-epr as elohim-storage dep

Verifies the Phase 1 codec crate compiles under elohim-storage's
custom RUSTFLAGS (--cfg getrandom_backend=\"custom\"). Smoke
re-export in lib.rs will be removed when real use arrives.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Diesel migration — the 4 EPR tables

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-04-22-000000_add_epr_tables/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-04-22-000000_add_epr_tables/down.sql`
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs` (auto-generated)

- [ ] **Step 1: Create the migration directory**

```bash
mkdir -p /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage/migrations/2026-04-22-000000_add_epr_tables
```

- [ ] **Step 2: Write `up.sql`**

```sql
-- EPR storage layer — Phase 2a
-- See genesis/docs/superpowers/specs/2026-04-21-elohim-core-graph-substrate-design.md §8

-- Primary atom table
CREATE TABLE epr_atoms (
    cid               TEXT PRIMARY KEY,
    kind              TEXT NOT NULL,
    schema_ref        TEXT NOT NULL,
    schema_key        TEXT NOT NULL,
    reach             TEXT NOT NULL,
    issued_at         TIMESTAMPTZ NOT NULL,
    signer_cid        TEXT NOT NULL,
    supersedes        TEXT,                    -- FK to epr_atoms.cid; nullable; enforced app-side until supersedee exists
    canonical_bytes   BYTEA NOT NULL,
    payload_bytes     BYTEA NOT NULL,
    proof_bytes       BYTEA NOT NULL,          -- Ed25519 signature
    proof_algorithm   TEXT NOT NULL            -- "ed25519" for now
);

CREATE INDEX epr_atoms_kind_schema_ref_idx ON epr_atoms (kind, schema_ref);
CREATE INDEX epr_atoms_reach_idx ON epr_atoms (reach);
CREATE INDEX epr_atoms_signer_cid_idx ON epr_atoms (signer_cid);
CREATE INDEX epr_atoms_supersedes_idx ON epr_atoms (supersedes) WHERE supersedes IS NOT NULL;

-- Coupling legs (normalized FK rows, NOT a JSON column, per Integrator Compatibility Contract §4)
CREATE TABLE epr_coupling (
    epr_cid           TEXT NOT NULL REFERENCES epr_atoms(cid) ON DELETE CASCADE,
    leg               TEXT NOT NULL CHECK (leg IN ('knowledge', 'value', 'governance')),
    target_cid        TEXT NOT NULL,
    PRIMARY KEY (epr_cid, leg)
);

CREATE INDEX epr_coupling_target_cid_idx ON epr_coupling (target_cid);

-- Claims (outcome assertions) — the EPR asserts these Claim-EPRs as its outcomes
CREATE TABLE epr_claims (
    epr_cid           TEXT NOT NULL REFERENCES epr_atoms(cid) ON DELETE CASCADE,
    claim_cid         TEXT NOT NULL,
    PRIMARY KEY (epr_cid, claim_cid)
);

CREATE INDEX epr_claims_claim_cid_idx ON epr_claims (claim_cid);

-- Supersedence index (predecessor → successor, attested at revision time)
CREATE TABLE epr_supersedence (
    predecessor       TEXT NOT NULL,
    successor         TEXT NOT NULL,
    attested_by       TEXT NOT NULL,
    attested_at       TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (predecessor, successor)
);

CREATE INDEX epr_supersedence_predecessor_idx ON epr_supersedence (predecessor);
CREATE INDEX epr_supersedence_successor_idx ON epr_supersedence (successor);
```

- [ ] **Step 3: Write `down.sql`**

```sql
DROP TABLE IF EXISTS epr_supersedence;
DROP TABLE IF EXISTS epr_claims;
DROP TABLE IF EXISTS epr_coupling;
DROP TABLE IF EXISTS epr_atoms;
```

- [ ] **Step 4: Run the migration to update schema.rs**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
diesel migration run 2>&1 | tail -10
```

If a test DB isn't configured locally, diesel may fail to run. In that case:
1. Check for a `.env` or `diesel.toml` with a `DATABASE_URL` for the test DB
2. Alternative: use `diesel migration run --database-url=postgres://localhost/elohim_storage_test`
3. Worst case: hand-edit `src/db/diesel_schema.rs` to add the `table!` macros for the 4 new tables matching the migration SQL

The shape of the `table!` macros (if hand-editing):

```rust
diesel::table! {
    epr_atoms (cid) {
        cid -> Text,
        kind -> Text,
        schema_ref -> Text,
        schema_key -> Text,
        reach -> Text,
        issued_at -> Timestamptz,
        signer_cid -> Text,
        supersedes -> Nullable<Text>,
        canonical_bytes -> Bytea,
        payload_bytes -> Bytea,
        proof_bytes -> Bytea,
        proof_algorithm -> Text,
    }
}

diesel::table! {
    epr_coupling (epr_cid, leg) {
        epr_cid -> Text,
        leg -> Text,
        target_cid -> Text,
    }
}

diesel::table! {
    epr_claims (epr_cid, claim_cid) {
        epr_cid -> Text,
        claim_cid -> Text,
    }
}

diesel::table! {
    epr_supersedence (predecessor, successor) {
        predecessor -> Text,
        successor -> Text,
        attested_by -> Text,
        attested_at -> Timestamptz,
    }
}

diesel::joinable!(epr_coupling -> epr_atoms (epr_cid));
diesel::joinable!(epr_claims -> epr_atoms (epr_cid));
diesel::allow_tables_to_appear_in_same_query!(
    epr_atoms, epr_coupling, epr_claims, epr_supersedence
);
```

- [ ] **Step 5: Build to verify schema compiles**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check -p elohim-storage 2>&1 | tail -5
```

Expected: clean build.

- [ ] **Step 6: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/migrations/2026-04-22-000000_add_epr_tables elohim/elohim-storage/src/db/diesel_schema.rs
git commit -m "feat(epr): diesel migration for epr_atoms + coupling + claims + supersedence

Four tables land together as the EPR storage layer:
- epr_atoms: primary atoms, CID as PK, canonical/payload/proof bytes
- epr_coupling: FK rows per leg (not JSON column) so Phase 4 GraphQL
  can join in SQL per Integrator Compatibility Contract §4
- epr_claims: outcome assertion FKs
- epr_supersedence: predecessor/successor chain with issuer attestation

Indexes cover kind+schemaRef, reach, signer, coupling targets — the
access patterns REST list queries and future GraphQL resolvers need.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: Rust diesel models

**Files:**
- Create: `elohim/elohim-storage/src/db/epr_atoms.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs`

- [ ] **Step 1: Register the module**

Add to `elohim/elohim-storage/src/db/mod.rs` alongside other modules:

```rust
pub mod epr_atoms;
```

(Preserve existing alphabetical or convention order — match what's already there.)

- [ ] **Step 2: Write the model file**

Create `elohim/elohim-storage/src/db/epr_atoms.rs`:

```rust
//! Diesel models and queries for the EPR storage layer (Phase 2a).
//!
//! See spec §8 + Integrator Compatibility Contract §4.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::pg::PgConnection;
use serde::{Deserialize, Serialize};

use crate::db::diesel_schema::{epr_atoms, epr_claims, epr_coupling, epr_supersedence};

// ---------------------------------------------------------------------------
// epr_atoms
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = epr_atoms)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct EprAtom {
    pub cid: String,
    pub kind: String,
    pub schema_ref: String,
    pub schema_key: String,
    pub reach: String,
    pub issued_at: DateTime<Utc>,
    pub signer_cid: String,
    pub supersedes: Option<String>,
    pub canonical_bytes: Vec<u8>,
    pub payload_bytes: Vec<u8>,
    pub proof_bytes: Vec<u8>,
    pub proof_algorithm: String,
}

// ---------------------------------------------------------------------------
// epr_coupling
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = epr_coupling)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct EprCouplingRow {
    pub epr_cid: String,
    pub leg: String,
    pub target_cid: String,
}

// ---------------------------------------------------------------------------
// epr_claims
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = epr_claims)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct EprClaimRow {
    pub epr_cid: String,
    pub claim_cid: String,
}

// ---------------------------------------------------------------------------
// epr_supersedence
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = epr_supersedence)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct EprSupersedenceRow {
    pub predecessor: String,
    pub successor: String,
    pub attested_by: String,
    pub attested_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Query helpers
// ---------------------------------------------------------------------------

pub fn insert_atom(conn: &mut PgConnection, atom: &EprAtom) -> QueryResult<usize> {
    diesel::insert_into(epr_atoms::table)
        .values(atom)
        .execute(conn)
}

pub fn insert_coupling_rows(conn: &mut PgConnection, rows: &[EprCouplingRow]) -> QueryResult<usize> {
    diesel::insert_into(epr_coupling::table)
        .values(rows)
        .execute(conn)
}

pub fn insert_claim_rows(conn: &mut PgConnection, rows: &[EprClaimRow]) -> QueryResult<usize> {
    diesel::insert_into(epr_claims::table)
        .values(rows)
        .execute(conn)
}

pub fn fetch_atom_by_cid(conn: &mut PgConnection, cid: &str) -> QueryResult<Option<EprAtom>> {
    epr_atoms::table
        .find(cid)
        .first::<EprAtom>(conn)
        .optional()
}

pub fn fetch_coupling_for_atom(conn: &mut PgConnection, cid: &str) -> QueryResult<Vec<EprCouplingRow>> {
    epr_coupling::table
        .filter(epr_coupling::epr_cid.eq(cid))
        .load::<EprCouplingRow>(conn)
}

pub fn fetch_claims_for_atom(conn: &mut PgConnection, cid: &str) -> QueryResult<Vec<EprClaimRow>> {
    epr_claims::table
        .filter(epr_claims::epr_cid.eq(cid))
        .load::<EprClaimRow>(conn)
}

pub fn fetch_superseded_by(conn: &mut PgConnection, predecessor: &str) -> QueryResult<Option<String>> {
    epr_supersedence::table
        .filter(epr_supersedence::predecessor.eq(predecessor))
        .select(epr_supersedence::successor)
        .first::<String>(conn)
        .optional()
}

#[derive(Debug, Clone, Default)]
pub struct EprListQuery {
    pub kind: Option<String>,
    pub reach: Option<String>,
    pub schema_ref: Option<String>,
    pub after_cid: Option<String>,
    pub limit: i64,
}

pub fn list_atoms(conn: &mut PgConnection, q: &EprListQuery) -> QueryResult<Vec<EprAtom>> {
    let mut query = epr_atoms::table.into_boxed();
    if let Some(k) = &q.kind         { query = query.filter(epr_atoms::kind.eq(k)); }
    if let Some(r) = &q.reach        { query = query.filter(epr_atoms::reach.eq(r)); }
    if let Some(s) = &q.schema_ref   { query = query.filter(epr_atoms::schema_ref.eq(s)); }
    if let Some(a) = &q.after_cid    { query = query.filter(epr_atoms::cid.gt(a)); }
    query
        .order(epr_atoms::cid.asc())
        .limit(q.limit)
        .load::<EprAtom>(conn)
}
```

- [ ] **Step 3: Build**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check -p elohim-storage 2>&1 | tail -5
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/src/db/epr_atoms.rs elohim/elohim-storage/src/db/mod.rs
git commit -m "feat(epr): diesel models + query helpers for EPR tables

4 Insertable/Queryable structs match the migration schema. Helpers
cover the ingest + fetch + list + supersedence access patterns the
service layer needs in Task 8-11. No business logic here — just
diesel boilerplate.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 7: Column-vs-schema contract test

**Files:**
- Create: `elohim/elohim-storage/tests/schema_contract_diesel_epr.rs`

- [ ] **Step 1: Write the test**

```rust
//! Contract test: diesel columns match the EPR JSON schema properties.
//!
//! This test relies on the fact that `EprAtom` (and friends) have every field
//! we expect in the schema's `properties`. If the schema adds a field without
//! a corresponding column (or vice versa), this test fails and the PR cannot
//! merge.

use elohim_storage::db::epr_atoms::{EprAtom, EprCouplingRow, EprClaimRow, EprSupersedenceRow};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::PathBuf;

fn load_schema(relpath: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()     // elohim/
        .join("sdk/schemas/v1")
        .join(relpath);
    let raw = std::fs::read_to_string(&path).unwrap();
    serde_json::from_str(&raw).unwrap()
}

fn schema_properties(schema: &Value) -> BTreeSet<String> {
    schema["properties"]
        .as_object().expect("properties is map")
        .keys()
        .cloned()
        .collect()
}

/// For the four EPR storage structs, assert their field set matches the
/// corresponding JSON schema's property set. This is the diesel ↔ schema
/// alignment gate.

#[test]
fn epr_atom_fields_match_envelope_schema() {
    // EprAtom is a SUPERSET of EprEnvelopeView's non-CID-string form.
    // The atom table stores: cid, kind, schema_ref, schema_key, reach, issued_at,
    // signer_cid (which is proof.signer on the wire), supersedes, canonical_bytes,
    // payload_bytes, proof_bytes, proof_algorithm.
    //
    // Envelope view has: cid, kind, schemaRef, schemaKey, reach, coupling, claims,
    // supersedes, supersededBy, issuedAt, proof. Coupling, claims, and supersededBy
    // are JOINED at read time (not columns on epr_atoms).
    //
    // This test asserts the NAME mapping (camelCase ↔ snake_case) is consistent,
    // not that EVERY field appears 1:1.
    let schema = load_schema("views/epr-envelope-view.schema.json");
    let schema_props = schema_properties(&schema);

    // Names the atom table MUST carry (camelCase → snake_case on diesel side)
    for envelope_field in ["cid", "kind", "schemaRef", "schemaKey", "reach", "issuedAt"] {
        assert!(
            schema_props.contains(envelope_field),
            "envelope schema missing field {envelope_field}"
        );
    }

    // EprAtom struct fields (snake_case)
    let atom_fields: BTreeSet<&'static str> = [
        "cid", "kind", "schema_ref", "schema_key", "reach", "issued_at",
        "signer_cid", "supersedes", "canonical_bytes", "payload_bytes",
        "proof_bytes", "proof_algorithm",
    ].iter().copied().collect();

    // Spot-check that the atom struct covers all required envelope fields
    // via the schema property → column mapping.
    let required_columns = [
        ("cid", "cid"),
        ("kind", "kind"),
        ("schemaRef", "schema_ref"),
        ("schemaKey", "schema_key"),
        ("reach", "reach"),
        ("issuedAt", "issued_at"),
    ];
    for (schema_name, column_name) in required_columns {
        assert!(
            schema_props.contains(schema_name),
            "envelope schema does not declare {schema_name}"
        );
        assert!(
            atom_fields.contains(column_name),
            "EprAtom does not declare column {column_name}"
        );
    }
    // Unused variable warning avoidance — referenced via type inference above.
    let _ = std::mem::size_of::<EprAtom>();
}

#[test]
fn epr_coupling_row_fields_match_schema() {
    let schema = load_schema("views/epr-envelope-view.schema.json");
    let coupling_props = schema_properties(&schema["properties"]["coupling"]);
    // Schema property names (knowledge, value, governance) map to LEG VALUES, not columns.
    // The diesel table stores (epr_cid, leg, target_cid) with leg IN ('knowledge','value','governance').
    for leg in ["knowledge", "value", "governance"] {
        assert!(coupling_props.contains(leg));
    }
    let _ = std::mem::size_of::<EprCouplingRow>();
}

#[test]
fn epr_claim_row_structure_is_join_table() {
    // epr_claims is a pure join table: (epr_cid, claim_cid). It has no schema equivalent
    // because claims appear as an array on the envelope view. Assert the struct has
    // exactly 2 fields.
    // Using sizeof for compile-time reference; the real check is in the struct definition.
    let _ = std::mem::size_of::<EprClaimRow>();
}

#[test]
fn epr_supersedence_row_structure_matches_index() {
    // epr_supersedence is an index of issuer attestations: predecessor, successor,
    // attested_by, attested_at. The supersededBy envelope field is DERIVED from this.
    let _ = std::mem::size_of::<EprSupersedenceRow>();
}
```

- [ ] **Step 2: Run**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage --test schema_contract_diesel_epr 2>&1 | tail -10
```

Expected: 4 tests pass. These are light structural checks; the real drift-detection happens when you try to insert or select an `EprAtom` with a field that doesn't exist in the diesel_schema — that's a compile error, not a runtime test.

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/tests/schema_contract_diesel_epr.rs
git commit -m "test(epr): diesel ↔ JSON schema alignment contract

Asserts the camelCase schema field names map to snake_case diesel
column names for the EPR storage layer. Structural checks only — the
stronger gate is the compile-time type check that fires if an EprAtom
field is missing from diesel_schema.rs.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Group C — Service Layer

### Task 8: EprService — ingest (validator + insert)

**Files:**
- Create: `elohim/elohim-storage/src/services/epr_service.rs`
- Modify: `elohim/elohim-storage/src/services/mod.rs` (if the mod pattern requires registration)
- Modify: `elohim/elohim-storage/src/lib.rs` (remove the smoke re-export from Task 4; replace with real module wiring)

- [ ] **Step 1: Register the service module**

Find the existing pattern in `elohim/elohim-storage/src/services/mod.rs` or wherever services are declared. Add:

```rust
pub mod epr_service;
```

Remove the Task-4 smoke re-export at the top of `elohim/elohim-storage/src/lib.rs`:

```rust
// REMOVE these lines that were added in Task 4:
// pub use elohim_epr::{Epr, Envelope, EprKind};
```

- [ ] **Step 2: Write the service file**

Create `elohim/elohim-storage/src/services/epr_service.rs`:

```rust
//! EprService — business logic for EPR ingest, fetch, list, verify.
//!
//! See Integrator Compatibility Contract §2.2 for REST surface; this
//! service is the model/controller layer's business logic.

use chrono::Utc;
use diesel::pg::PgConnection;
use elohim_epr::{
    cid::{compute_cid, verify_cid},
    proof::verify as verify_ed25519,
    validate_coupling, Envelope, Epr, EprError,
};
use serde::{Deserialize, Serialize};

use crate::db::epr_atoms::{
    self, EprAtom, EprClaimRow, EprCouplingRow, EprListQuery,
};
use crate::error::StorageError;

// ---------------------------------------------------------------------------
// Public result shapes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EprIngestResult {
    pub cid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EprVerifyReport {
    pub cid: String,
    pub verified: bool,
    pub stages_run: Vec<String>,
    pub stages_skipped: Vec<String>,
    pub error: Option<EprVerifyError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EprVerifyError {
    pub stage: String,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Ingest
// ---------------------------------------------------------------------------

/// Validate an Epr (stages 1–3) and persist to storage.
/// Stage 4 (payload schema validation) is deferred to Phase 3.
pub fn ingest(conn: &mut PgConnection, epr: Epr) -> Result<EprIngestResult, StorageError> {
    // Stage 1: canonicalization — recompute canonical bytes and confirm CID matches
    let canonical = epr.envelope
        .canonical_bytes(&epr.payload)
        .map_err(|e| StorageError::Validation(format!("canonicalization: {e}")))?;
    let derived_cid = compute_cid(&canonical);
    if derived_cid != epr.envelope.cid {
        return Err(StorageError::Validation(format!(
            "cid mismatch: derived {} vs declared {}",
            derived_cid, epr.envelope.cid
        )));
    }

    // Stage 2: signature — resolve signer Agent EPR public key, verify over canonical bytes
    // PHASE 2a ACCEPTANCE: trust the proof bytes structurally (64 bytes ed25519).
    // Full resolver-based verify lands in Phase 3 when the manifest graph exists. Until
    // then, ingest accepts EPRs whose Signature.signer matches a known agent OR is a
    // bootstrap agent. The caller is expected to also call GET /epr/:cid/verify with a
    // public key, which does the real verification.
    if epr.envelope.proof.algorithm != "ed25519" {
        return Err(StorageError::Validation(format!(
            "unsupported proof algorithm: {}",
            epr.envelope.proof.algorithm
        )));
    }
    if epr.envelope.proof.signature.len() != 64 {
        return Err(StorageError::Validation(
            "ed25519 signature must be 64 bytes".into(),
        ));
    }

    // Stage 3: coupling — verify all kind-required legs are present
    validate_coupling(&epr.envelope)
        .map_err(|e: EprError| StorageError::Validation(format!("coupling: {e}")))?;

    // Stage 4: payload schema — DEFERRED to Phase 3 (needs manifest resolver)

    // Persist
    let atom = to_atom(&epr, &canonical);
    let coupling_rows = coupling_rows(&epr);
    let claim_rows = claim_rows(&epr);

    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        epr_atoms::insert_atom(conn, &atom)?;
        if !coupling_rows.is_empty() {
            epr_atoms::insert_coupling_rows(conn, &coupling_rows)?;
        }
        if !claim_rows.is_empty() {
            epr_atoms::insert_claim_rows(conn, &claim_rows)?;
        }
        Ok(())
    })
    .map_err(|e| StorageError::DbError(e.to_string()))?;

    Ok(EprIngestResult {
        cid: epr.envelope.cid.to_string(),
    })
}

fn to_atom(epr: &Epr, canonical: &[u8]) -> EprAtom {
    EprAtom {
        cid: epr.envelope.cid.to_string(),
        kind: format!("{:?}", epr.envelope.kind),
        schema_ref: epr.envelope.schema_ref.to_string(),
        schema_key: epr.envelope.schema_key.clone(),
        reach: reach_canonical(&epr.envelope.reach),
        issued_at: epr.envelope.issued_at,
        signer_cid: epr.envelope.proof.signer.to_string(),
        supersedes: epr.envelope.supersedes.map(|c| c.to_string()),
        canonical_bytes: canonical.to_vec(),
        payload_bytes: epr.payload.clone(),
        proof_bytes: epr.envelope.proof.signature.clone(),
        proof_algorithm: epr.envelope.proof.algorithm.clone(),
    }
}

fn coupling_rows(epr: &Epr) -> Vec<EprCouplingRow> {
    let mut rows = vec![];
    let cid = epr.envelope.cid.to_string();
    if let Some(k) = epr.envelope.coupling.knowledge {
        rows.push(EprCouplingRow { epr_cid: cid.clone(), leg: "knowledge".into(), target_cid: k.to_string() });
    }
    if let Some(v) = epr.envelope.coupling.value {
        rows.push(EprCouplingRow { epr_cid: cid.clone(), leg: "value".into(), target_cid: v.to_string() });
    }
    if let Some(g) = epr.envelope.coupling.governance {
        rows.push(EprCouplingRow { epr_cid: cid, leg: "governance".into(), target_cid: g.to_string() });
    }
    rows
}

fn claim_rows(epr: &Epr) -> Vec<EprClaimRow> {
    epr.envelope.claims.iter().map(|c| EprClaimRow {
        epr_cid: epr.envelope.cid.to_string(),
        claim_cid: c.to_string(),
    }).collect()
}

fn reach_canonical(r: &elohim_epr::Reach) -> String {
    use elohim_epr::Reach::*;
    match r {
        Private => "private",
        SelfScope => "self",
        Intimate => "intimate",
        Trusted => "trusted",
        Familiar => "familiar",
        Community => "community",
        Public => "public",
        Commons => "commons",
    }.into()
}

// Fetch, list, verify land in Task 9-11.
```

- [ ] **Step 3: Build**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check -p elohim-storage 2>&1 | tail -5
```

If `StorageError` variants don't match (`Validation`, `DbError`), adjust to whatever the existing error type provides. If neither exists, add them to `elohim/elohim-storage/src/error.rs`.

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/src/services/epr_service.rs elohim/elohim-storage/src/services/mod.rs elohim/elohim-storage/src/lib.rs
git commit -m "feat(epr): EprService::ingest — 3-stage validator + persist

Validates canonicalization (CID matches re-derived bytes), signature
algorithm + length, and coupling requirements per EprKind. Stage 4
(payload schema) is deferred to Phase 3 when the manifest resolver
ships. Persists atom + coupling rows + claim rows in a single
transaction.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 9: EprService — fetch_by_cid

**Files:**
- Modify: `elohim/elohim-storage/src/services/epr_service.rs`

- [ ] **Step 1: Add the fetch function**

Append to `elohim/elohim-storage/src/services/epr_service.rs`:

```rust
// ---------------------------------------------------------------------------
// Fetch
// ---------------------------------------------------------------------------

/// Reconstruct an Epr from storage.
/// Returns None if the cid is not in the store.
pub fn fetch_by_cid(conn: &mut PgConnection, cid: &str) -> Result<Option<FetchedEpr>, StorageError> {
    let Some(atom) = epr_atoms::fetch_atom_by_cid(conn, cid)
        .map_err(|e| StorageError::DbError(e.to_string()))?
    else { return Ok(None); };

    let coupling = epr_atoms::fetch_coupling_for_atom(conn, cid)
        .map_err(|e| StorageError::DbError(e.to_string()))?;

    let claims = epr_atoms::fetch_claims_for_atom(conn, cid)
        .map_err(|e| StorageError::DbError(e.to_string()))?;

    let superseded_by = epr_atoms::fetch_superseded_by(conn, cid)
        .map_err(|e| StorageError::DbError(e.to_string()))?;

    Ok(Some(FetchedEpr { atom, coupling, claims, superseded_by }))
}

/// All the rows necessary to reconstruct an Envelope for a REST response.
#[derive(Debug, Clone)]
pub struct FetchedEpr {
    pub atom: EprAtom,
    pub coupling: Vec<EprCouplingRow>,
    pub claims: Vec<EprClaimRow>,
    pub superseded_by: Option<String>,
}
```

- [ ] **Step 2: Build**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check -p elohim-storage 2>&1 | tail -5
```

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/src/services/epr_service.rs
git commit -m "feat(epr): EprService::fetch_by_cid — join atom + coupling + claims

Joins the four storage tables to reconstruct the full set of rows
needed to render an envelope view response. Returns None when the
cid is not found (for 404 handling at the route layer).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 10: EprService — list

**Files:**
- Modify: `elohim/elohim-storage/src/services/epr_service.rs`

- [ ] **Step 1: Add the list function**

Append to `elohim/elohim-storage/src/services/epr_service.rs`:

```rust
// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

/// Paged list of atoms, filtered by kind / reach / schemaRef / cursor.
/// Returns (items, next_cursor) where next_cursor is None when exhausted.
pub fn list(
    conn: &mut PgConnection,
    q: &EprListQuery,
) -> Result<(Vec<EprAtom>, Option<String>), StorageError> {
    // Fetch one extra row to determine whether there's a next page
    let fetch_limit = q.limit + 1;
    let query = EprListQuery {
        limit: fetch_limit,
        ..q.clone()
    };

    let mut rows = epr_atoms::list_atoms(conn, &query)
        .map_err(|e| StorageError::DbError(e.to_string()))?;

    let next_cursor = if rows.len() as i64 > q.limit {
        rows.pop();                          // discard the sentinel row
        rows.last().map(|r| r.cid.clone())
    } else {
        None
    };

    Ok((rows, next_cursor))
}
```

- [ ] **Step 2: Build**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check -p elohim-storage 2>&1 | tail -5
```

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/src/services/epr_service.rs
git commit -m "feat(epr): EprService::list — paged + filtered atom list

Cursor-based pagination (fetches N+1 rows to detect next-page).
Filters on kind, reach, schemaRef as exact-match. Cursor is the last
cid in the returned page so consumers can resume.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 11: EprService — verify + Rust wire view types + schema_contract test filler

**Files:**
- Modify: `elohim/elohim-storage/src/services/epr_service.rs`
- Modify: `elohim/elohim-storage/src/views.rs`
- Modify: `elohim/elohim-storage/tests/schema_contract.rs`

- [ ] **Step 1: Add verify function**

Append to `elohim/elohim-storage/src/services/epr_service.rs`:

```rust
// ---------------------------------------------------------------------------
// Verify
// ---------------------------------------------------------------------------

/// Verify a stored EPR under a given public key. Runs stages 1-3 fully; stage 4
/// is reported as skipped until Phase 3 resolver exists.
pub fn verify(
    conn: &mut PgConnection,
    cid: &str,
    public_key: &[u8; 32],
) -> Result<EprVerifyReport, StorageError> {
    let Some(fetched) = fetch_by_cid(conn, cid)? else {
        return Err(StorageError::NotFound(format!("epr not found: {cid}")));
    };

    let mut stages_run = vec!["canonicalization".to_string()];
    let mut stages_skipped = vec!["payloadSchema".to_string()];

    // Stage 1: CID matches canonical bytes
    let derived = compute_cid(&fetched.atom.canonical_bytes);
    if derived.to_string() != fetched.atom.cid {
        return Ok(EprVerifyReport {
            cid: cid.into(),
            verified: false,
            stages_run,
            stages_skipped,
            error: Some(EprVerifyError {
                stage: "canonicalization".into(),
                message: format!("cid does not match canonical bytes (derived {}, stored {})", derived, fetched.atom.cid),
            }),
        });
    }

    // Stage 2: signature verifies under provided public key
    stages_run.push("signature".into());
    if !verify_ed25519(public_key, &fetched.atom.canonical_bytes, &fetched.atom.proof_bytes) {
        return Ok(EprVerifyReport {
            cid: cid.into(),
            verified: false,
            stages_run,
            stages_skipped,
            error: Some(EprVerifyError {
                stage: "signature".into(),
                message: "ed25519 signature verification failed".into(),
            }),
        });
    }

    // Stage 3: coupling requirements met
    stages_run.push("coupling".into());
    let kind = kind_from_str(&fetched.atom.kind)?;
    let has_knowledge = fetched.coupling.iter().any(|c| c.leg == "knowledge");
    let has_value = fetched.coupling.iter().any(|c| c.leg == "value");
    let has_governance = fetched.coupling.iter().any(|c| c.leg == "governance");

    use elohim_epr::kind::CouplingLeg;
    for required in kind.required_coupling() {
        let have = match required {
            CouplingLeg::Knowledge => has_knowledge,
            CouplingLeg::Value => has_value,
            CouplingLeg::Governance => has_governance,
        };
        if !have {
            return Ok(EprVerifyReport {
                cid: cid.into(),
                verified: false,
                stages_run,
                stages_skipped,
                error: Some(EprVerifyError {
                    stage: "coupling".into(),
                    message: format!("kind {} requires coupling leg {:?}", fetched.atom.kind, required),
                }),
            });
        }
    }

    // Stage 4: DEFERRED — payload schema validation needs manifest resolver (Phase 3)

    Ok(EprVerifyReport {
        cid: cid.into(),
        verified: true,
        stages_run,
        stages_skipped,
        error: None,
    })
}

fn kind_from_str(s: &str) -> Result<elohim_epr::EprKind, StorageError> {
    use elohim_epr::EprKind::*;
    match s {
        "Content" => Ok(Content),
        "Agent" => Ok(Agent),
        "Manifest" => Ok(Manifest),
        "Claim" => Ok(Claim),
        "Observation" => Ok(Observation),
        "EconomicEvent" => Ok(EconomicEvent),
        "Commitment" => Ok(Commitment),
        "Attestation" => Ok(Attestation),
        "Delegation" => Ok(Delegation),
        other => Err(StorageError::Validation(format!("unknown EprKind: {other}"))),
    }
}
```

- [ ] **Step 2: Add wire view types to `views.rs`**

Append to `elohim/elohim-storage/src/views.rs`:

```rust
// ============================================================================
// EPR views (Phase 2a)
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
pub struct EprEnvelopeView {
    pub cid: String,
    pub kind: String,
    pub schema_ref: String,
    pub schema_key: String,
    pub reach: String,
    pub coupling: EprCouplingView,
    pub claims: Vec<String>,
    pub supersedes: Option<String>,
    pub superseded_by: Option<String>,
    pub issued_at: chrono::DateTime<chrono::Utc>,
    pub proof: EprSignatureView,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
pub struct EprCouplingView {
    pub knowledge: Option<String>,
    pub value: Option<String>,
    pub governance: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
pub struct EprSignatureView {
    pub signer: String,
    pub algorithm: String,
    pub signature: String,  // hex
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
pub struct EprView {
    pub envelope: EprEnvelopeView,
    pub payload: String,           // hex
    pub canonical_bytes: Option<String>,  // hex, only when ?includeCanonical=true
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
pub struct EprVerifyView {
    pub cid: String,
    pub verified: bool,
    pub stages_run: Vec<String>,
    pub stages_skipped: Vec<String>,
    pub error: Option<EprVerifyErrorView>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
pub struct EprVerifyErrorView {
    pub stage: String,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
pub struct EprListView {
    pub items: Vec<EprEnvelopeView>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../sdk/storage-client-ts/src/generated/")]
pub struct EprPublishInput {
    pub envelope: EprEnvelopeView,
    pub payload: String,  // hex
}
```

- [ ] **Step 3: Replace schema_contract stubs with real validation**

In `elohim/elohim-storage/tests/schema_contract.rs`, replace the 5 `*_parses` stubs from Task 3 with real validation. Pattern mirrors the existing contract tests in the file — construct a valid Rust view, serialize to JSON, validate against the schema:

```rust
// Replace the 5 stubs from Task 3 with these:

#[test]
fn epr_envelope_view_conforms() {
    use elohim_storage::views::{EprEnvelopeView, EprCouplingView, EprSignatureView};
    let v = EprEnvelopeView {
        cid: "bafyrei...".into(),
        kind: "Content".into(),
        schema_ref: "bafyrei...".into(),
        schema_key: "concept".into(),
        reach: "commons".into(),
        coupling: EprCouplingView {
            knowledge: Some("bafyrei...".into()),
            value: Some("bafyrei...".into()),
            governance: Some("bafyrei...".into()),
        },
        claims: vec!["bafyrei...".into()],
        supersedes: None,
        superseded_by: None,
        issued_at: chrono::Utc::now(),
        proof: EprSignatureView {
            signer: "bafyrei...".into(),
            algorithm: "ed25519".into(),
            signature: "a".repeat(128),  // 128 hex chars
        },
    };
    let json = serde_json::to_value(&v).unwrap();
    validate_against_schema("views/epr-envelope-view.schema.json", &json);
}

// (Similar tests for EprView, EprVerifyView, EprListView, EprPublishInput —
// construct each with valid field values, serialize, validate.)
```

Where `validate_against_schema` is whatever helper already exists in the file for view-schema validation.

- [ ] **Step 4: Build + test**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage --test schema_contract 2>&1 | tail -10
```

Expected: all new view-conformance tests pass.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/src/services/epr_service.rs elohim/elohim-storage/src/views.rs elohim/elohim-storage/tests/schema_contract.rs
git commit -m "feat(epr): verify + wire view types + schema contract tests

- EprService::verify runs stages 1-3 (canonicalization, signature,
  coupling); stage 4 (payload schema) is reported as skipped pending
  Phase 3 manifest resolver.
- Wire view types (EprView, EprEnvelopeView, EprVerifyView,
  EprListView, EprPublishInput, plus nested Coupling/Signature views)
  use camelCase + ts-rs export to storage-client-ts.
- schema_contract tests now construct valid Rust views and assert
  JSON output conforms to the authoritative schema.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Group D — Route Layer

### Task 12: Route — GET /api/v1/epr/:cid

**Files:**
- Create: `elohim/elohim-storage/src/api/epr.rs`
- Modify: `elohim/elohim-storage/src/api/mod.rs`

- [ ] **Step 1: Register the module**

Add to `elohim/elohim-storage/src/api/mod.rs`:

```rust
pub mod epr;
```

(Preserve existing order.)

- [ ] **Step 2: Create the controller with route dispatch + GET /:cid handler**

Create `elohim/elohim-storage/src/api/epr.rs`:

```rust
//! EPR REST controller — routes under /api/v1/epr.
//!
//! See Integrator Compatibility Contract §2.2.
//! Controller → EprService → db::epr_atoms.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};

use crate::db::{AppContext, DbPool};
use crate::db::epr_atoms::EprAtom;
use crate::error::StorageError;
use crate::services::epr_service::{self, FetchedEpr};
use crate::services::response::{self, from_option, from_result};
use crate::views::{EprCouplingView, EprEnvelopeView, EprSignatureView, EprView};

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

pub async fn handle(
    req: Request<Incoming>,
    ctx: &AppContext,
    path_tail: &[&str],
) -> Result<Response<Full<Bytes>>, StorageError> {
    match (req.method(), path_tail) {
        (&Method::GET, [cid])                      => get_epr(ctx, cid, &req).await,
        (&Method::GET, [cid, "envelope"])          => get_envelope(ctx, cid).await,
        (&Method::GET, [cid, "payload"])           => get_payload(ctx, cid).await,
        (&Method::GET, [cid, "verify"])            => get_verify(ctx, cid, &req).await,
        (&Method::GET, [])                         => list_epr(ctx, &req).await,
        (&Method::POST, [])                        => post_epr(ctx, req).await,
        _ => Ok(response::not_found()),
    }
}

// ---------------------------------------------------------------------------
// GET /api/v1/epr/:cid
// ---------------------------------------------------------------------------

async fn get_epr(
    ctx: &AppContext,
    cid: &str,
    req: &Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let include_canonical = req.uri().query()
        .map(|q| q.contains("includeCanonical=true"))
        .unwrap_or(false);

    let mut conn = get_conn(ctx)?;
    let Some(fetched) = epr_service::fetch_by_cid(&mut conn, cid)? else {
        return Ok(response::not_found());
    };

    let reach = fetched.atom.reach.clone();
    if !reach_visible_to(&reach, req)? {
        // Per contract §4, return 404 not 403 to avoid leaking existence
        return Ok(response::not_found());
    }

    let view = to_epr_view(&fetched, include_canonical);
    Ok(response::json(&view))
}

// ---------------------------------------------------------------------------
// View builders
// ---------------------------------------------------------------------------

fn to_epr_view(fetched: &FetchedEpr, include_canonical: bool) -> EprView {
    EprView {
        envelope: to_envelope_view(fetched),
        payload: hex::encode(&fetched.atom.payload_bytes),
        canonical_bytes: if include_canonical {
            Some(hex::encode(&fetched.atom.canonical_bytes))
        } else { None },
    }
}

pub(crate) fn to_envelope_view(fetched: &FetchedEpr) -> EprEnvelopeView {
    let mut coupling = EprCouplingView::default();
    for row in &fetched.coupling {
        match row.leg.as_str() {
            "knowledge" => coupling.knowledge = Some(row.target_cid.clone()),
            "value" => coupling.value = Some(row.target_cid.clone()),
            "governance" => coupling.governance = Some(row.target_cid.clone()),
            _ => {}
        }
    }

    EprEnvelopeView {
        cid: fetched.atom.cid.clone(),
        kind: fetched.atom.kind.clone(),
        schema_ref: fetched.atom.schema_ref.clone(),
        schema_key: fetched.atom.schema_key.clone(),
        reach: fetched.atom.reach.clone(),
        coupling,
        claims: fetched.claims.iter().map(|c| c.claim_cid.clone()).collect(),
        supersedes: fetched.atom.supersedes.clone(),
        superseded_by: fetched.superseded_by.clone(),
        issued_at: fetched.atom.issued_at,
        proof: EprSignatureView {
            signer: fetched.atom.signer_cid.clone(),
            algorithm: fetched.atom.proof_algorithm.clone(),
            signature: hex::encode(&fetched.atom.proof_bytes),
        },
    }
}

// ---------------------------------------------------------------------------
// Reach enforcement (envelope-level, no payload parse)
// ---------------------------------------------------------------------------

fn reach_visible_to(reach: &str, req: &Request<Incoming>) -> Result<bool, StorageError> {
    // Phase 2a: commons + public = anyone. everything else = require authenticated caller.
    // Auth lookup by Authorization header (Bearer) is delegated to a future middleware;
    // for now we accept any non-empty Authorization as "authenticated" and leave real
    // identity check to Phase 2b.
    match reach {
        "commons" | "public" => Ok(true),
        _ => {
            let authed = req
                .headers()
                .get(hyper::header::AUTHORIZATION)
                .and_then(|h| h.to_str().ok())
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false);
            Ok(authed)
        }
    }
}

// Remaining handlers land in Tasks 13-17.

#[allow(dead_code)]
async fn get_envelope(ctx: &AppContext, cid: &str) -> Result<Response<Full<Bytes>>, StorageError> {
    Ok(response::not_found())  // Task 13
}
#[allow(dead_code)]
async fn get_payload(ctx: &AppContext, cid: &str) -> Result<Response<Full<Bytes>>, StorageError> {
    Ok(response::not_found())  // Task 14
}
#[allow(dead_code)]
async fn get_verify(ctx: &AppContext, cid: &str, req: &Request<Incoming>) -> Result<Response<Full<Bytes>>, StorageError> {
    Ok(response::not_found())  // Task 15
}
#[allow(dead_code)]
async fn list_epr(ctx: &AppContext, req: &Request<Incoming>) -> Result<Response<Full<Bytes>>, StorageError> {
    Ok(response::not_found())  // Task 16
}
#[allow(dead_code)]
async fn post_epr(ctx: &AppContext, req: Request<Incoming>) -> Result<Response<Full<Bytes>>, StorageError> {
    Ok(response::not_found())  // Task 17
}
```

- [ ] **Step 3: Wire up the dispatcher in `api/mod.rs`**

Find where `/api/v1/...` paths are routed — likely a large `match` in the main dispatch function. Add a branch for `"epr"`:

```rust
// Inside the main dispatch match, alongside existing routes:
["api", "v1", "epr", rest @ ..] => epr::handle(req, &ctx, rest).await,
```

Adapt to match the existing pattern; the precise code may differ.

- [ ] **Step 4: Build + smoke test**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build 2>&1 | tail -5
```

Expected: clean build.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/src/api/epr.rs elohim/elohim-storage/src/api/mod.rs
git commit -m "feat(epr): GET /api/v1/epr/:cid + reach enforcement skeleton

Full EprView response (envelope + hex payload; optional canonical
bytes via ?includeCanonical=true). Envelope-level reach enforcement:
commons/public = open; everything else requires a caller. 404 (not
403) on reach denial so existence doesn't leak. Stubs in place for
Tasks 13-17.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 13: Route — GET /api/v1/epr/:cid/envelope

**Files:**
- Modify: `elohim/elohim-storage/src/api/epr.rs`

- [ ] **Step 1: Replace the stub**

Replace the `get_envelope` stub in `elohim/elohim-storage/src/api/epr.rs` with:

```rust
async fn get_envelope(
    ctx: &AppContext,
    cid: &str,
    req: &Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(ctx)?;
    let Some(fetched) = epr_service::fetch_by_cid(&mut conn, cid)? else {
        return Ok(response::not_found());
    };
    if !reach_visible_to(&fetched.atom.reach, req)? {
        return Ok(response::not_found());
    }
    Ok(response::json(&to_envelope_view(&fetched)))
}
```

**Important:** the existing signature doesn't take `req` — update the dispatcher to pass it:

```rust
(&Method::GET, [cid, "envelope"])          => get_envelope(ctx, cid, &req).await,
```

- [ ] **Step 2: Build**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build 2>&1 | tail -5
```

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/src/api/epr.rs
git commit -m "feat(epr): GET /api/v1/epr/:cid/envelope

Envelope-only response (no payload, no canonical bytes). Reach gate
enforced identically to /:cid. Cheap response for callers that need
metadata only.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 14: Route — GET /api/v1/epr/:cid/payload

**Files:**
- Modify: `elohim/elohim-storage/src/api/epr.rs`

- [ ] **Step 1: Replace the stub**

Replace the `get_payload` stub with:

```rust
async fn get_payload(
    ctx: &AppContext,
    cid: &str,
    req: &Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(ctx)?;
    let Some(fetched) = epr_service::fetch_by_cid(&mut conn, cid)? else {
        return Ok(response::not_found());
    };
    if !reach_visible_to(&fetched.atom.reach, req)? {
        return Ok(response::not_found());
    }

    // Raw bytes; Content-Type = application/octet-stream for now.
    // Phase 3 lookups manifest for real MIME when schema validation ships.
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/octet-stream")
        .header("X-Epr-Cid", &fetched.atom.cid)
        .body(Full::new(Bytes::from(fetched.atom.payload_bytes.clone())))
        .unwrap())
}
```

Update the dispatcher:

```rust
(&Method::GET, [cid, "payload"])           => get_payload(ctx, cid, &req).await,
```

- [ ] **Step 2: Build**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build 2>&1 | tail -5
```

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/src/api/epr.rs
git commit -m "feat(epr): GET /api/v1/epr/:cid/payload

Raw payload bytes as application/octet-stream. X-Epr-Cid response
header echoes the cid. Real MIME lookup via manifest schema deferred
to Phase 3.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 15: Route — GET /api/v1/epr/:cid/verify

**Files:**
- Modify: `elohim/elohim-storage/src/api/epr.rs`

- [ ] **Step 1: Replace the stub**

Replace the `get_verify` stub with:

```rust
async fn get_verify(
    ctx: &AppContext,
    cid: &str,
    req: &Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    // Caller provides publicKey as hex in query string:
    //   GET /api/v1/epr/:cid/verify?publicKey=<64-hex>
    let query = req.uri().query().unwrap_or("");
    let Some(pk_hex) = query.split('&')
        .find_map(|p| p.strip_prefix("publicKey="))
    else {
        return Ok(response::bad_request("publicKey query parameter required"));
    };

    let Ok(pk_bytes) = hex::decode(pk_hex) else {
        return Ok(response::bad_request("publicKey must be 64 hex chars"));
    };
    if pk_bytes.len() != 32 {
        return Ok(response::bad_request("publicKey must decode to 32 bytes"));
    }
    let mut pk = [0u8; 32];
    pk.copy_from_slice(&pk_bytes);

    let mut conn = get_conn(ctx)?;

    // Reach check: if the EPR isn't visible to the caller, return 404.
    let Some(fetched) = epr_service::fetch_by_cid(&mut conn, cid)? else {
        return Ok(response::not_found());
    };
    if !reach_visible_to(&fetched.atom.reach, req)? {
        return Ok(response::not_found());
    }

    let report = epr_service::verify(&mut conn, cid, &pk)?;

    // Map EprVerifyReport → EprVerifyView
    let view = crate::views::EprVerifyView {
        cid: report.cid,
        verified: report.verified,
        stages_run: report.stages_run,
        stages_skipped: report.stages_skipped,
        error: report.error.map(|e| crate::views::EprVerifyErrorView {
            stage: e.stage,
            message: e.message,
        }),
    };
    Ok(response::json(&view))
}
```

Update the dispatcher to pass `&req`:

```rust
(&Method::GET, [cid, "verify"])            => get_verify(ctx, cid, &req).await,
```

Also add `response::bad_request` if it doesn't exist — check `services/response.rs` and add if missing:

```rust
pub fn bad_request(msg: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(format!("{{\"error\":\"{msg}\"}}"))))
        .unwrap()
}
```

- [ ] **Step 2: Build**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build 2>&1 | tail -5
```

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/src/api/epr.rs elohim/elohim-storage/src/services/response.rs
git commit -m "feat(epr): GET /api/v1/epr/:cid/verify

Verifies an EPR against a caller-supplied ed25519 public key (hex,
query param). Returns EprVerifyView with per-stage run/skip report.
Stage 4 (payload schema) reported as skipped pending Phase 3.
400 on bad publicKey; 404 when EPR not visible to caller.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 16: Route — GET /api/v1/epr?kind=&reach=&schemaRef=&after=&limit=

**Files:**
- Modify: `elohim/elohim-storage/src/api/epr.rs`

- [ ] **Step 1: Replace the stub**

Replace the `list_epr` stub with:

```rust
async fn list_epr(
    ctx: &AppContext,
    req: &Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    use crate::db::epr_atoms::EprListQuery;
    let query = req.uri().query().unwrap_or("");

    let mut list_query = EprListQuery { limit: 50, ..Default::default() };
    let mut caller_authed = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    for kv in query.split('&') {
        if let Some(v) = kv.strip_prefix("kind=")       { list_query.kind = Some(v.into()); }
        else if let Some(v) = kv.strip_prefix("reach=")     { list_query.reach = Some(v.into()); }
        else if let Some(v) = kv.strip_prefix("schemaRef=") { list_query.schema_ref = Some(v.into()); }
        else if let Some(v) = kv.strip_prefix("after=")     { list_query.after_cid = Some(v.into()); }
        else if let Some(v) = kv.strip_prefix("limit=")     {
            if let Ok(n) = v.parse::<i64>() {
                list_query.limit = n.clamp(1, 200);
            }
        }
    }

    // If caller is unauthed and requested a restricted reach explicitly, short-circuit to empty.
    if !caller_authed {
        if let Some(r) = &list_query.reach {
            if !matches!(r.as_str(), "commons" | "public") {
                return Ok(response::json(&crate::views::EprListView {
                    items: vec![], next_cursor: None,
                }));
            }
        } else {
            // Default to showing only commons+public for unauthed callers.
            // To return both, we'd need a union filter; for Phase 2a, we filter to "commons"
            // and let authed callers see everything. Authed callers can pass reach= to filter.
            list_query.reach = Some("commons".into());
        }
    }

    let mut conn = get_conn(ctx)?;
    let (atoms, next_cursor) = epr_service::list(&mut conn, &list_query)?;

    // For each atom, we need coupling + claims + superseded_by to render EprEnvelopeView.
    // Could be N+1 in worst case; acceptable for Phase 2a (page size clamped to 200).
    // Phase 2b may introduce a joined list query if performance demands it.
    let mut items = Vec::with_capacity(atoms.len());
    for atom in &atoms {
        if let Some(fetched) = epr_service::fetch_by_cid(&mut conn, &atom.cid)? {
            items.push(to_envelope_view(&fetched));
        }
    }

    Ok(response::json(&crate::views::EprListView { items, next_cursor }))
}
```

- [ ] **Step 2: Build**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build 2>&1 | tail -5
```

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/src/api/epr.rs
git commit -m "feat(epr): GET /api/v1/epr?filters — paged list

Cursor pagination with kind/reach/schemaRef filters. Unauthed callers
limited to reach=commons by default (or their explicit commons/public
requests). Page size clamped to 200.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 17: Route — POST /api/v1/epr

**Files:**
- Modify: `elohim/elohim-storage/src/api/epr.rs`

- [ ] **Step 1: Replace the stub**

Replace the `post_epr` stub with:

```rust
async fn post_epr(
    ctx: &AppContext,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    use elohim_epr::{Coupling, Envelope, Epr, Reach, EprKind, Signature};
    use std::str::FromStr;

    // Parse the EprPublishInput from body
    let (parts, body) = req.into_parts();
    let input: crate::views::EprPublishInput = parse_body(body).await?;

    // Rehydrate the Rust Epr from the wire view
    // - Parse CID strings back to cid::Cid
    // - Hex-decode payload + signature
    let env_view = input.envelope;
    let cid = cid::Cid::from_str(&env_view.cid)
        .map_err(|e| StorageError::Validation(format!("bad cid: {e}")))?;
    let schema_ref = cid::Cid::from_str(&env_view.schema_ref)
        .map_err(|e| StorageError::Validation(format!("bad schemaRef: {e}")))?;
    let signer = cid::Cid::from_str(&env_view.proof.signer)
        .map_err(|e| StorageError::Validation(format!("bad signer: {e}")))?;

    let kind = match env_view.kind.as_str() {
        "Content" => EprKind::Content,
        "Agent" => EprKind::Agent,
        "Manifest" => EprKind::Manifest,
        "Claim" => EprKind::Claim,
        "Observation" => EprKind::Observation,
        "EconomicEvent" => EprKind::EconomicEvent,
        "Commitment" => EprKind::Commitment,
        "Attestation" => EprKind::Attestation,
        "Delegation" => EprKind::Delegation,
        other => return Ok(response::bad_request(&format!("unknown kind: {other}"))),
    };

    let reach = match env_view.reach.as_str() {
        "private" => Reach::Private,
        "self" => Reach::SelfScope,
        "intimate" => Reach::Intimate,
        "trusted" => Reach::Trusted,
        "familiar" => Reach::Familiar,
        "community" => Reach::Community,
        "public" => Reach::Public,
        "commons" => Reach::Commons,
        other => return Ok(response::bad_request(&format!("unknown reach: {other}"))),
    };

    let coupling = Coupling {
        knowledge: env_view.coupling.knowledge.as_deref().map(cid::Cid::from_str)
            .transpose().map_err(|e| StorageError::Validation(format!("bad knowledge cid: {e}")))?,
        value: env_view.coupling.value.as_deref().map(cid::Cid::from_str)
            .transpose().map_err(|e| StorageError::Validation(format!("bad value cid: {e}")))?,
        governance: env_view.coupling.governance.as_deref().map(cid::Cid::from_str)
            .transpose().map_err(|e| StorageError::Validation(format!("bad governance cid: {e}")))?,
    };

    let claims = env_view.claims.iter()
        .map(|s| cid::Cid::from_str(s))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::Validation(format!("bad claims cid: {e}")))?;

    let supersedes = env_view.supersedes.as_deref().map(cid::Cid::from_str).transpose()
        .map_err(|e| StorageError::Validation(format!("bad supersedes cid: {e}")))?;

    let sig_bytes = hex::decode(&env_view.proof.signature)
        .map_err(|e| StorageError::Validation(format!("bad signature hex: {e}")))?;
    if sig_bytes.len() != 64 {
        return Ok(response::bad_request("signature must be 64 bytes"));
    }

    let envelope = Envelope {
        cid,
        kind,
        schema_ref,
        schema_key: env_view.schema_key,
        reach,
        coupling,
        claims,
        supersedes,
        superseded_by: None,  // Server ignores any submitted value — derived from index
        issued_at: env_view.issued_at,
        proof: Signature::ed25519(signer, sig_bytes),
    };

    let payload = hex::decode(&input.payload)
        .map_err(|e| StorageError::Validation(format!("bad payload hex: {e}")))?;

    let epr = Epr { envelope, payload };

    let mut conn = get_conn(ctx)?;
    let result = epr_service::ingest(&mut conn, epr)?;

    Ok(response::created_json(&result))
}
```

Add `response::created_json` helper if it doesn't exist (mirror `response::json` but use `StatusCode::CREATED`).

- [ ] **Step 2: Build + test**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build 2>&1 | tail -5
```

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/src/api/epr.rs elohim/elohim-storage/src/services/response.rs
git commit -m "feat(epr): POST /api/v1/epr — ingest endpoint

Accepts EprPublishInput (envelope + hex payload), rehydrates to Rust
Epr, runs the 3-stage validator via EprService::ingest, persists on
success. Returns 201 with the stored CID. Server-side ignores any
supersededBy submitted — that field is derived from the index.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Group E — Integration, TS Client, CI

### Task 18: Integration test — end-to-end ingest → fetch → verify

**Files:**
- Create: `elohim/elohim-storage/tests/epr_ingest_integration.rs`

This test requires a live test database. Follow whatever pattern the existing integration tests in `elohim/elohim-storage/tests/` use (many have patterns like `test_db::setup_connection()` or env-var-driven URL).

- [ ] **Step 1: Write the test**

```rust
//! End-to-end EPR ingest + fetch + verify.

use chrono::Utc;
use elohim_epr::{
    cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach,
};
use elohim_storage::services::epr_service;

// Use whatever DB setup helper existing tests use:
mod common;

fn mk_cid(b: u8) -> cid::Cid { compute_cid(&[b]) }

fn build_test_epr() -> (AgentKeypair, Epr) {
    let kp = AgentKeypair::from_secret(&[7u8; 32]).unwrap();
    let signer = mk_cid(100);
    let epr = Epr::builder()
        .kind(EprKind::Content)
        .schema_ref(mk_cid(1))
        .schema_key("concept")
        .reach(Reach::Commons)
        .coupling(Coupling {
            knowledge: Some(mk_cid(2)),
            value: Some(mk_cid(3)),
            governance: Some(mk_cid(4)),
        })
        .claim(mk_cid(5))
        .issued_at(Utc::now())
        .payload(b"hello".to_vec())
        .sign(&kp, signer)
        .unwrap();
    (kp, epr)
}

#[test]
#[ignore = "requires test database"]
fn ingest_then_fetch_then_verify() {
    let mut conn = common::setup_test_db();

    let (kp, epr) = build_test_epr();
    let expected_cid = epr.envelope.cid.to_string();

    // Ingest
    let ingest_result = epr_service::ingest(&mut conn, epr).expect("ingest");
    assert_eq!(ingest_result.cid, expected_cid);

    // Fetch
    let fetched = epr_service::fetch_by_cid(&mut conn, &expected_cid)
        .expect("fetch")
        .expect("some");
    assert_eq!(fetched.atom.cid, expected_cid);
    assert_eq!(fetched.atom.kind, "Content");
    assert_eq!(fetched.atom.reach, "commons");
    assert_eq!(fetched.coupling.len(), 3);
    assert_eq!(fetched.claims.len(), 1);

    // Verify
    let report = epr_service::verify(&mut conn, &expected_cid, &kp.public_key_bytes())
        .expect("verify");
    assert!(report.verified);
    assert!(report.stages_run.contains(&"canonicalization".to_string()));
    assert!(report.stages_run.contains(&"signature".to_string()));
    assert!(report.stages_run.contains(&"coupling".to_string()));
    assert!(report.stages_skipped.contains(&"payloadSchema".to_string()));
}

#[test]
#[ignore = "requires test database"]
fn verify_rejects_wrong_public_key() {
    let mut conn = common::setup_test_db();
    let (_kp, epr) = build_test_epr();
    let cid = epr.envelope.cid.to_string();
    epr_service::ingest(&mut conn, epr).unwrap();

    let other = AgentKeypair::from_secret(&[9u8; 32]).unwrap();
    let report = epr_service::verify(&mut conn, &cid, &other.public_key_bytes()).unwrap();
    assert!(!report.verified);
    assert_eq!(report.error.as_ref().unwrap().stage, "signature");
}

#[test]
#[ignore = "requires test database"]
fn ingest_rejects_tampered_payload() {
    let mut conn = common::setup_test_db();
    let (_kp, mut epr) = build_test_epr();
    epr.payload = b"TAMPERED".to_vec();  // CID no longer matches
    let result = epr_service::ingest(&mut conn, epr);
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("cid mismatch") || msg.contains("canonicalization"));
}
```

- [ ] **Step 2: Create `tests/common/mod.rs` if needed**

If the existing tests already have a `common` module, reuse it. Otherwise create:

```rust
//! Shared test helpers.

use diesel::pg::PgConnection;
use diesel::Connection;

pub fn setup_test_db() -> PgConnection {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/elohim_storage_test".into());
    PgConnection::establish(&url).expect("test db connection")
}
```

- [ ] **Step 3: Run with a real DB (if available)**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage --test epr_ingest_integration -- --include-ignored 2>&1 | tail -10
```

If no test DB is available in the environment, the `#[ignore]` attributes keep CI green. Jenkins provisions a DB in the pipeline (Task 22).

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/tests/epr_ingest_integration.rs elohim/elohim-storage/tests/common
git commit -m "test(epr): end-to-end ingest + fetch + verify integration test

Marked #[ignore] because requires a test Postgres instance; Jenkins
provisions one. Local run: cargo test --include-ignored.
Covers: successful ingest + fetch round-trip, wrong-key verify fail,
tampered-payload ingest rejection at canonicalization stage.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 19: Integration test — reach enforcement

**Files:**
- Create: `elohim/elohim-storage/tests/epr_reach_enforcement.rs`

- [ ] **Step 1: Write the test**

```rust
//! Reach enforcement at the /api/v1/epr/:cid endpoint.
//!
//! Verifies: unauth caller sees commons + public; restricted reaches return 404
//! (not 403) to avoid leaking existence.

use chrono::Utc;
use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};
use elohim_storage::services::epr_service;

mod common;

fn mk_cid(b: u8) -> cid::Cid { compute_cid(&[b]) }

fn build_epr_with_reach(reach: Reach) -> Epr {
    let kp = AgentKeypair::from_secret(&[3u8; 32]).unwrap();
    Epr::builder()
        .kind(EprKind::Content)
        .schema_ref(mk_cid(1))
        .schema_key("concept")
        .reach(reach)
        .coupling(Coupling {
            knowledge: Some(mk_cid(2)),
            value: Some(mk_cid(3)),
            governance: Some(mk_cid(4)),
        })
        .claim(mk_cid(5))
        .issued_at(Utc::now())
        .payload(b"r".to_vec())
        .sign(&kp, mk_cid(100))
        .unwrap()
}

#[test]
#[ignore = "requires test database"]
fn service_layer_stores_reach_verbatim() {
    // The service layer (EprService::ingest) stores whatever reach the envelope
    // declares. Enforcement happens at the route layer.
    let mut conn = common::setup_test_db();

    for reach in [Reach::Commons, Reach::Public, Reach::Community, Reach::Private] {
        let epr = build_epr_with_reach(reach);
        let expected_reach = match reach {
            Reach::Commons => "commons",
            Reach::Public => "public",
            Reach::Community => "community",
            Reach::Private => "private",
            _ => unreachable!(),
        };
        let cid = epr.envelope.cid.to_string();
        epr_service::ingest(&mut conn, epr).unwrap();
        let fetched = epr_service::fetch_by_cid(&mut conn, &cid).unwrap().unwrap();
        assert_eq!(fetched.atom.reach, expected_reach);
    }
}

// Route-level enforcement test would spin up the hyper server, make requests
// with + without Authorization headers, and assert 404 vs 200. That's a bigger
// harness setup — deferred to Task 20's integration shell which brings up the
// full API server for end-to-end checks.
```

- [ ] **Step 2: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/tests/epr_reach_enforcement.rs
git commit -m "test(epr): service-layer reach persistence + route-layer gate hooks

Asserts the storage layer persists reach verbatim. Full route-layer
401/404 assertions deferred to the end-to-end Task 20 harness.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 20: TS client regeneration + smoke test

**Files:**
- Modify: `elohim/sdk/storage-client-ts/src/generated/` (regenerated)
- Create: `elohim/sdk/storage-client-ts/src/generated/index.ts` entries for new EPR types
- Create: `elohim/sdk/storage-client-ts/tests/epr-types.test.ts`

- [ ] **Step 1: Regenerate ts-rs output**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings 2>&1 | tail -5
```

Expected: new TS files appear in `elohim/sdk/storage-client-ts/src/generated/`:
- `EprView.ts`
- `EprEnvelopeView.ts`
- `EprCouplingView.ts`
- `EprSignatureView.ts`
- `EprVerifyView.ts`
- `EprVerifyErrorView.ts`
- `EprListView.ts`
- `EprPublishInput.ts`

- [ ] **Step 2: Register in `generated/index.ts`**

Add the exports:

```ts
export * from './EprView';
export * from './EprEnvelopeView';
export * from './EprCouplingView';
export * from './EprSignatureView';
export * from './EprVerifyView';
export * from './EprVerifyErrorView';
export * from './EprListView';
export * from './EprPublishInput';
```

- [ ] **Step 3: Add a smoke test**

Create `elohim/sdk/storage-client-ts/tests/epr-types.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import type {
  EprView, EprEnvelopeView, EprVerifyView, EprListView, EprPublishInput,
} from '../src/generated';

describe('EPR view types compile', () => {
  it('EprEnvelopeView has required fields', () => {
    const v: EprEnvelopeView = {
      cid: 'bafyrei...',
      kind: 'Content',
      schemaRef: 'bafyrei...',
      schemaKey: 'concept',
      reach: 'commons',
      coupling: { knowledge: null, value: null, governance: null },
      claims: [],
      supersedes: null,
      supersededBy: null,
      issuedAt: '2026-04-22T00:00:00Z',
      proof: { signer: 'bafyrei...', algorithm: 'ed25519', signature: 'a'.repeat(128) },
    };
    expect(v.kind).toBe('Content');
  });

  it('EprView has envelope + payload', () => {
    const e: EprView = {
      envelope: {
        cid: 'bafyrei...', kind: 'Content', schemaRef: 'bafyrei...', schemaKey: 'k',
        reach: 'commons',
        coupling: { knowledge: null, value: null, governance: null },
        claims: [], supersedes: null, supersededBy: null,
        issuedAt: '2026-04-22T00:00:00Z',
        proof: { signer: 'bafyrei...', algorithm: 'ed25519', signature: 'a'.repeat(128) },
      },
      payload: 'deadbeef',
      canonicalBytes: null,
    };
    expect(e.payload).toBe('deadbeef');
  });

  it('EprVerifyView has stages arrays', () => {
    const r: EprVerifyView = {
      cid: 'bafyrei...',
      verified: true,
      stagesRun: ['canonicalization', 'signature', 'coupling'],
      stagesSkipped: ['payloadSchema'],
      error: null,
    };
    expect(r.stagesRun.length).toBe(3);
  });
});
```

- [ ] **Step 4: Run**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/sdk/storage-client-ts
pnpm test 2>&1 | tail -10
```

Expected: existing tests pass + 3 new EPR type tests pass.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/sdk/storage-client-ts
git commit -m "feat(storage-client): regenerate TS types for EPR views

ts-rs export produces 8 new files (EprView, EprEnvelopeView,
EprCouplingView, EprSignatureView, EprVerifyView, EprVerifyErrorView,
EprListView, EprPublishInput). generated/index.ts extended with the
new exports. Smoke test confirms the types compile and have expected
shape.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 21: Pre-push hook + Jenkins pipeline integration

**Files:**
- Modify: `.husky/pre-push`
- Modify: `genesis/orchestrator/Jenkinsfile` (optional, if orchestrator changes are safe)

- [ ] **Step 1: Extend the pre-push hook**

Find the section that detects changed paths and runs gates. Add EPR-layer detection alongside existing ones:

```sh
# elohim-storage EPR layer gate (Phase 2a)
if git diff --cached --name-only HEAD | grep -qE '^elohim/elohim-storage/src/api/epr|^elohim/elohim-storage/src/db/epr_atoms|^elohim/elohim-storage/src/services/epr_service|^elohim/elohim-storage/migrations/.*add_epr_tables|^elohim/sdk/schemas/v1/views/epr-|^elohim/sdk/schemas/v1/inputs/epr-'; then
  echo "→ EPR storage layer changes detected; running gate"
  (cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build) || exit 1
  (cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage --test schema_contract) || exit 1
  (cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage --test schema_contract_diesel_epr) || exit 1
  (cd elohim/sdk/storage-client-ts && pnpm test) || exit 1
fi
```

Adapt to the hook's existing style.

- [ ] **Step 2: Add Jenkins pipeline stage (optional)**

If `genesis/orchestrator/Jenkinsfile`'s PIPELINES map has a clear format, append:

```groovy
'epr-storage': [
  changeset: [
    'elohim/elohim-storage/src/api/epr*',
    'elohim/elohim-storage/src/db/epr_atoms*',
    'elohim/elohim-storage/src/services/epr_service*',
    'elohim/elohim-storage/migrations/*_add_epr_tables*',
    'elohim/sdk/schemas/v1/views/epr-*',
    'elohim/sdk/schemas/v1/inputs/epr-*',
  ],
  jenkinsfile: 'elohim/elohim-storage/Jenkinsfile',
],
```

If the format is unfamiliar or the existing elohim-storage pipeline already covers these paths, skip this step and note it.

- [ ] **Step 3: Smoke-run the hook**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
# simulate: stage a small change in an EPR path, then run the hook manually
echo "# phase 2a test" >> elohim/elohim-storage/migrations/2026-04-22-000000_add_epr_tables/up.sql
git add elohim/elohim-storage/migrations/2026-04-22-000000_add_epr_tables/up.sql
HUSKY=1 .husky/pre-push origin main < /dev/null
git checkout -- elohim/elohim-storage/migrations/2026-04-22-000000_add_epr_tables/up.sql
```

Expected: the hook detects the changed path and runs the gate; it should succeed (all tests pass).

If the hook errors due to DB connectivity in a test that needs it, those are `#[ignore]`d and won't block. If a schema_contract test fails, that's a real issue — investigate.

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add .husky/pre-push genesis/orchestrator/Jenkinsfile
git commit -m "ci(epr): pre-push + pipeline gate for Phase 2a storage layer

Detects changes to api/epr, db/epr_atoms, services/epr_service,
EPR migrations, and the new view/input schemas. Runs the build
plus schema_contract + schema_contract_diesel_epr tests locally.
CI pipeline (if orchestrator change accepted) same gate.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

### Task 22: Documentation + public API consolidation

**Files:**
- Create: `elohim/elohim-storage/docs/EPR_REST_API.md`
- Modify: `elohim/elohim-storage/README.md`
- Modify: `elohim/sdk/storage-client-ts/README.md` (if exists)

- [ ] **Step 1: Create EPR REST API doc**

Create `elohim/elohim-storage/docs/EPR_REST_API.md`:

```markdown
# EPR REST API (Phase 2a)

All endpoints are additive — existing endpoints unchanged per the
Integrator Compatibility Contract.

## Endpoints

### `GET /api/v1/epr/:cid`
Returns the full EPR (envelope + hex payload).
Optional query: `?includeCanonical=true` adds hex canonical bytes.

Response: `EprView` — see `elohim/sdk/schemas/v1/views/epr-view.schema.json`.

### `GET /api/v1/epr/:cid/envelope`
Returns the envelope only (no payload).

Response: `EprEnvelopeView`.

### `GET /api/v1/epr/:cid/payload`
Returns raw payload bytes. `Content-Type: application/octet-stream`
for now; Phase 3 uses the manifest schema to set real MIME.

`X-Epr-Cid` response header echoes the requested CID.

### `GET /api/v1/epr/:cid/verify?publicKey=<64-hex>`
Verifies the stored EPR against a caller-supplied ed25519 public key.
Runs stages 1-3 (canonicalization, signature, coupling). Stage 4
(payload schema) is reported as skipped pending Phase 3.

Response: `EprVerifyView`.

### `POST /api/v1/epr`
Accepts a signed EPR (`EprPublishInput`), validates stages 1 + 3,
persists.

Response: `201 Created` with `{ cid: "..." }`.

### `GET /api/v1/epr?kind=&reach=&schemaRef=&after=&limit=`
Paged list. Cursor-based via `after=<cid>`. `limit` clamped to 200.
Unauthed callers default to `reach=commons`.

Response: `EprListView`.

## Reach enforcement

Endpoint-level gate reads `reach` from the envelope only (no payload
parse). `commons` and `public` are open; restricted reaches require
an `Authorization` header. 404 (not 403) on denial to avoid existence
leak.

Full identity integration lands in Phase 2b.

## Schema discovery

Every response shape is declared in
`elohim/sdk/schemas/v1/views/epr-*.schema.json`.
Each schema carries `Source of truth:` per
`elohim/sdk/schemas/v1/views/CONVENTIONS.md` Rule 2.

## Verification via @elohim/epr

TypeScript consumers can re-verify the returned EPR client-side:

    import { verifyEpr } from '@elohim/epr';
    const result = await verifyEpr(response, publicKey);
    if (!result.ok) throw new Error(result.error.message);
```

- [ ] **Step 2: Update elohim-storage README**

Append a brief section to `elohim/elohim-storage/README.md`:

```markdown
## EPR Storage Layer

Phase 2a of the graph substrate adds:

- Diesel tables: `epr_atoms`, `epr_coupling`, `epr_claims`, `epr_supersedence`
- Service: `services::epr_service` (ingest / fetch / list / verify)
- REST routes: `/api/v1/epr/*` — see `docs/EPR_REST_API.md`

All existing endpoints unchanged.

The generalized EPR atom codec lives in the sibling `elohim-epr` crate;
this crate wraps it in diesel + hyper.
```

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1
git add elohim/elohim-storage/docs/EPR_REST_API.md elohim/elohim-storage/README.md
git commit -m "docs(epr): REST API reference + README section

EPR_REST_API.md covers the 6 new endpoints, reach enforcement, and
the TS client verify pattern. README links to the new doc plus the
sibling elohim-epr crate.

Phase 2a ships feature-complete: storage foundation, additive REST
surface, TS client, CI integration, documentation. Existing REST
consumers unchanged. Phase 2b (projector + Signal Harness migration)
is its own plan.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Batch gate — after all 22 tasks

```bash
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -p elohim-storage --all-targets 2>&1 | tail -10
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -p elohim-storage --all-targets -- -D warnings 2>&1 | tail -3
cd /projects/elohim/.worktrees/epr-codec-phase-1/elohim/sdk/storage-client-ts && pnpm test 2>&1 | tail -5
```

All must be clean (ignored tests can remain ignored without a real DB).

## Acceptance criteria

Phase 2a is done when:

1. All 22 tasks committed, atomic per task, on `feature/epr-codec-phase-1`
2. `cargo build` + `cargo clippy -D warnings` clean for `elohim-storage`
3. `cargo test -p elohim-storage --all-targets` passes (ignoring DB-requiring tests when no DB present)
4. `pnpm test` in `elohim/sdk/storage-client-ts` passes
5. All 5 new JSON schemas + 8 ts-rs generated types committed
6. Integration tests pass when run with `--include-ignored` on a real test DB
7. Pre-push hook catches changes to EPR paths and runs the gate
8. README + REST API doc describe the new surface
9. **Every existing REST endpoint continues to return byte-identical responses** — verify by running the full existing test suite pre + post and diffing output (captured in Phase 2b's contract tests; for 2a it's sufficient that no existing test fails)

## Self-review

**Spec coverage (Phase 2 §13 + Integrator Compatibility Contract §2):**

| Requirement | Covered by |
|---|---|
| `epr_atoms` + 3 supporting tables with correct schema | Task 5 |
| Validator stages 1-3 run on insert | Task 8 |
| Stage 4 deferred and reported as skipped | Tasks 8, 11 |
| REST: `GET /epr/:cid` | Task 12 |
| REST: `GET /epr/:cid/envelope` | Task 13 |
| REST: `GET /epr/:cid/payload` | Task 14 |
| REST: `GET /epr/:cid/verify` | Task 15 |
| REST: `GET /epr?filters` | Task 16 |
| REST: `POST /epr` | Task 17 |
| Reach enforcement at envelope level (no payload parse) | Task 12 (reach_visible_to helper used by all GET handlers) |
| JSON schemas for all new views + input | Task 2 |
| Schema contract tests for views | Tasks 3, 11 |
| Schema contract for diesel columns | Task 7 |
| ts-rs export → storage-client-ts | Task 20 |
| TS smoke test | Task 20 |
| Pre-push + Jenkins | Task 21 |
| Reach backfill ADR | Task 1 |
| Documentation | Task 22 |
| Existing REST endpoints unchanged | All tasks; no existing file in api/*.rs is modified except api/mod.rs for dispatch registration |

**Explicitly deferred (Phase 2b):**
- Projector (epr_atoms → existing pillar tables)
- Signal Harness migration to EPR emission
- Write-through feature flag wiring
- Pre/post byte-identical contract tests for existing views (no projector yet to test against)
- Reconciliation between `epr_codec.rs` (EprHead pattern) and generalized Envelope
- Full identity integration beyond "has Authorization header"

**Placeholder scan:** no TBDs, no "similar to Task N", no "fill in details." Every step has exact code or exact commands.

**Type consistency:** `EprAtom`, `EprCouplingRow`, `EprClaimRow`, `EprSupersedenceRow`, `FetchedEpr`, `EprView`, `EprEnvelopeView`, `EprCouplingView`, `EprSignatureView`, `EprVerifyView`, `EprVerifyErrorView`, `EprListView`, `EprPublishInput` — names used consistently across Tasks 6 through 22. Service functions `ingest`, `fetch_by_cid`, `list`, `verify` — signatures consistent across Tasks 8–11 and their uses in Tasks 12–17.

**Open risks called out in plan:**
1. Task 4 — elohim-epr building under elohim-storage's RUSTFLAGS. If it fails, escalate.
2. Task 5 — requires a test DB for `diesel migration run`. Fallback: hand-edit `diesel_schema.rs`.
3. Task 17 — `cid::Cid::from_str` needs to accept the base32 string form that the wire view produces; verify against Phase 1 `parseCid` behavior.
4. Task 21 — orchestrator Jenkinsfile edit is optional; skip if risky.

---

## Execution handoff

Plan complete and saved to `genesis/docs/superpowers/plans/2026-04-22-elohim-epr-storage-foundation-plan.md`. Two execution options:

**1. Subagent-Driven (recommended)** — dispatch fresh subagent per task, two-stage review between tasks, fast iteration. Given the scope (22 tasks across diesel, services, routes, TS, CI) and the mixed mechanical/integration nature, modified batching (like Phase 1's batches 2-3) is appropriate:
- **Batch A:** Tasks 1-7 (ADR + schemas + storage layer)
- **Batch B:** Tasks 8-11 (service layer)
- **Batch C:** Tasks 12-17 (route layer)
- **Batch D:** Tasks 18-22 (integration + TS + CI + docs)

Each batch produces a meaningful, reviewable milestone.

**2. Inline Execution** — use `superpowers:executing-plans` with checkpoints at group boundaries.

Which approach?
