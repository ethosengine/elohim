---
epr-habit-version: 1
id: release-retention
invariant: >
  A peer holds the bytes of the latest releases of each channel it follows, up to its own
  declared depth (default ten), and lets the bytes of older releases go from every store that
  holds them. Nothing a peer serves is ever let go.
status: red
active: false
checks:
  - "a2o @concern:release-retention (genesis/a2o/features/delivery/release-retention.feature — Act I household: a peer set to keep two releases lets the first of three go, a peer at the default keeps it, the doorway still serves)"
  - "cargo test --features 'p2p p2p-iroh' --lib -- retention release_ledger a_forgotten_blob (elohim/elohim-storage)"
  - "fleet read: elohim_release_retention_releases{standing} and elohim_release_retention_blobs_held{reason} per alpha peer, beside elohim_node_store_bytes{store=blobs|blobs_iroh|cache}"
first_move: >
  Read the fleet gauges after the first deploy that carries this. Then reclaim what predates
  the ledger: matthew holds at least nineteen releases' bundles that no ledger row names, so
  the pass will never touch them. The custody-commitment history names each bundle an app
  once pointed at and is the likeliest source for a one-time backfill; it has not been read
  on the fleet.
refs:
  - "genesis/docs/content/elohim-protocol/architecture/2026-10-05-storage-physics-benchmark.md — §3 fleet read, §7 retention floor, §8 declared defaults"
  - "elohim/elohim-storage/src/services/release_adoption/retention.rs"
  - "elohim/elohim-storage/src/db/release_ledger.rs"
retire-when: >
  when the retention window and its holders are declared by the channel or the holon with
  standing (brit/rakia) and enforced from that declaration, so a per-peer depth setting and a
  peer-local ledger are no longer what bounds a peer's release bytes.
---
DELTA 2026-10-05 (born RED; household proof passes, fleet unread): the retention story passed twice on the household mesh, dual transport, 14/14 steps each (sprint-report-household-20261005T211245Z-bc0692fd, -211833Z-bc0692fd; 4m53s and 4m06s). matthew's peer at depth 2 let the first of three releases go (ledger held 2, released 1, 3,664 B from the blob store and 3,664 B staged, 0 failures, 0 blobs held as served or pledged); jessica's at the default 10 kept all three; doorway alpha served the third. 15 lib tests pass with both transports compiled (retention 11, ledger 3, the iroh store's collection 1). RED because the fleet is unread and matthew's existing backlog predates the ledger, so this pass cannot reach it. Not exercised on the household: a bundle large enough to live as a file in the iroh store (the fixture bundles are under its inline threshold; the unit test covers the file case), and a release held back by a live custody commitment (none stood). Found on the way: the household coordinator refuses the steward's direct earned declaration ('invocation authority: assigned, function-listed mandate required', rule from e9e0b3b63, 2026-10-01), which the app-bundle-elected-delivery story's baseline, promote and revert steps use; this story publishes staging releases only. Blind-reader on the story: 6 rounds (c/i/p: 0/5/2, 0/2/3, 0/3/3, 0/1/3, 0/1/2, then 0/1/3 on the staging-only rewrite), every interpretability finding resolved; the last one-phrase fix was not re-read.
