---
id: "backlog-deprecation-storage-bulk-x-schema-version-header"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-storage bulk writes tolerate a missing X-Schema-Version header (deprecated client contract) — every in-repo client is now conformant; the terminal server-tolerance decision is an open operator wire-contract call"
slug: "deprecation-storage-bulk-x-schema-version-header"
written: "2026-08-07"
updated: "2026-08-21"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: blocked
severity: low
fingerprints: ["3d8af5658223", "f47f0600b001", "cdb5ca58ee6c", "d7af212f42f2", "fef5d7190486", "ea88a05cc69f", "388edb6e4ce8", "b4f2961e0593", "61d2d6c7bc10", "4b6912d97c00", "f7398a2d0c88", "0ab7ff6b4673", "757534fab7b6", "2942cdb50eba", "a997ee70af19", "27ddce001d3a"]
relatedNodeIds: []
tags: [deprecation, elohim-storage, schema-negotiation, http-contract, bulk-endpoints]
cites:
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-views/src/shared.rs
  - elohim/elohim-storage/src/services/response.rs
  - crates/elohim-storage-client/src/client.rs
  - crates/elohim-sdk/src/client/content_client.rs
  - crates/elohim-sdk/src/client/mod.rs
  - genesis/a2o/src/framework/dataplane/surfaces.ts
  - app/elohim-app/src/app/elohim/services/storage-client.service.ts
  - app/elohim-library/projects/elohim-service/src/client/elohim-client.ts
  - genesis/seeder/src/doorway-client.ts
  - genesis/data/timeline/backlog/deprecation-sentinel-redundant-capture-surfaces.md
---

## What is deprecated

Not a third-party deprecation — a **first-party HTTP contract** elohim-storage
declares deprecated on its own guarded bulk-write routes. `elohim/elohim-storage/src/http.rs`:

```rust
None => {
    warn!("Bulk request missing X-Schema-Version header (deprecated: clients should send this header)");
    Ok(None)
}
```

`validate_schema_version_header` is advisory-on-absence and strict-on-presence:
an unsupported value is a 400 (`Unsupported schema version: N. Supported: [1]`),
an absent header falls back to `default_schema_version()` and emits the warning.
`SUPPORTED_SCHEMA_VERSIONS = &[1]` (`elohim/elohim-views/src/shared.rs:73`), and the
server advertises the negotiation surface back via the `X-Supported-Schema-Versions`
response header (`elohim/elohim-storage/src/services/response.rs:46`).

Six routes are guarded — all of them bulk writes:

| Route | Handler |
|---|---|
| `POST /db/content/bulk` | `handle_db_content_bulk` |
| `POST /db/relationships/bulk` | `handle_db_relationships_bulk` |
| `POST /db/presences/bulk` | `handle_presences_bulk` |
| `POST /db/events/bulk` | `handle_events_bulk` |
| `POST /db/mastery/bulk` | `handle_mastery_bulk` |
| `POST /db/allocations/bulk` | `handle_allocations_bulk` |

The app-scoped form `/db/{app_id}/<resource>/bulk` strips to the same
`resource_path` and reaches the same handlers, so an app-scoped client is
equally in scope.

Sibling bulk routes are NOT guarded and are out of scope:
`/api/v1/economic-events/bulk`, `/api/v1/steward-affinity/bulk`,
`/api/v1/stewardship/allocations/bulk`.

## Usage inventory

Scoped 2026-08-07 across every in-repo caller of the six guarded routes.

**Already conformant before this pass** — `genesis/seeder/src/doorway-client.ts`
(content / relationships / presences / events / mastery bulk, five sites) and
`genesis/seeder/src/seed-nodes.ts:88`. This is why the warning has never been
observed from the primary seed path.

**Non-conformant, fixed in this pass:**

