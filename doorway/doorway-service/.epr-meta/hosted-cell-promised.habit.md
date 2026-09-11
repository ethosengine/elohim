---
epr-habit-version: 1
id: hosted-cell-promised
invariant: >
  A hosted cell is a promise the notary recorded, not a favour the doorway keeps privately:
  registering as a hosted human yields a delegates-compute commitment (scope hosted-cell)
  whose provider is the steward key the pool conductor's own peer names for itself, readable
  back by cid from any household peer holding the mishpat cell — never only from the doorway
  or its pool — and withdrawn (still readable, ended) when the account closes; a doorway's
  humansServed is the count of such live promises its pool has made.
status: red
active: false
checks:
  - "a2o @concern:hosted-compute-contracted (genesis/a2o/features/auth/hosted-human/07-hosted-by-a-household.feature — @act:i; household lane is the authority: `just test mesh features/auth/hosted-human/07-hosted-by-a-household.feature` for the API-side scenarios and `just test mesh-browser features/auth/hosted-human/07-hosted-by-a-household.feature` for the two @browser-only ones. The story creates and removes its own human.)"
  - "a2o @concern:humans-served (genesis/a2o/features/dataplane/doorway-humans-served.feature — @act:i; needs `just mesh prologue` to have cast the hosted humans: `just test mesh features/dataplane/doorway-humans-served.feature`. The count is doorway-local and substrate-derived; a federation-wide aggregate is a different number and is not this check.)"
refs:
  - "genesis/docs/superpowers/plans/2026-09-10-doorway-federation-three-reds-to-green-plan.md (D2, D3, Tasks 13, 13b, 13c, 15, 16, 17c)"
  - "doorway/doorway-service/src/routes/hosted_cell.rs · elohim/elohim-storage/src/api/compute_grants.rs · elohim/elohim-storage/src/services/commitment_read_fallback.rs"
  - "genesis/data/timeline/backlog/arch-scale-risk-backlog.md row 7 (786 MB conductor heap per hosted human)"
  - "genesis/data/timeline/backlog/doorway-hosted-cell-grant-minted-before-row-insert-2026-09-11.md · commitment-read-fallback-created-at-is-projection-time-2026-09-11.md"
retire-when: >
  when hosted provisioning is itself a substrate act — the pool peer mints the promise as part
  of installing the cell and the doorway only relays it — so a hosted cell without a notarized
  promise cannot exist by construction, and the count is a fold over the DHT, not a doorway row.
---
DELTA 2026-09-11 (BORN red — split out of hosted-human-lifecycle on the day that habit went green, so
its two extension checks stop masking a proven invariant): the promise chain WORKS end to end on the
household mesh — real cell, canonical key, grant cid minted on register (b71b6d66e, 13c), provider = the
peer's own steward key (fixture stamps it from the steward app, 646e244b3/803d422d4), revoke on close
(9957e6369/eff30a245), read-back by cid from a non-pool peer via the DHT fallback (fb4d10c7d, live curl
200 on jessica), humansServed derived (a33e3876a). Measured 2026-09-11 06:52–07:00 (runs
20260911T065255Z / 065417Z / 065903Z / 065959Z-d48f3b69): 07 mesh 1/3, 07 browser 1/2, humans-served
mesh 2/4, browser 0/1. THREE NAMED CAUSES, none product: (1) the off-pool read asserts on the first GET while
the non-authoring peer's DHT view is still converging — the same cid answers 200 on all peers minutes later;
the glue must wait the fixture's declared convergenceWindowMs (fix in flight); (2) `database is locked` on
concurrent install_app against one conductor — contention ceiling; 15 of 17 cast land; (3) humans-served
browser scenario asserts against a status it never read (glue precondition, fix in flight). Also measured:
786 MB conductor heap per hosted human — the mesh recycles conductors between hosted lanes.

