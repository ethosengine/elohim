---
id: "backlog-landing-shard-byte-divergence-post-custody-refix"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-host-landing: the two doorways hold DIFFERENT bytes for the same slug at the shard/custody-manifest layer — served-head equality does not imply byte equality"
slug: "landing-shard-byte-divergence-post-custody-refix"
written: "2026-07-31"
author: "claude (cards RCA)"
status: "open"
priority: "high"
tags: [dataplane, replication, custody, shard-manifest, elohim-host-landing, doorway, alpha, reach-head-replication-planes]
cites:
  - genesis/data/timeline/backlog/content-head-election-vs-reach-fork-arbitration.md
  - genesis/data/timeline/backlog/dataplane-peer-fallback-and-blob-replication.md
  - genesis/data/timeline/backlog/alpha-a-projector-chronic-catchup-flap.md
  - elohim/elohim-storage/src/services/self_stewardship.rs
  - elohim/elohim-storage/src/p2p/mod.rs
---

# The landing page's two doorways serve honest but DIFFERENT bytes

Evidence (cards RCA, 2026-07-31, commit context `429e2f669` "re-claim self-held
custody after a re-key"): after that fix each doorway now honestly reports its own
single steward for `elohim-host-landing`'s shard — but the two doorways' shards are
still not the SAME bytes.

- alpha-A serves a shard `sha256-30011cff…` (9,994,802 B).
- elohim.host serves a shard `sha256-de394363…` (9,990,975 B).
- Each doorway's `GET /blob/{hash}` returns 200 **locally** for its own hash — neither
  doorway is lying about what it holds; they simply hold different objects for the
  same slug.

## This is the replication plane, not the head-election plane — say so precisely

The memory principle applies directly:
[[project_reach_head_replication_distinct_planes]] — reach (audience, earned) ≠
`content_head` (version, declared) ≠ replication (availability, custody) are three
orthogonal planes, and landing-page divergence has repeatedly turned out to be a
**replication** bug, never a head-election question. Two prior items already worked
through exactly this slug at the head/pointer layer and both concluded the same:

- `content-head-election-vs-reach-fork-arbitration.md` (2026-07-09→11): the earlier
  landing divergence was ONE intended head that failed to CONVERGE across peers —
  "replication exonerated" once both peers were shown to serve 200 for **each
  other's** bundle hash; the located gap was serve-path/declared-head-honoring, not
  byte availability.
- `dataplane-peer-fallback-and-blob-replication.md` (2026-06-29, supersession note
  2026-07-31): the earlier gap was a NULL `blobHash` **pointer** on elohim.host
  (bytes present, pointer missing) — items 2 and 3 of that plan (heal-on-read via
  `race_fetch`, EPR-head-aware syncing status) have since LANDED.

**Today's finding is a THIRD, more precise layer than either prior record**: both
doorways now have a non-null pointer, both resolve to *a* shard, both shards are
individually retrievable — but the shard **hash itself differs** between doorways
for the same slug. Same-day earlier probes (per the prompt evidence) showed
identical declared head + identical served blob on both doorways, so this divergence
was not present at the served-head layer earlier — it is localized to the
shard/custody-manifest bookkeeping, most plausibly touched by (or adjacent to) the
same-day `429e2f669` custody re-claim fix, though causation is NOT yet established.

## Why this matters for ch10

The resiliency-saga's ch10 "both doorways tell the same truth" needs same-BYTES, not
merely same-declared-head. A learner (or a diversity-placement/salvage pass) reading
`elohim-host-landing` from alpha-A vs elohim.host today gets two different objects
of nearly-identical-but-not-equal size — a silent content fork at the byte layer that
the head-equality checks in the cited prior items would NOT catch, because they
verify head-pointer agreement, not shard-hash agreement.

## Open question — which artifact diverges (RCA not yet done)

Exactly located to the shard/custody-manifest layer, not the served-head layer, but
which SPECIFIC artifact is wrong is still open:

1. **Shard manifest**: did each doorway's `shard_manifests` row get built from a
   different source blob (e.g. two independent per-host builds notarized
   separately, echoing the "each host independently builds + notarizes" root cause
   the head-election item already named for the pointer-layer split)?
2. **Encoding**: do the two ~9.99MB objects differ by a non-deterministic build step
   (asset ordering, timestamp-embedding, compression level) rather than genuinely
   different content — the ~4KB size delta (9,994,802 vs 9,990,975 = 3,827 B) is
   small relative to the whole bundle, consistent with either a metadata/timestamp
   delta or a small asset-set difference?
3. **Stale shard row**: is one doorway's `shard_locations`/manifest row a leftover
   from a prior build that never got superseded when a newer bundle was produced —
   i.e. is one of the two hashes simply OLD?

None of these is confirmed. The next step is a diff of the two blobs' contents (not
just hashes) to see whether the delta is metadata-shaped or content-shaped, cross-
referenced against `shard_manifests` write timestamps on each doorway and the deploy
history around `429e2f669` and the per-host build+notarize step named in the cited
head-election item.

## Status

**Open, needs RCA.** Blocked on: (1) pulling both blobs and diffing them
byte-for-byte to classify the divergence (metadata vs content vs staleness); (2)
correlating `shard_manifests` write timestamps per doorway against recent deploys.
Do not conflate with the head-election or pointer-propagation items above — both of
those are cited as prior art for the SAME slug at DIFFERENT layers, and both
consider themselves resolved or landed at their respective layers; this item exists
because a further, deeper-layer divergence surfaced after those fixes.

shift_objective: |
  RCA the elohim-host-landing shard-hash divergence between alpha-A
  (sha256-30011cff…, 9,994,802 B) and elohim.host (sha256-de394363…, 9,990,975 B).
  Fetch both blobs via their doorways' `/blob/{hash}` routes, diff them to classify
  the ~3,827-byte delta as metadata/encoding-shaped or content-shaped, and check each
  doorway's `shard_manifests` row write-timestamp against recent deploy history
  (especially around `429e2f669`) to determine whether one side is simply stale.
  Cross-reference `elohim/elohim-storage/src/services/self_stewardship.rs` (custody
  re-claim / manifest backfill path) for whether the re-claim fix could itself
  author a manifest row against a locally-rebuilt (and therefore differently-hashed)
  blob rather than the peer's existing one. Land a fix once the layer is identified;
  do not re-open the head-election or pointer-propagation items to explain this —
  they are a different layer, already addressed.