| File | Route |
|---|---|
| `app/elohim-app/src/app/elohim/services/storage-client.service.ts` | content + relationships bulk |
| `app/elohim-library/projects/elohim-service/src/client/elohim-client.ts` | content bulk (both the direct-storage and failover branches share one headers object) |
| `genesis/seeder/src/seed-sqlite.ts` | content bulk |
| `genesis/seeder/src/seed-stewardship.ts` | allocations bulk |
| `genesis/a2o/scripts/publish-results.ts` | content bulk |
| `crates/elohim-storage-client/src/client.rs` | `bulk_create_content` — fixed as a reqwest **client default header** so future guarded routes on this client are conformant by construction |

**Fixed in the 2026-08-21 pass** — the two remaining holes, one of which was
the actual live emitter:

| File | Route | Note |
|---|---|---|
| `crates/elohim-sdk/src/client/content_client.rs` | `/db/{app_id}/{content_type}/bulk` in `flush_to_projection` + `flush_to_storage` | Fixed as a **client default header** via the new `schema_conformant_http_client()` in `crates/elohim-sdk/src/client/mod.rs`, not per call site — the pattern `crates/elohim-storage-client` already used. `elohim-views` was already a direct SDK dependency, so the value reads from `default_schema_version()` and cannot drift. `ProjectionWarmer`'s two constructors take the same client. |
| `genesis/a2o/src/framework/dataplane/surfaces.ts` | `postRaw()` → `/db/content/bulk` from `steps/mesh/sync-control.steps.ts:709` | **This was the live emitter.** Fixed on the shared `postRaw` helper, so every bodied a2o dataplane POST is conformant by construction. `postRawInvalidJson` in `sync-control.steps.ts` was updated in step to preserve its documented "mirrors postRaw's shape exactly, minus the stringify" invariant. |

No in-repo caller of the six guarded routes is non-conformant as of this pass
(`grep -rn '/bulk'` sweep over `crates/ elohim/ doorway/ steward/` for Rust and
`app/ genesis/` for TS — every remaining hit is a server-side route definition
or a doc comment).

## Migration path

No upstream guide — the contract is ours. Two steps:

1. **Every client sends `X-Schema-Version: 1`.** Value read from the shared
   contract where the language allows it (`elohim_views::shared::default_schema_version()`
   in Rust) rather than a literal, so a future `SUPPORTED_SCHEMA_VERSIONS` bump
   cannot silently desync a client. **This step is COMPLETE as of 2026-08-21.**
2. **Decide the terminal state for the server tolerance** — either promote the
   absent-header case from `warn` + default to a 400, or delete the deprecation
   language and accept omission as permanently supported. This is a wire-contract
   decision affecting any external/third-party client of a doorway, not just the
   monorepo. **This is the only remaining work, and it is not a background-agent
   call.**

## Current decision

**BLOCKED on an operator wire-contract decision. Step 1 is complete and
runtime-verified; do not re-dispatch triage for this warning.**

Every in-repo client of the six guarded bulk routes now sends the header. The
warning can now only be emitted by an out-of-repo client, or by a stale binary
or stale checkout predating this pass — if it reappears from an in-repo path,
that is a genuine regression and the ledger will correctly re-fire.

The one remaining action is step 2, and it needs a human call because it changes
a public wire contract. Recommendation carried forward from the 2026-08-07 pass,
unchanged and now stronger: **keep the tolerance, drop the "deprecated"
language**, and let `X-Supported-Schema-Versions` carry the negotiation. A hard
400 buys nothing while `SUPPORTED_SCHEMA_VERSIONS == [1]` — there is no second
version to disambiguate — and it converts a silent-and-correct default into an
outage for any external client. Revisit if and when version 2 ships. Executing
that decision deletes the `warn!` line, which also ends this concern's
contribution to the sentinel self-capture load described below, and closes
this entry.

