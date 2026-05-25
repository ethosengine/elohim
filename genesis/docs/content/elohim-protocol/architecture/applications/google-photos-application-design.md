---
title: Google Photos — substrate-native media library
tier: architecture
status: Full draft
created: 2026-05-24
authors: Matthew Dowell + Opus 4.7
pillar coupling: lamad (media content), elohim (substrate), imagodei (face / face-cluster identity)
realizes:
  - genesis/docs/content/elohim-protocol/social_medium/epic.md (data sovereignty for personal media; "Emma's photos safely backed up across network"; the medium itself encodes love)
  - genesis/docs/content/elohim-protocol/value_scanner/epic.md (family-memory archetypes — grandparent, caregiver, child; the 4th-birthday photo is the value-scanner moment in reverse — not purchase evidence, but memory made durable)
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md (the eight foundational primitives; D.1 subordination for album→photo, D.3 submerge / cold-photo path, D.6 vision-elohim authoring, D.9 grant-reach for shared albums, D.10 vocabulary governance, D.12 checkpoint for backup-state Resources, D.19 agent-private Attestations for face-cluster privacy, D.20 Layered Commons)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md (iroh-blob as bytes transport; BLAKE3 content-addressing)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md (auto-tag Attestations; agent-private reach discipline)
informs:
  - app/elohim-app/src/app/lamad/ (media-timeline, album, memory-reel Angular surfaces)
  - elohim/sdk/domains/lamad/manifest.json (content_types: photo, video, album, memory-reel; observation_kinds: lamad:image-captured, lamad:media-viewed; attestation_kinds: auto-tag, face-cluster; resource_classifications: backup-state)
  - bridges/google-photos/ (Takeout import — batch-graduate Photo EPRs)
  - bridges/apple-photos/ (same pattern for Apple Photos library export)
defers:
  - Specific vision-model implementation (face recognition, object detection) — application-layer ML, not substrate; vision-elohim wraps whatever model is local
  - Print / order-physical-photo flows — bridges to legacy print services
  - Video transcoding pipeline — observation → transcode-event graduation is a separate spec
---

## The grandma test

Grandma opens the app. She sees: a timeline of family photos — hers, her daughter's, her grandchildren's — arranged chronologically. She taps "Memories" and a curated highlight reel of Maya's 4th birthday last week plays automatically, assembled by her household's elohim-agent without asking her to do anything. She can see all photos tagged "Maya" in one tap, search "birthday cake" and get results from twelve years of family history, and share the birthday album with Grandpa's household with a single button press. The app feels like Google Photos. It is not Google Photos. No company has her photos. No company indexes her family's faces. She can leave with everything at any time.

## Primitive composition

| What you see | Primitive | Notes |
|---|---|---|
| Photo | EPR (`content_type: "photo"`) | thumbnail_cid in EPR metadata; full bytes in iroh-blob via `media_cid`; reach=`agent-private` by default |
| Video | EPR (`content_type: "video"`) | same shape; blob is larger; transcript Attestation optional |
| Album | EPR (`content_type: "album"`) | `parent_epr_cid` links child Photo/Video EPRs per D.1; reach=`household` typical |
| Auto-tag label | Attestation (`attestation_kind: "auto-tag"`) | issued by vision-elohim per D.6; `subject_cid = photo_epr_cid`; reach=`agent-private`; `evidence_json: { label, confidence }` |
| Face cluster | Attestation (`attestation_kind: "face-cluster"`) | issued by vision-elohim; `subject_cid = photo_epr_cid`; `evidence_json: { cluster_id, face_embedding_cid }`; reach=`agent-private` per D.19 |
| Memory / highlight reel | EPR (`content_type: "memory-reel"`) | authored by memory-elohim; references constituent Photo EPR CIDs in `metadata_json`; derived content, no new blob bytes |
| Share album | Event (`action: "grant-reach"`) | widens album EPR's reach to `household` collective or named recipient; per D.9 |
| View a photo | Observation (`observation_kind: "lamad:media-viewed"`) | libp2p only — not DHT; graduates to summary Event at cadence per lamad manifest |
| Backup coverage | Resource (`resource_classified_as: "backup-state"`) | tracks copy-count per Photo EPR across peer mesh + cold archive; subordinate to album EPR per D.1 |
| Cold archive | Commitment (`action: "custody-quilt", tier_floor: "shelved"`) | authored by memory-elohim when photo reaches cold-archive policy threshold; per D.3 |
| Geotag | Attestation (`attestation_kind: "geotag"`) | reverse-geocode from photo EXIF; issued by vision-elohim at capture; reach=`agent-private` |

