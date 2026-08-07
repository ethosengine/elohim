---
id: "backlog-deprecation-storage-bulk-x-schema-version-header"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-storage bulk writes tolerate a missing X-Schema-Version header (deprecated client contract) — in-repo clients now conformant, server tolerance and the SDK projection flush remain"
slug: "deprecation-storage-bulk-x-schema-version-header"
written: "2026-08-07"
author: "deprecation-triage"
status: "wip"
priority: "low"
deprecation_status: in-progress
severity: low
fingerprints: ["3d8af5658223", "f47f0600b001", "cdb5ca58ee6c", "d7af212f42f2", "fef5d7190486", "ea88a05cc69f", "388edb6e4ce8", "b4f2961e0593"]
relatedNodeIds: []
tags: [deprecation, elohim-storage, schema-negotiation, http-contract, bulk-endpoints]
cites:
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-views/src/shared.rs
  - elohim/elohim-storage/src/services/response.rs
  - crates/elohim-storage-client/src/client.rs
  - crates/elohim-sdk/src/client/content_client.rs
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

**Still non-conformant (the remaining trajectory):**

| File | Route | Why not fixed here |
|---|---|---|
| `crates/elohim-sdk/src/client/content_client.rs:395,428` | `/db/{app_id}/{content_type}/bulk` in `flush_to_projection` + `flush_to_storage` | The file was under concurrent edit by another agent during this run and was declared out of scope for the dispatch. Both call sites build a bare `self.http_client.post(&url).json(&items)` with no schema header. |

## Migration path

No upstream guide — the contract is ours. Two steps:

1. **Every client sends `X-Schema-Version: 1`.** Value read from the shared
   contract where the language allows it (`elohim_views::shared::default_schema_version()`
   in Rust) rather than a literal, so a future `SUPPORTED_SCHEMA_VERSIONS` bump
   cannot silently desync a client. This step is what makes the runtime warning
   stop firing.
2. **Decide the terminal state for the server tolerance** — either promote the
   absent-header case from `warn` + default to a 400, or delete the deprecation
   language and accept omission as permanently supported. This is a wire-contract
   decision affecting any external/third-party client of a doorway, not just the
   monorepo, and it is the reason this entry stays open after step 1.

## Current decision

**Step 1 landed for every in-repo client except the SDK projection flush; step 2
is undecided and needs an operator call.**

The six non-conformant in-repo call sites now send the header, verified green
(see Verification). The runtime warning can now only be emitted by
`crates/elohim-sdk`'s `content_client` flush or by an out-of-repo client.

Next actions, in order:

1. Add `X-Schema-Version` to `crates/elohim-sdk/src/client/content_client.rs`
   `flush_to_projection` and `flush_to_storage` — small and mechanical, blocked
   on the concurrent edit clearing. Prefer wiring it as a default header on the
   SDK's shared `http_client` (the pattern used in `crates/elohim-storage-client`)
   rather than per call site.
2. Then decide step 2. Recommendation: **keep tolerance, drop the "deprecated"
   language**, and let `X-Supported-Schema-Versions` carry the negotiation. A
   hard 400 buys nothing while `SUPPORTED_SCHEMA_VERSIONS == [1]` — there is no
   second version to disambiguate — and it turns a silent-and-correct default
   into an outage for any external client. Revisit if and when version 2 ships.
   Executing that decision deletes the `warn!` line, which also removes this
   concern's contribution to the sentinel self-capture load described below.

**Note on the ledger fingerprints.** All eight canonicalized fingerprints are
captures of the SAME source line, re-hashed each time `http.rs` grew and the
`warn!` moved (line 260 → 267 → 277 → 317) or each time a different `grep`
prefix was used. None is a distinct runtime observation — every one is a
grep-output self-capture. That multiplication is Class 3 of
`deprecation-sentinel-redundant-capture-surfaces.md`, where this warning is
already the worked example (`f47f0600b001` ← five rows at three statuses). It
stays owned there; this entry owns the underlying HTTP contract only. Fingerprint
`3d8af5658223` previously cited a backlog file
(`deprecation-elohim-storage-schema-version-header-warn.md`) that was never
committed — this entry replaces that dangling citation.

## Verification

Gates run 2026-08-07 against the client changes. All green.

| Tree | Command | Result |
|---|---|---|
| `crates/elohim-storage-client` | `RUSTFLAGS="" cargo build` | `Finished dev profile`, exit 0 |
| `crates/elohim-storage-client` | `RUSTFLAGS="" cargo test` | `3 passed; 0 failed` (doc-tests), exit 0 |
| `crates/elohim-storage-client` | `cargo clippy --all-targets -- -D warnings` | `Finished dev profile`, no errors |
| `crates/elohim-storage-client` | `cargo fmt --check` | clean |
| `genesis/seeder` | `pnpm run typecheck` | exit 0 |
| `genesis/seeder` | `pnpm test` | `31 passed \| 1 skipped (32)` files, `444 passed \| 9 skipped (453)` tests |
| `genesis/a2o` | `pnpm run typecheck` | exit 0 |
| `genesis/a2o` | `pnpm run lint` | `34 problems (0 errors, 34 warnings)` |
| `app/elohim-library/projects/elohim-service` | `pnpm test` | `32 passed (32)` files, `798 passed (798)` tests |
| `app/elohim-app` | `pnpm exec vitest run --config vite.config.ts src/app/elohim/services/storage-client.service.spec.ts` | `30 passed (30)` |
| `app/elohim-library` | `pnpm exec eslint projects/elohim-service/src/client/elohim-client.ts` | `0 errors, 4 warnings` (pre-existing) |

`pnpm exec eslint` on `storage-client.service.ts` reports two `import/order`
errors at lines 24–25. These are pre-existing debt on the branch: `git diff -U0`
on that file shows hunks at `+34`, `+275`, `+291` only — the import block is
untouched.

The banner-gone proof for step 1 is structural rather than observational: the
warning is emitted only on the absent-header branch, and every in-repo caller of
the six guarded handlers now takes the present-header branch (inventory above,
`grep -rn "/bulk"` sweep). It is not claimed as fully closed precisely because
`crates/elohim-sdk`'s flush still takes the deprecated branch.
