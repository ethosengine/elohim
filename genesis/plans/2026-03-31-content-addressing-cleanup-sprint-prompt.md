# Content Addressing Cleanup Sprint — Session Prompt

> **For Claude:** This is a standalone session prompt. Start by reading the design context, then invoke `superpowers:brainstorming` to explore approaches before writing the implementation plan.

## Context

During the resilient HTML5 app delivery work (Sprint 1-3, 2026-03-30), we discovered several naming/addressing inconsistencies that need cleanup. These are not bugs blocking delivery — they're architectural hygiene that prevents confusion as more apps and content types are built on the protocol.

## Issues to Address

### 1. hAppId vs appId Disambiguation

**Problem:** `appId` means two different things depending on context:
- **Holochain app context** (`"lamad"`) — the installed Holochain app ID, stored at `data.appId` in projections
- **HTML5 app identifier** (`"evolution-of-trust"`) — the content's app slug, stored inside `data.contentBody` JSON

This caused a production bug where doorway's projection cache couldn't find the HTML5 app because it was reading the Holochain app context instead of the content body's app ID.

**Proposed fix:** Rename the Holochain app context field to `hAppId` everywhere it appears:
- `elohim-storage` views, models, and HTTP handlers
- `doorway-service` projection collections
- `storage-client-ts` generated types
- Seed data and seeder pipeline
- MongoDB projected_entries documents

**Files to audit:**
- `elohim/elohim-storage/src/views.rs` (API boundary)
- `elohim/elohim-storage/src/db/models.rs` (DB models)
- `elohim/elohim-storage/src/http.rs` (route handlers, app_index, build_manifest)
- `doorway/doorway-service/src/projection/collections/` (projection schemas)
- `doorway/doorway-service/src/cache/app_file_cache.rs` (app index resolution)
- `genesis/seeder/src/seed.ts` (seeder pipeline)
- `app/elohim-library/projects/elohim-service/` (TypeScript consumers)

### 2. Slug-to-CID URL Migration

**Problem:** App URLs use convenience slugs (`/apps/evolution-of-trust/index.html`) instead of content addresses (`/apps/{CID}/index.html`). The slug is a human-readable alias; the CID is the truth.

**Current state:**
- `blob_hash` (sha256-{hex}) IS the content address for the ZIP blob
- `appId` slug maps to `blob_hash` via `app_index` HashMap
- The slug is needed for URL readability but shouldn't be the primary key

**Proposed approach:**
- Primary URL: `/apps/{blob_hash}/index.html` (content-addressed, cacheable forever)
- Alias URL: `/apps/{slug}/index.html` (resolves to CID, 302 redirect or transparent proxy)
- EPR URI: `epr:evolution-of-trust` resolves to the current CID at runtime
- Service Worker cache keys already use blob_hash — the SW is ready for this

**Key constraint:** The iframe renderer in Angular currently constructs `/apps/{appId}/{entryPoint}` from the content body. Changing the URL pattern requires updating the renderer too.

**Files to audit:**
- `app/elohim-app/src/app/lamad/components/iframe-renderer/` (URL construction)
- `elohim/elohim-storage/src/http.rs` (handle_app_request path parsing)
- `doorway/doorway-service/src/routes/apps.rs` (doorway path parsing)
- `app/elohim-app/src/apps-sw.ts` (SW fetch intercept pattern)

### 3. Projection Field Consistency

**Problem:** The projection store (`projected_entries` in MongoDB) has field naming inconsistencies:
- `data.appId` — Holochain app context (should be `data.hAppId`)
- `data.contentBody` — sometimes a JSON string, sometimes a parsed object
- `data.blobHash` vs `data.blob_hash` — camelCase inconsistency possible

**Proposed fix:** Audit all projection collection schemas, normalize to consistent camelCase with clear naming that distinguishes protocol fields from app vocabulary.

## A2O Scenario Connection

These changes touch the delivery pipeline verified by:
- `genesis/a2o/features/delivery/web2-absorption.feature` — cache population uses app_id
- `genesis/a2o/features/delivery/delivery-diagnostics.feature` — observability headers

Add regression scenarios for:
- "Content with slug URL resolves to CID-based cache entry"
- "Re-seeded content with new CID invalidates old slug mapping"

## Memory References

- [Schema before code](feedback-schema-before-code.md) — edit protocol schema first
- [Protocol Schema Contract](project-protocol-schema-contract.md) — IoC pattern
- [Doorway reuses storage compute reporting](feedback-doorway-reuses-storage-compute.md) — one node, one report

## Sprint Approach

This is a rename/refactor sprint — high surface area, low algorithmic complexity. Good candidate for:
1. Write the plan with exact file paths and search-replace patterns
2. Execute via subagent-driven development with parallel agents for independent crates
3. Run full test suite after each crate to catch downstream breakage

Estimated: 1 session, ~8-10 tasks.
