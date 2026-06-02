---
status: Draft
cites:
  - ../specs/2026-05-25-pillar-epr-decomposition-design.md   # the design spec this plan implements
---

# B16 Disposition — EPR Resolver Endpoint for MVP

**Date:** 2026-05-25
**Plan:** `genesis/docs/superpowers/plans/2026-05-25-pillar-epr-decomposition-plan.md` Task B16
**Branch:** `design/pillar-epr-decomposition`

## What Task B16 said

> Add `GET /api/v1/epr/{id}` to doorway as the HyperCard resolution endpoint
> consumed by `<elohim-epr-link>` for inline card-flip rendering.

## What we landed instead

**Nothing new — the resolver is `GET /db/content/{id}`, which already exists
and is already proxied through doorway via storage's `build_manifest()`.**

The `<elohim-epr-link>` primitive (`app/elohim-elements/elohim-core/src/elohim-epr-link.ts`)
takes a `.resolver` property — an async function injected by the consuming
bundle. For MVP, that function calls `GET /db/content/{id}` directly and
maps the returned Content row to the `EprLinkResolution` shape the
primitive expects (title, description, pillar, reach, etc.).

## Why this is the right call

Three constraints converged:

1. **doorway's no-per-domain-proxy discipline** (`doorway/CLAUDE.md` +
   `doorway/doorway-service/src/server/CLAUDE.md`): any route that's a thin
   wrap of a storage endpoint must live in storage's `build_manifest()`,
   not as a hand-coded match arm in doorway's `http.rs`. Adding a thin
   doorway-side `handle_epr_resolve` would be the 14th proxy file the
   architecture explicitly deleted.

2. **storage already has GET /api/v1/epr/{cid}** at
   `elohim/elohim-storage/src/api/epr.rs:193` (handler `get_epr`). But it
   serves the full EPR atom + envelope/payload/verify/providers/nav-context
   surface, which requires the EPR-atom data layer to be populated. For
   MVP our 2 EPRs (`elohim-host-landing`, `lamad-spa`) are Content rows,
   not EPR atoms. Calling `get_epr` against them would 404.

3. **storage's GET /db/content/{id}** is already in `build_manifest()`,
   cached for 300s, marked `public_if_reach("commons")`, and handled by
   `get_content`. doorway proxies it automatically. For Content-row-based
   EPRs (which is what MVP has), this IS the resolution endpoint.

## What this means for downstream tasks

- **B17–B18 (lamad bundle split):** when the lamad bundle imports
  `<elohim-epr-link>` from elohim-core, the bundle code wires
  `link.resolver = async (epr) => fetchAndMap(epr, '/db/content/' + epr)`.
  Documented in B18's bundle bootstrapping.

- **B22 a2o scenarios:** the "EPR-link with display=chip resolves content
  inline" scenario verifies the primitive renders title + metadata from
  the resolved `/db/content/` response. No `/api/v1/epr/` endpoint is
  asserted.

- **Post-MVP (EPR-atom substrate landed):** when EPR atoms exist as
  first-class entries, add `/api/v1/epr/{cid}` to `build_manifest()` and
  update the bundle's resolver to call that path. The primitive itself
  doesn't change — only the bundle's resolver wiring.

## Files touched

None. This is a "do nothing in code; record the architectural choice"
task closure.

## Cross-references

- `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md`
  §5.2 (Scenario B — EPR-link click) — describes the conceptual endpoint;
  doesn't bind it to a specific URL path.
- `app/elohim-elements/elohim-core/src/elohim-epr-link.ts` lines 33-42 —
  defines the `resolver` property surface; documents that production
  wiring happens at the bundle level.
- `doorway/doorway-service/src/server/CLAUDE.md` — the no-per-domain-proxy
  gate that ruled out the doorway-side handler.
