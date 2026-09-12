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
DELTA 2026-09-11b (final sprint measure; RED, causes narrowed): glue now waits the fixture's convergenceWindowMs
(60 s) on the off-pool read and the browser count scenario reads status itself (dc38ed3e5). Runs 20260911T0708{32}Z
/ 070905Z / 070928Z / 071014Z-dc38ed3e: humans-served browser **1/1 GREEN**; humans-served mesh 1/4 — 11 of 17
minted commitments read back live from jessica within 60 s, 6 do not (commitment convergence to a non-authoring peer
exceeds the declared window for some — dataplane-convergence territory, measured for the first time); 07 mesh 0/3 and
07 browser 0/2 — every failure is conductor-0's admin API dying under install_app (`ConnectionAborted`, `BrokenPipe`;
earlier `database is locked`) before any read-back ran: the conductor-contention ceiling of real hosting on one
conductor (runtime-death-witnessed's liveness gap wears this face). 07 scenario 4 (two newcomers, two promises) and
05-leaving 6/6 passed on the same binaries when the conductor was fresh. Flip needs: a run where the conductor is
recycled before the lane AND the 6 slow commitments converge — or the window raised on evidence, not hope.

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
DELTA 2026-09-11c (household re-measure on the running mesh; RED preserved): lane 20260911T223432Z-466536cb (storage sut sha256:371d348555e6d37f, doorway http://localhost:8888): hosted-compute-contracted 0/3 — every scenario stops at `DoorwaySessionError: Invalid credentials` on the story's own freshly registered human (register succeeds, the next authenticated call is refused), so the promise assertions never ran; humans-served 1/4 — `humansServed=1` while 0 of 16 attempted registrants hold a live hosted-cell commitment read from a non-doorway peer, and the close path hits the same Invalid credentials. Reading: the doorway-side count and the substrate-side promise still disagree (D2/D3 of the 2026-09-10 plan, Tasks 19b/21 open), and the credential refusal is a new precondition red to localise before the promise can be measured — it predates nothing in this session (no doorway or storage code changed).
DELTA 2026-09-12 (precondition ladder climbed; RED preserved, one real red left per check): the `Invalid credentials` class was MESH STATE — the 08:08Z cold start dropped the doorway archive and left the prologue roster behind, so the lane read 16 names with no account (fixed 696ff3f77: roster dropped with the archive; `just test mesh` refuses before launch without a fresh cast). Two further preconditions surfaced and were cured in the same push set: the prologue's fixture writer dropped the storage peers' agentPubKey/conductorAppUrl after `storage-restart` had stamped them (hosted scenarios failed "no household fixture storage peer matches the pool conductor's origin"; 93588235e re-stamps), and story humans leaked because a failing close was silently swallowed until the pool conductor hit its 25/25 hosted-agent ceiling (b73c1fe2e: scoped After hook, logged failures). On a cold-started, re-cast mesh with the fixture re-stamped (run 20260912T021354Z-90ba1dc9, storage = origin/dev 586e78085 + fix, iroh): 07-hosted-by-a-household 2/3 — "a newcomer is lent a cell, and the lending is promised" PASSED and "two newcomers are lent two cells under two promises" PASSED, i.e. the notarized delegates-compute promise now measures green on the household mesh; the one red is "its provider is not the doorway's own service identity", which on this 3-peer topology is a fixture limitation, not a code verdict: doorway A's pool conductor IS its primary storage peer (matthew), so provider and service identity coincide by construction — the scenario needs a pool conductor that is not the doorway's primary (e.g. jessica as A's pool) before it can distinguish them. humans-served (run 20260912T021156Z): 2/4 — "hosts nobody says none" and "closing drops the count and re-casting restores it" PASSED; the two reds are the substrate-vs-doorway disagreement the invariant names: humansServed=14 while 13 of 17 cast registrants hold a live hosted-cell commitment read from a non-doorway peer (D2/D3, Tasks 19b/21).