Eight primitives, ~9 discriminator values, no special-casing for media storage.

## How one photo upload flows

Grandma taps the camera and takes a photo of Maya blowing out birthday candles:

```
1. Camera app writes JPEG to device local storage
   size: 4 MB; EXIF: timestamp, GPS lat/lon

2. Household elohim-agent observes the new file:
       Observation(observation_kind: "lamad:image-captured",
                   subject_cid: <household_epr_cid>,
                   payload_json: { filename, exif_json, local_path },
                   observation_refs: [] )
       libp2p only — no DHT write

3. Graduation policy: image-captured → 1:1 (every capture graduates; no batching)
   vision-elohim picks up the observation, runs local inference:
   a. Generates thumbnail (scaled JPEG, ~50 KB)
   b. Identifies faces → assigns to existing clusters or creates new cluster
   c. Runs object/scene detection → labels ["birthday cake", "children", "candle"]
   d. Reverse-geocodes GPS → "Maria's Kitchen, Oakland CA"

4. vision-elohim authors four DHT entries:

   a. Photo EPR:
          EPR(content_type: "photo",
              metadata_json: { timestamp, exif_summary, thumbnail_cid, media_cid })
          reach: agent-private
          DHT write: ~3 KB entry

   b. Attestation(attestation_kind: "face-cluster",
                  subject_cid: photo_epr_cid,
                  evidence_json: { cluster_id: "maya-cluster", confidence: 0.97 })
          reach: agent-private   ← D.19 enforcement: never leaves household

   c. Attestation(attestation_kind: "auto-tag",
                  subject_cid: photo_epr_cid,
                  evidence_json: { labels: ["birthday cake", "candle", "children"] })
          reach: agent-private

   d. Attestation(attestation_kind: "geotag",
                  subject_cid: photo_epr_cid,
                  evidence_json: { place: "Maria's Kitchen, Oakland CA", lat, lon })
          reach: agent-private

5. iroh-blob plane: full JPEG bytes keyed by BLAKE3 hash (media_cid)
   Thumbnail bytes stored separately by thumbnail_cid
   No gossip — pull-on-view only

6. backup-state Resource authored:
       Resource(resource_classified_as: ["backup-state"],
                parent_epr_cid: album_epr_cid,
                metadata_json: { photo_cid, copy_count: 1, tier: "active" })

7. Album EPR's EprToResource link created (D.1): album → backup-state Resource

8. libp2p sync plane delta-syncs the four new DHT entries to grandma's other
   devices (laptop, tablet) via cursor-tracked sync

9. Local SQL projection updates: photos table, attestations table, resources table
   Dashboard refresh: new photo appears at top of timeline in <100 ms
```

Cash-out: grandma disconnects the app. She gets a ZIP containing original JPEGs with full EXIF, face-tag JSON per photo, album structure, and auto-tag labels — all in standard formats. She can import into any photo library. No proprietary lock-in, no re-negotiation.

## Storage footprint per household

