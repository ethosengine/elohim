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
  - "a2o @concern:release-retention (genesis/a2o/features/delivery/release-retention.feature — Act I household, two scenarios: a peer set to keep two releases lets the first of three go while a peer at the default keeps it; and a peer lets go of an earlier build nothing names, keeps a file with no record of how it arrived, and the doorway still serves)"
  - "cargo test --features 'p2p p2p-iroh' --lib -- retention release_ledger holds blob_arrivals shard_service a_forgotten_blob (elohim/elohim-storage)"
  - "fleet read: elohim_node_holds_bytes{store=blobs,reason} per alpha peer, own_unnamed falling at 64 blobs per pass for bytes older than a day and unrecorded naming what nobody can account for, beside elohim_release_retention_releases{standing}, elohim_release_retention_blobs_held{reason} and elohim_node_store_bytes{store=blobs|blobs_iroh|cache}"
first_move: >
  Read elohim_node_store_bytes{store=blobs} on matthew, adam and jessica falling as their
  own_unnamed goes (the holds counters already show the letting go; the store gauge had not
  moved by 04:08Z on 2026-10-07), and doorway alpha still serving / and /version.json after
  matthew's third pass. Those two reads turn this green. Then the standing question: matthew's
  984 MB that read own_unnamed is twice the 449 MB of nineteen releases the ledger gap named;
  what the other ~500 MB was is answered by matthew's recentlyLetGo on /admin/adoption.
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

DELTA 2026-10-06b (household proof of the holds pass; NO status change, stays red until the fleet is read).
The retention sweeper now ends every pass with a holds pass over the whole blob store
(`services/holds.rs`): one reason per blob, and a blob this peer brought here by its own act that
nothing names any more is let go after two consecutive passes and a day (reloadable). Household, both
transports, two runs green on the new binary with the storage peers restarted onto it:
sprint-report-household-20261006T225811Z-f4fe555b and -20261006T231613Z-0820d2ff, 2 scenarios and
32 steps each. The second scenario hands matthew's peer two builds of a channel-less app in turn:
the first build read own_unnamed (arrived self-put) and was gone from matthew's and jessica's stores
within the budget, the second stayed, the file put over the shard route stayed as unrecorded, and
doorway alpha served the second build. 87 lib tests pass with both transports (holds 21, arrivals 6,
retention 11, ledger 3, shard service 22; thirty of them new); the gate's clippy is clean. Conductor: the
mesh ran on the hc-fork-5f4c16abe-proof pair, one fork commit behind the pin (the pin's pair is
not built; the one commit bounds the conductor's wasm store), declared here. What the fixture
taught, both fixed before this delta: a `blob:` manifest is NOT evidence of how a blob arrived
(the manifest backfill stamps one on every local blob; a file put over the shard route had one
three minutes later), so pre-record evidence is now only the put path's self-custody row and this
node's own fetch events; and counting passes was bounded with the letting go, so a blob among 3,500
unnamed was looked at once per fourteen minutes. Under the looser evidence, the first failing run
let go about 1,500 seed-body blobs from 2026-10-04 that no content row on any peer named; the
remaining 2,000 read unrecorded on the fixed binary and stay. Fleet: unread; the first_move names
the read. Blind-reader on the story, a new reader each round, c/i/p: 0/1/3, 1/1/1, 0/2/1, 1/1/0, 1/1/1, 1/3/1; every correctness finding and all but three interpretability findings resolved (left, named for the operator: the Background's E2E_* addresses are not said to be environment variables; the two-app rationale comes after the Background that uses it; the prose does not say the check intervals are set through the peer's runtime-config file); the round-6 one-sentence fixes were not re-read.

DELTA 2026-10-07 (fleet read, the first half of check 3; NO status change yet). Edge #1565 rolled all
seven alpha storage peers to 1.0.0-dev-e0035525 at 03:50Z; the first holds pass published 04:01:30–
04:03:30Z (boot + the 300 s wait + one 300 s interval), and on every peer the holds account summed to
the blob store's bytes exactly. Read at the first pass (MB; blobs): matthew kept 50.9/9, own_unnamed
984.1/238, pledged 162.8/23, served 11.2/6, unrecorded 88.3/15; adam 3.6/1, 487.2/134, 219.5/27,
46.7/11, 83.6/14; jessica 50.9/9, 40.5/75, 222.7/28, 4.1/5, 12.9/2; susan, james, gertrude and eve
3.6/1, 33.5–36.6/8–9, 210–218/25–27, 35/8–9, 9.3–12.9/1–2. Nothing read placed, arriving or part_of.
The forty quiet-peer files were not unrecorded: they read pledged (live custody pledges name ~210 MB
on every peer), served, and eight self-fetched bundles nothing names. At the second pass, five
minutes later, every peer let go: matthew, jessica and adam 64 each (the per-pass cap; 264.7, 24.6
and 265.5 MB), the four quiet peers all eight (33.5 MB each) — the age gate counts from arrival, so
bytes already older than a day go on the second pass, and the day protects new arrivals; the
earlier reading of this habit ("after a day") was wrong about that and is corrected above.
failures_total 0 everywhere; elohim_node_store_bytes had not moved by 04:08Z (first_move). Doorway
alpha answered 200 on /, /version.json and /db/content/elohim-host-landing at 04:07Z. Also found
tonight and fixed on dev (a0601e49d): the household's channel-create miss on 2026-10-06 was not the
lock change but signal handlers left on a dead app websocket after a conductor restart
(166f7a54c); the conductor-volume grow step died on a missing jq and held every conductor after
eve's (a0601e49d); neither is this habit's concern, both are recorded in their own.
