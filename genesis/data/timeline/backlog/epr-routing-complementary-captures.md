---
id: "backlog-epr-routing-complementary-captures"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Complementary captures from the EPR-app routing brainstorm (2026-06-04)"
slug: "epr-routing-complementary-captures"
written: "2026-06-04"
author: "claude"
status: "captured"
priority: "low"
themes: [epr-routing, doorway-projection, observability, ingress-reconcile, memkit]
relatedNodeIds:
  - "genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md"
  - "genesis/docs/superpowers/plans/2026-05-29-substrate-shakeout-epr-delivery-sprint.md"
tags: [captures, D8, doorway]
---

# Complementary captures — EPR-app routing brainstorm

Surfaced while designing §12 of the pillar-EPR decomposition spec (URL & Routing Contract).
Each is adjacent work deliberately NOT absorbed into that design. Domain: D8 (Web2 Projection &
Doorway) unless noted. One line each:

- **Doorway proxy drops `X-Cache`** — storage sets `X-Cache` on app-file hits (`elohim-storage/src/http.rs:4589`) but the doorway proxy forwards only content-type + cache-control (`doorway-service/src/server/http.rs:1395-1404`); cache observability is blind through the doorway. Forward it.
- **Alpha ingress `/lamad/path` SSR-intent rules are dead weight** — `genesis/orchestrator/manifests/elohim-app/alpha/ingress.yaml:91-97` carries route-specific prefixes anticipating SSR that doorway doesn't serve (SSR is `#[cfg(feature="ssr")]` + `/spa/*` only). Reconcile: remove the rules or wire the SSR seam from spec §12.2.
- **Tauri-direct deep-link verification** — storage-side safety-net fallback (§12.2) is what makes `:8090` deep links work; verify on the steward desktop app once Slice 1 lands (tauri-architect).
- **`LamadNotFoundComponent` → designed gate experience** — upgrade the lamad `**` page to the §6 outward-face pattern (preview + hints); gated on gap `#6-2` of the 2026-06-06 Slice-3 spec (`epr-route-claims-link-conformance-design`) — the §6 gate-face UI explicitly deferred out of the Slice-3 implementation (no plan file yet; the gap item is the tracking home).
- **MAP-drift gate misreport** — SessionStart claimed "9 seeds changed since MAP update" but git shows 2 architecture commits since 2026-06-03; the path-currency accumulator may be over-counting (process-meta, memkit subdomain).
- **`/db/paths` list endpoint absent** — all path-list HTTP probes 404 with the conductor-hint envelope; confirm no client depends on a list route (the app loads paths via `/db/content/{id}`), or add the route to storage's manifest if discovery needs it.

## Slice-3 execution captures (2026-06-06, post-execution sweep)

- **Conformance crawler + sweep statuses** (gap `#7-5` remainder, spec §7.5/§3.4) — the a2o crawler walking rendered anchors against the doorway sitemap expected-set; the sweep stamping `claims-stale` (grant's `claimsManifestCid` ≠ bundle manifest CID) and `DEAD-ALIAS` (alias targeting a retired mount). Sitemap + a2o sitemap scenario landed (`385a7485a`, `5dfc3ddc1`); this is the continuous-verification half.
- **MOUNT-arm reach gate is still a 401 wall** — `dispatch_to_projected_epr` (doorway http.rs ~1498) returns hardcoded 401 for `reach != commons` on the *direct pretty-mount* path; spec §5.1's "mounts never see unauthorized traffic / never a wall" applies. Resolve with gap `#6-2`'s gate-face work (route gated mounts to the shell boundary). Tie-back marker added in code.
- **`updateForProfile` static canonical is a real SEO bug** — both seo.service.ts files set every profile's canonical + JSON-LD `@id` to the identical `/lamad/human`; collapse-all-profiles-into-one for search engines. Fix: per-human canonical (`/lamad/human:{username}`, mirroring the governance `/lamad/{type}:{id}` convention); verify the doorway serves that address first. Pre-existing (2025-12/2026-05).
- **trust-badge `/resource/` mints → claims-minted migration** — trust-badge.service.ts (both apps) builds legacy `/resource/{id}` URLs by hand; migrate to `eprToUniversalHref`/claims minting when trust-badge joins the EPR-link surface.
- **`elohim-views` not in any build-manifest source glob** — a views-only change triggers no pipeline; it rides only via co-changed doorway/storage src (pre-existing). Add to the edge manifest's source globs.
- **elohim-app eslint baseline: 603 errors latent** — no push path runs eslint for elohim-app (manifest gate = `just gate` = deps build test); wiring eslint into any gate requires driving the baseline down first.
- **`provideLamadCrossPillarBridge()` — single-source the shell↔lamad token bridge** — the shell root manually mirrors the lamad bundle's cross-pillar token bindings (11 `useExisting` aliases after the 2026-06-06 NullInjector fix); the two roots drift silently whenever a new token enters the viewer chain (four tokens drifted behind the live error). A shared provider fn authored once, consumed by both composition roots, makes the viewer-chain contract single-sourced.
- **"View as Content" on claimed types is a UX contradiction** — post-Slice-3, `/epr/{path-id}` 302s back to the path mount (the claim owns the type, §12.1), so the overview's View-as-Content link round-trips to itself. Decide: remove the affordance for claimed types, or design an explicit raw-node surface. (Surfaced by operator click 2026-06-06; the minted `path-` prefix bug it exposed is fixed.)

## Reference-surface & acquisition audit captures (2026-06-07, spec Appendices D+E)

- **Scenario gaps from the Appendix-D traceability map** — (a) fragment survival post-302 (`/epr/{id}#step/n` → mount → step route); (b) cross-bundle **nav-stack handoff** (back affordance over the boundary, omnibar §4.4); (c) **canonical-correctness** (rendered `rel=canonical` matches the minted universal address); (d) **element Loader CID-verify** (the stack's strongest SRI analog has no a2o pin — corrupt-bytes hard-fail).
- **Load-time bundle integrity** — bundle blobs verify at ingest only; apps-sw cache poisoning class (the v1→v2 white-screen) shows the gap. Decide the load-verification story alongside the #13 link-audit attestation (deterministic-zip + auto cache-invalidation Sprint-2 debt is the prerequisite).
- **Blob capability-by-hash is an undocumented design decision** — `GET /blob/{hash}` has no reach gate; sharing a gated EPR's blob hash shares the bytes. Either canonize capability-by-hash explicitly (with hash-secrecy framing) or design blob-reach enforcement; touches doorway REACH.md + dataplane replication eligibility.
- **BRAINSTORM SEED: EPR acquisition affordances + async pull queue + multipeer striping** (one family, spec Appendix E) — the link affordance ladder (browse → Open-in menu [§7.5 parent] → download/offline → pin-as-peer [REA provide commitment, quilt custody D5] → sync-cluster [parent-EPR closure walk, `ContentToSync`]) riding an **async pull queue** mirroring the publish drain queue (`{total, fetched, pending}`, P1 reconciliation-controller pointed inward), with torrent-style multipeer striping as its transport (compose: apps-sw scored peers × sharding.rs/blob_protocol × elohim-bitswap [today steward-only] × RS(N,K) quilt). p2p-design-gate mandatory; D5+D8+shefa.

