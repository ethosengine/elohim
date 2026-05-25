---
title: WordPress-shape composed SPA / personal site on the substrate
tier: architecture
status: Horizon (coherent pattern, not on active subsumption path)
created: 2026-05-24
authors: Matthew Dowell + Opus 4.7
realizes:
  - genesis/docs/content/elohim-protocol/social_medium/epic.md
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md (web2 projection)
defers:
  - Active implementation — the active social-medium targets (Meta/Facebook + Patreon) carry the load-bearing reach + creator-monetization patterns; site-publishing as such falls out of those
---

## Why this is a horizon, not active

WordPress-shape personal sites and blogs are coherent on the substrate — every post is an EPR, every site is a composition of Posts, doorway projects to web2 readers. But "publish a blog" is less load-bearing as a subsumption target than the creator-monetization (Patreon) and social-graph (Meta) flows that actually power people's reasons to publish. Once those are real, a WordPress-shape site is largely assembly: the same Posts shown in a different doorway projection.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Site | EPR (`content_type: "site"`) | root vessel for the author's publication |
| Post | EPR (`content_type: "post"`) | `parent_epr_cid = site_cid`; body inline or in iroh-blob |
| Page | EPR (`content_type: "page"`) | static; child of Site |
| Theme | Content referenced by Site | not embedded; swap-able |
| Media asset | EPR (`content_type: "media-asset"`) | inline or iroh-blob |
| Comment | FeedbackSignal (`signal_kind: "comment"`) | |
| Like / share | FeedbackSignal | reach-gated |
| Page view | Observation (`observation_kind: "social:page-view"`) | graduates to traffic-summary Attestation |
| Plugin | Manifest-declared extension | not opaque code blob |
| Subscriber | Commitment (subscription) | child of Site |

## Stress points the substrate handles

- **Composability**: site IS a SQL query over the author's Posts (local SQL); theme is a swappable rendering layer; plugins are manifest-declared extensions
- **SEO / reach**: reach=commons makes posts discoverable via the substrate's distributed index; doorway projection makes them browser-readable for readers without Holochain peers
- **Multi-author governance**: site as Collective EPR with member-authors; each Post has its author signature
- **Migration from legacy WordPress**: bridge crate ingesting WordPress export XML → batch-graduated Events authoring the Posts under stewardship-elohim signature

## Why this is deferred

The substrate's first reach-and-creator wins come from Meta + Patreon, where the social-graph + monetization flows have the most active user pull. Once those carry their weight, a WordPress-shape personal site is a composition pattern, not a new substrate move.
