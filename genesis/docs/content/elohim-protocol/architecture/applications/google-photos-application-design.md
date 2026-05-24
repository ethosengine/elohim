---
title: Google Photos — substrate-native media library
tier: architecture
status: Composition draft (primitives mapped; full walkthrough pending)
created: 2026-05-24
authors: Matthew Dowell + Opus 4.7
pillar coupling: lamad (media content), elohim (substrate), imagodei (face / face-cluster identity)
realizes:
  - genesis/docs/content/elohim-protocol/social_medium/epic.md (data sovereignty for personal media)
  - genesis/docs/content/elohim-protocol/value_scanner/epic.md (family-memory archetypes — grandparent, caregiver, child)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md (auto-tag Attestations)
informs:
  - app/elohim-app/src/app/lamad/ + elohim pillar for media surfaces
  - elohim/sdk/domains/lamad/manifest.json (content_types: photo, video, album; observation_kinds: media-viewed)
defers:
  - Specific vision-model implementation (face recognition, object detection) — application layer ML, not substrate
  - Print / order-physical-photo flows — bridges to legacy print services
---

## The grandma test

A grandparent opens the app. They see: a timeline of family photos, recent uploads from their kids and grandkids, auto-curated "memories" (this week last year), face-clusters ("everyone tagged as Maya"), shared albums. They tap a photo from Maya's 4th birthday last week — it loads from the local cache or pulls from her parent's elohim-node. Photos-shape — but no Google has it, no Google indexes it, no Google can revoke it.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Photo | EPR (`content_type: "photo"`) with `media_cid → iroh-blob` | thumbnail in EPR; full bytes pull-fetched |
| Video | EPR (`content_type: "video"`) with `media_cid → iroh-blob` | same shape; larger blobs |
| Album | EPR (`content_type: "album"`) | `parent_epr_cid → children` (photos and videos) |
| Auto-tag | Attestation (`content_type: "attestation:auto-tag"`) | referencing photo by CID; issued by elohim-vision-agent |
| Face cluster | Attestation (`content_type: "attestation:face-cluster"`) | links multiple photos via `subject_face_cid` |
| Memory / highlight | EPR (`content_type: "curation"`) | elohim-curation creates this; references constituent photos |
| Share | Event (`action: "grant-reach"`) | album reach widens to specific recipients or community |
| View | Observation (`observation_kind: "media:viewed"`) | libp2p; graduates to summary Event |
| Geotag | derived from photo metadata + reverse-geocode Attestation | |
| Backup state | Resource (`resource_classified_as: "backup-state"`) | tracks how many copies of each photo exist across peer mesh + cold archive |

## Stress points the substrate handles

- **Massive blob storage** (10s of thousands per household): bytes in iroh-blob, popular sharing replicates by demand, cold photos in quilt with K-of-N recovery
- **Auto-organization**: vision-agent issues Attestations referencing the photo CID; search is local-SQL-FTS over Attestations + metadata
- **Family / multi-household sharing**: shared album = grant-reach Event; album EPR's reach extended to a household-collective; each family's elohim-node syncs the album metadata; bytes pull-fetched on view
- **Privacy of original photos**: by default `agent-private` reach; auto-tag Attestations stay private until explicitly shared; face clusters never leave the household unless the family opts in
- **Cross-device sync**: each device runs its own elohim-node OR is a thin client to the household-hub; photos sync via libp2p cursor-tracked sync plane

## Scale answer

- Per-household: 100k photos × 2 KB EPR metadata = 200 MB SQL; bytes in iroh-blob ~500 GB local + cold backup
- Auto-tags: thousands of Attestations per household × 1 KB = few MB SQL
- Per-photo gossip cost: only the EPR metadata gossips; bytes stay in iroh-blob (pull-on-view)
- 8B humans × 100k photos = 8 × 10¹⁷ photos globally — but distributed across peers; no central system

## Bridges to legacy

- **bridges/google-photos/** (import) — Google Takeout → batch-graduated Photo EPRs under stewardship-elohim with original metadata preserved
- **bridges/apple-photos/** (import) — same shape for Apple Photos library export
- **Cash-out**: photos export with original metadata + face tags + albums preserved; portable to any photo library

## Code anchors

| Surface | Path |
|---|---|
| Media surfaces | `app/elohim-app/src/app/lamad/` (media-specific) |
| Content views | `elohim/elohim-storage/src/views.rs` (`ContentView` with media discriminator) |
| Vision agent (planned) | `app/elohim-app/src/app/elohim/elohim-agents/vision-elohim.service.ts` |

*Full draft pending.*