| Item | Count | Size | Total |
|---|---|---|---|
| Photo EPRs (metadata only) | 100k | 3 KB | 300 MB SQL projection |
| Video EPRs (metadata only) | 5k | 5 KB | 25 MB |
| Album EPRs | 500 | 2 KB | 1 MB |
| Memory-reel EPRs | 200 | 5 KB | 1 MB |
| Auto-tag Attestations (avg 4 per photo) | 400k | 0.5 KB | 200 MB |
| Face-cluster Attestations (avg 2 per photo) | 200k | 1 KB | 200 MB |
| Geotag Attestations | 80k | 0.5 KB | 40 MB |
| Backup-state Resources | 100k | 0.5 KB | 50 MB |
| Cold-archive Commitments | 30k | 1 KB | 30 MB |
| **Total local SQL projection** | | | **~847 MB** |
| iroh-blob: full photos (100k × 4 MB) | 100k | 4 MB | ~400 GB local warm |
| iroh-blob: thumbnails (100k × 50 KB) | 100k | 50 KB | ~5 GB local warm |
| Quilt cold archive (photos >2 yr, erasure-coded) | ~50k | variable | ~100 GB cold residue |

**SQL fits on a phone; blob bytes need a household-hub or external drive for full warm storage. Thumbnails fit everywhere; full-res is pull-on-demand.**

## Network bandwidth profile

- New photo upload: ~8 KB DHT write (4 entries) + neighborhood gossip ≈ ~80 KB total peer load; blob bytes stay local until pulled
- Shared album view (family member opening shared album): thumbnail-pull only — 500 photos × 50 KB ≈ 25 MB one-time; full-res fetched per tap
- libp2p sync plane: cursor-driven metadata-only; new EPRs and Attestations delta only
- iroh-blob pull-on-view: one BLAKE3-keyed fetch per full-res view; QUIC transport; sub-second typical on local mesh
- **Per household: <1 GB/month metadata; blob pull is demand-driven (0 if nobody views old photos)**

## DHT entry impact

- 100k photos × 4 DHT entries (EPR + face-cluster + auto-tag + geotag) = 400k entries per household
- But: all entries are `agent-private` reach — they live only on grandma's household peers; not gossiped globally
- Per-peer DHT entry visibility: only the household's own entries + entries in shared-album reach scope
- Shared album grants reach to a recipient household's peers — those peers hold the album's EPRs + referenced Photo EPRs (not Attestations — face clusters do not widen)
- Global DHT: sees zero face-cluster or auto-tag entries from any household; the protocol has no global photo index

## Why the photo timeline renders fast

- **Timeline**: `SELECT * FROM photos ORDER BY exif_timestamp DESC LIMIT 100`. Indexed on `exif_timestamp`. Local SQL, milliseconds. Thumbnails cached in iroh-blob local store.
- **"All photos of Maya"**: `SELECT p.* FROM photos p JOIN attestations a ON a.subject_cid = p.epr_cid WHERE a.attestation_kind = 'face-cluster' AND json_extract(a.evidence_json, '$.cluster_id') = 'maya-cluster' ORDER BY exif_timestamp DESC`. Zero DHT reads; the SQL join over local projection is indexed and sub-100ms for 100k photos.
- **Search "birthday cake"**: `SELECT p.* FROM photos p JOIN attestations a ON a.subject_cid = p.epr_cid WHERE a.attestation_kind = 'auto-tag' AND a.evidence_json LIKE '%birthday cake%'`. FTS on `evidence_json` column; sub-200ms even at 100k.
- **Backup health dashboard**: `SELECT album_cid, COUNT(*), SUM(CASE WHEN lifecycle_state='active' THEN 1 END) FROM resources WHERE classified_as LIKE '%backup-state%' GROUP BY album_cid`. Per A.3's worked example in the records-lifecycle spec — all local SQL, milliseconds.
- **Shared album (cross-household)**: album metadata synced via libp2p to recipient household; thumbnails pre-fetched on first open; subsequent renders are local SQL + cached blobs.

## Why face clusters don't become surveillance

The substrate makes privacy structural, not contractual:

- **No global face index exists.** Face-cluster Attestations are authored with `reach: agent-private` (D.19 enforcement). The integrity zome rejects any `grant-reach` Event that would elevate `attestation:face-cluster` above `household` reach without explicit household-council attestation. No company, no bridge, no elohim agent can widen face-cluster reach without the household's active consent.
- **Clusters don't travel with shared albums.** When grandma shares an album via `Event(action="grant-reach")`, the grant widens the Album EPR and its child Photo EPRs to the recipient's reach scope. It does NOT widen the Attestations. Recipients who view the shared album see photos, not face assignments. They can run their own local vision-elohim to cluster faces from what they can see — but that output stays in their household, not grandma's.
- **The vision-elohim is local.** Per D.6, vision-elohim runs inside the household elohim-node (or a trusted collective elohim-node, manifest-declared). Inference happens locally. Face embeddings never leave the household — they sit in `evidence_json` inside an agent-private Attestation on the DHT, readable only by peers with `agent-private` reach (the household's own devices). See D.19 for the per-Attestation subtype reach enforcement.
- **OCR Attestations follow the same rule.** Text extracted from screenshots, whiteboard photos, or document scans is `attestation:auto-tag` with `reach: agent-private`. The substrate has no full-text index visible to external parties.

## Cold-photo recovery: the D.3 submerge path

Photos older than the household's cold-archive policy threshold (manifest-declared; default 2 years for full-res bytes, never for metadata):

```
1. memory-elohim evaluates photo EPR lifecycle per cold-archive policy
   trigger: photo exif_timestamp older than policy_threshold AND
            photo has had zero views in the past 90 days

2. memory-elohim authors:
       Commitment(action: "custody-quilt",
                  tier_floor: "shelved",
                  subject_cid: photo_epr_cid,
                  metadata_json: { shard_destinations: [quilt_peer_cids],
                                   k_of_n: "4-of-7",
                                   coldness_policy: "household:cold-after-2yr" })

3. ReconcileController fans out:
   a. lifecycle_state projection: photo EPR moves to "shelved"
   b. quilt placement: iroh-blob full-res bytes erasure-coded across 7 pantry peers
      (4-of-7 recovery); BLAKE3 hash retained in EPR metadata_json as media_cid
   c. backup-state Resource updated: tier → "quilt-cold", copy_count → 7 shards
   d. Local warm blob store: full-res bytes GC'd from household-hub disk
      (thumbnail stays warm in iroh-blob local; timeline still renders)

4. Photo remains in timeline (EPR is intact; thumbnail loads instantly from local cache)
   "Full resolution" tap → QUIC fetch from 4-of-7 quilt peers → reconstruct → display
   Typical latency: 1–5 seconds for a 4 MB JPEG on a mesh with healthy pantry peers
```

Re-elevation (grandma decides to re-print a 10-year-old photo):

```
5. Tap "keep in full quality" on the photo
   memory-elohim authors: Event(action: "surface",
                                subject_cid: photo_epr_cid,
                                metadata_json: { new_tier: "active" })
   ReconcileController: lifecycle_state → "active"; iroh-blob re-fetches full-res
   backup-state Resource: tier → "active", copy_count recomputed
```

D.12 checkpoint Commitments are authored quarterly per album by memory-elohim — summarizing byte-count balances across warm + cold tiers so the backup-health dashboard renders without summing 10 years of Commitment history on every load.

## Shared album: the grant-reach flow

Grandma wants Grandpa (separate household) to see Maya's birthday album:

```
1. Grandma taps "Share" on the Birthday 2026 album EPR
2. Household elohim (reach-mutation-elohim, per D.9) authors:
       Event(action: "grant-reach",
             subject_cid: album_epr_cid,
             metadata_json: { target_reach: "household",
                              target_household_cid: grandpa_household_cid,
                              rationale: "family sharing" })
3. D.9 validation: grandma's standing authorizes household-level sharing;
   no council attestation needed below community reach
4. libp2p sync plane: album EPR + child Photo EPRs delta-sync to grandpa's household peers
   iroh-blob: thumbnails pre-fetched; full-res stays lazy
5. Grandpa opens the shared album in his app:
   Timeline renders from his local SQL projection (just synced)
   Thumbnail loads from his iroh-blob local cache
   Tapping full-res photo: QUIC pull from grandma's household-hub or quilt peers
6. Grandma revokes: Event(action: "revoke-reach", subject_cid: album_epr_cid)
   Grandpa's sync plane receives revocation; local projection updates lifecycle_state;
   his iroh-blob GC clears the pre-fetched thumbnails on next pass
   The photos are gone from his app immediately — structural, not dependent on trust
```

## Memories / highlight reel: the elohim-curation path

```
1. memory-elohim runs weekly on household photo EPRs:
       SELECT p.epr_cid, p.exif_timestamp, a.evidence_json AS tags, fc.evidence_json AS faces
       FROM photos p
       JOIN attestations a ON a.subject_cid = p.epr_cid AND a.attestation_kind = 'auto-tag'
       LEFT JOIN attestations fc ON fc.subject_cid = p.epr_cid AND fc.attestation_kind = 'face-cluster'
       WHERE exif_timestamp BETWEEN (now - 1yr - 7d) AND (now - 1yr)
       ORDER BY exif_timestamp

2. memory-elohim selects ~15 photos using diversity-of-faces + label-variety heuristics
   (runs locally; no external API; heuristics manifest-declared per lamad pillar)

3. Graduates the curation as a derived EPR:
       EPR(content_type: "memory-reel",
           metadata_json: { constituent_photo_cids: [...15 cids...],
                            theme: "Maya's 4th Birthday — 1 year ago",
                            curated_by: "memory-elohim" })
       reach: household

4. No new blob bytes — the memory-reel references existing Photo EPR CIDs
   Dashboard surface: "1 Year Ago" card appears in timeline
```

## Where agentic intelligence carries the load

- **Without vision-elohim**: 100k household photos are un-searchable beyond filename. Face clusters require manual tagging (Google asks you to do this; families don't). Auto-tags require manual effort nobody has. Family memory becomes a flat chronological dump.
- **With vision-elohim**: local inference (manifest-declared model, runs on household-hub CPU or NPU) tags photos at capture time; clusters faces without sending embeddings anywhere; reverse-geocodes GPS silently. The household's private photo library becomes searchable in seconds without a Google account.
- **Without memory-elohim**: "Memories" is a manual effort — grandma has to remember to make a collage. Nobody does this at scale.
- **With memory-elohim**: the highlight reel appears weekly, drawn from the household's own observation of which photos are valued (views, shares, face-cluster density). Family memory becomes actively curated by an agent that only the household authorizes.
- **Without cold-archive-elohim**: old photos accumulate until the hard drive fills. The human's choices are: delete everything or pay for unlimited cloud. Neither is dignified.
- **With cold-archive-elohim**: photos move gracefully to cold archive per household policy. Thumbnails stay warm. Full-res recovers in seconds. The household never loses a photo and never pays a cloud subscription. **This is the value-prop unlock for personal media sovereignty.**

## Dissolution: the D.7 path

Grandma decides she wants to permanently remove a photo (right-to-be-forgotten for a moment she regrets sharing):

- `Event(action: "dispose", subject_cid: photo_epr_cid)` authored directly (or via household-elohim if she used natural language: "delete that photo forever")
- Integrity zome validates: disposals on agent-private photos require no council; disposals on community-reach photos require household + community standing
- ReconcileController: lifecycle_state → "closed"; no future Events can bind to the closed photo EPR; existing Event-history remains queryable to the household (audit trail); iroh-blob full-res bytes GC'd from all peer stores via the quilt revocation path
- Attestations referencing the photo are NOT cascade-deleted — they reference a closed EPR; the integrity zome will reject future reads that attempt to surface the photo's content, but the Attestation structure remains as evidence the photo existed (mishpat-governed right-to-forget operates at the content level, not the structural level)
- Album EPR: photo's subordination link is updated to "closed-subordinate"; album's photo count decrements in derived view

## Bridges: legacy interop and cash-out

- **bridges/google-photos/** — Google Takeout import only (Takeout ZIP → parse `metadata.json` per file → batch-graduate Photo EPRs under stewardship-elohim signature; face-tags in Google's JSON → graduated face-cluster Attestations; albums → Album EPRs with `parent_epr_cid` links). No write-back to Google (stewardship-elohim doesn't have write access). Cash-out is in the opposite direction: the household's photos export with original metadata + face-tag JSON + album structure → importable by any photo library.
- **bridges/apple-photos/** — same pattern for Apple Photos library export (`.photoslibrary` database → Photo EPRs; Smart Albums → Album EPRs; People albums → face-cluster Attestation seeding).
- **bridges/print-service/** (deferred) — Photo EPR CID → print-service HTTP API; the bridge authors an Event(action="deliver-service") when the print ships.

The protocol's commitment is bidirectional legibility: anyone can leave with their data; anyone can join with their history. No household's family memory is hostage to a vendor's pricing decision.

## Code anchors

| Surface | Path |
|---|---|
| Media-timeline Angular services | `app/elohim-app/src/app/lamad/` (media-specific services) |
| Photo / album views | `elohim/elohim-storage/src/views.rs` (`ContentView` with `content_type: "photo"` / `"album"` discriminator) |
| View schemas | `elohim/sdk/schemas/v1/views/content-view.schema.json`, `attestation-view.schema.json`, `economic-resource-view.schema.json` |
| Lamad pillar manifest | `elohim/sdk/domains/lamad/manifest.json` (photo/video/album/memory-reel content_types; `lamad:image-captured` / `lamad:media-viewed` observation_kinds; `auto-tag` / `face-cluster` attestation_kinds; `backup-state` resource_classification) |
| Vision-elohim agent (planned) | `app/elohim-app/src/app/elohim/elohim-agents/vision-elohim.service.ts` |
| Memory-elohim agent (planned) | `app/elohim-app/src/app/elohim/elohim-agents/memory-elohim.service.ts` |
| iroh-blob store | `elohim/elohim-storage/src/p2p_iroh/blob_store.rs` (`IrohBlobStore` — BLAKE3-keyed; `add_bytes` / `get_bytes` / `has`; Phase 2 wrapper) |
| Entry types (integrity zome) | `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs` (Content EPR, EconomicResource, Attestation entry types; EprToResource link type per D.1) |
| Coordinator functions | `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` |
| ReconcileController | `elohim/elohim-storage/src/reconcile/controller.rs` (post-commit projection fan-out for cold-archive Commitments + surface Events + grant-reach Events) |
| Graduation evaluator (planned) | `elohim/elohim-storage/src/services/graduation_evaluator.rs` (per-pillar tokio task; drives image-captured → EPR + Attestations path per D.6) |
| Google Photos bridge (planned) | `bridges/google-photos/` |
| Apple Photos bridge (planned) | `bridges/apple-photos/` |

## What this proves about the substrate

A skeptical systems architect should walk away from this archetype able to say:

- The iroh-blob BLAKE3 content-addressing handles a Google-Photos-shape working set (400 GB warm + 100 GB cold) at household scale with zero centralized storage service — bytes live on the household-hub or in the quilt's pantry peers, pull-fetched on demand via QUIC
- The agent-private Attestation reach model (D.19 discipline) makes face recognition structurally private — not policy-private, not contractually private, but cryptographically and gossip-topology private: face-cluster entries never appear on peers outside the household's reach scope
- The subordination primitive (D.1) makes album→photo a cost-shedding hierarchy: 100k Photo EPRs subordinate under ~500 Album EPRs; photos don't independently gossip unless elevated; the DHT carries only what earns its bandwidth
- The quilt cold-archive path (D.3 + iroh-blob erasure coding) gives household-scale media storage a graceful lifecycle that doesn't require infinite disk — old photos demote automatically, recover in seconds when needed, and cost the household nothing beyond the peer mesh they're already part of
- The grant-reach Event pattern (D.9) makes cross-household sharing structural and revocable — the moment reach is revoked, the recipient's projection updates and their blobs GC; no "delete everywhere" button, no trust required, no vendor to call

If those five claims hold for Google Photos, the substrate's blob-and-metadata architecture is real at the most demanding consumer-scale media storage pattern.