**A correction this pass had to make.** The 2026-08-07 entry claimed step 1's
banner-gone proof "structurally" — every in-repo caller was believed to take the
present-header branch. That claim was wrong: `genesis/a2o`'s `postRaw` helper was
never in the 2026-08-07 inventory, and it emitted the warning against the live
local mesh twice on 2026-08-21 (`matthew.log` 20:22:04 and 20:26:13). A
structural argument over an inventory is only as good as the inventory. This
pass replaces it with a runtime A/B (below) rather than a second structural
claim.

**Note on the ledger fingerprints.** Sixteen fingerprints canonicalize here, and
only two are distinct runtime observations (the `matthew.log` pair above,
captured as `61d2d6c7bc10` / `4b6912d97c00`). The other fourteen are grep- and
diff-output self-captures of the same source line, re-hashed each time `http.rs`
grew and the `warn!` moved (line 260 → 267 → 277 → 317 → 430), each time a
different `grep` prefix was used, or — during this very triage run — each time
the agent's own scoping commands and doc-comment diffs printed the warning text
back to a hooked shell. That multiplication is Class 3 of
`deprecation-sentinel-redundant-capture-surfaces.md`, where this warning is
already the worked example. It stays owned there; this entry owns the underlying
HTTP contract only. Fingerprint `3d8af5658223` previously cited a backlog file
(`deprecation-elohim-storage-schema-version-header-warn.md`) that was never
committed — this entry replaces that dangling citation.

## Verification

**Runtime A/B against the live local mesh (2026-08-21), matthew storage at
`http://localhost:8090`** — the banner-gone proof the previous pass could not
produce. Warning occurrences counted in `/tmp/elohim-local-mesh/logs/matthew.log`:

| Probe | Request | HTTP | Warning delta |
|---|---|---|---|
| A — old `postRaw` shape | `POST /db/content/bulk` with `content-type` only | 200 | **+1** (reproduces the ledger warning on demand) |
| B — new `postRaw` shape | same POST plus `x-schema-version: 1` | 200 | **0** (banner gone) |

Probe A reproduces the exact captured warning and probe B extinguishes it, with
both requests accepted — so the header is both necessary and sufficient, and the
fix cannot be a false negative from the request simply failing earlier.

**Gates run 2026-08-21 on the trees changed in this pass. All green.**

| Tree | Command | Result |
|---|---|---|
| `crates/elohim-sdk` | `RUSTFLAGS="" cargo build --all-features` | `Finished dev profile`, exit 0 |
| `crates/elohim-sdk` | `RUSTFLAGS="" cargo test --all-features` | `8 passed; 0 failed` (lib) · `1 passed; 0 failed` (integration) · doc-tests exit 0 |
| `crates/elohim-sdk` | `cargo clippy --all-features --all-targets -- -D warnings` | `Finished dev profile`, exit 0 |
| `crates/elohim-sdk` | `cargo fmt --check` | clean, exit 0 |
| `genesis/a2o` | `pnpm exec eslint` (both changed files) | exit 0, no findings |
| `genesis/a2o` | `pnpm exec prettier --check` (both changed files) | "All matched files use Prettier code style!" |
| `genesis/a2o` | `pnpm run typecheck` | 1 error, **pre-existing and not from this pass** — see below |

The new lib test is
`client::tests::schema_conformant_client_sends_schema_version_header_on_bulk_post`,
which binds a loopback listener, issues a bulk POST through
`schema_conformant_http_client()`, and asserts the header appears in the raw
request bytes — an on-the-wire assertion rather than a construction-only one, so
the SDK side is regression-guarded independently of a running mesh.

The single `pnpm run typecheck` error is
`steps/delivery/acquisition-pins.steps.ts(504,13): error TS2339: Property
'status' does not exist on type 'ResponseData<null>'`. It belongs to another
session's concurrent in-flight work on that file (`git diff HEAD` shows 433
insertions there, and line 504 falls inside the `@@ -266,6 +493,26 @@` added
hunk). It is untouched by and unrelated to this pass, and was deliberately not
fixed here to avoid colliding with that session. This pass's two a2o files carry
zero typecheck, lint, or format findings.
