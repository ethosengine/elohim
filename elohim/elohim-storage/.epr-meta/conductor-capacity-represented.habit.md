---
epr-habit-version: 1
id: conductor-capacity-represented
invariant: >
  The conductor's DB read pool is a REPRESENTED resource, not one discovered
  by timing out against it: every zome call is admitted through one
  process-wide gate sized from db_max_readers, occupancy (L) and hold-time
  (W) are exported so Little's Law is computable, a call that cannot get
  capacity is shed BEFORE dispatch (costing the conductor nothing), and no
  pacing controller closes a loop over a signal that is not monotonic in the
  quantity it controls.
status: green
active: false
checks:
  - "cargo test --lib conductor_admission (elohim/elohim-storage) — 9 tests: the gate bounds concurrency at capacity, a full gate sheds rather than queueing forever, occupancy rises/falls with held permits, background yields to interactive under contention, and sizing mirrors the conductor's own incoming_request_concurrency_limit (db_max_readers − 3)"
  - "cargo test --lib reconcile_rails::tests::aimd_shrinks_on_a_call_level_failure (elohim/elohim-storage) — the Err arm the controller used to never hear from"
  - "cargo test --lib head_batch_resolver::tests::queue_wait_reads_as_headroom_when_the_conductor_stalls_in_the_extern (elohim/elohim-storage) — the sign inversion, stated as arithmetic: a 9s round-trip with 8.9s in-wasm reads as 100ms of queue-wait, UNDER the backpressure threshold, and grows the batch 32→48"
  - "LIVE-LOCAL (GREEN 2026-08-18): python3 app/elohim-app/scripts/hc-admission-probe.py --concurrency 64 --seconds 20 against `just dev start` (or an island conductor + storage). Asserts the gate bounds a REAL conductor: in_flight present and pinned at capacity under load, hold_ms populated per zome, and L = lambda*W closing to within 1%. This is the leg that moved the habit off 'proven to bound a semaphore'."
  - "LIVE-FLEET: elohim_conductor_admission_in_flight present on an alpha pod's /metrics with elohim_conductor_admission_hold_ms populated per zome. This is the leg local proof cannot carry — alpha adds the doorway as a SECOND ungated client of the same pool, real gossip/validation load the gate cannot see, and a cgroup CPU ceiling. Query: elohim_conductor_admission_hold_ms_count and elohim_conductor_admission_in_flight over job=\"elohim-alpha/elohim-edgenode\"."
  - "cargo test --lib admission_egress (elohim/elohim-storage) + cargo test --lib source_chain::tests — a shed reaches the WIRE as backpressure (503 + Retry-After + X-Available-Permits + {status:catching-up}), and no handler launders ADMISSION_SHED_MARKER on the way there"
guard: >
  Regression risk 1 — a NEW conductor call path that does not go through
  HcClient::call_zome{,_mishpat,_imagodei}. The gate covers all 59 current
  call sites precisely because those three methods are the only door; a
  fourth door silently un-gates whatever uses it.
  Risk 2 — a caller that holds an AdmissionPermit across a nested zome call.
  No such path exists today (call_zome is a leaf), and capacity permits with
  N nested callers would deadlock rather than degrade.
  Risk 3 — the doorway is a SECOND process holding its own websockets to the
  same conductor and has no gate at all, so the process-wide bound here is
  not a node-wide one. Any occupancy reading is elohim-storage's share only.
  Risk 4 — the reserve and both wait bounds are env-overridable
  (ELOHIM_CONDUCTOR_PERMITS etc.); a deploy that sets them without occupancy
  evidence re-opens exactly the guessing this habit exists to end.
refs:
  - "spike: genesis/docs/superpowers/specs/2026-08-08-conductor-call-deadline-capability-spike.md §2 — the cancellable/uncancellable table, and the naming of the gap this closes ('no equivalent ceiling for app-interface zome calls')"
  - "elohim/elohim-storage/src/conductor_admission.rs — the gate; the module doc carries the sizing derivation and the admission-vs-timeout distinction"
  - "NOT covered — the AdoptCandidate provenance collapse (an observed Ok(None) mislabelled Unreachable) and the non-batch background callers still classed Interactive; both named in review, neither addressed here"
retire-when: >
  when the conductor exports its own admission state and elohim-storage CONSUMES it rather
  than modelling it — the resource represented by the thing that owns it. This habit's
  local gate is then a compatibility shim, and shims retire.
---
DELTA 2026-08-20 (GREEN — the LIVE-FLEET leg is measured, and it was the
only leg outstanding): elohim_conductor_admission_in_flight and
elohim_conductor_admission_hold_ms are BOTH live on the alpha fleet, on
all 7 edgenode pods, read from Prometheus (job
"elohim-alpha/elohim-edgenode") 2026-08-20 ~22:35Z. hold_ms is populated
per zome — 21 series, 3 zomes (content_store / infrastructure / imagodei)
x 7 pods — with per-pod observation totals 405..1190 (matthew 1190 =
715 content_store + 434 infrastructure + 41 imagodei; adam 506; james 648;
jessica 618; gertrude 546; eve 554; susan 405). in_flight reads 0..2
across the fleet in the same instant (matthew 1, adam 1, susan 1,
gertrude 1, eve 2, james 0, jessica 0). So the gate is not merely
compiled onto the fleet: it is ADMITTING and TIMING real conductor calls
on every peer, and L and W are externally readable per zome — which is
exactly what the invariant asks ("occupancy (L) and hold-time (W) are
exported so Little's Law is computable"), now on the substrate the
invariant was finally about rather than on a local island. The five cargo
legs were re-run host-green on this tree the same day: conductor_admission
9 passed, reconcile_rails::tests::aimd_shrinks_on_a_call_level_failure 1,
head_batch_resolver::tests::queue_wait_reads_as_headroom_when_the_conductor_stalls_in_the_extern
1, admission_egress 3, source_chain::tests 2 — 0 failed, exit 0 each.
WHAT THIS DOES NOT CLAIM, and what the guard still watches: the fleet
capacity A/B has NOT been re-run, so the local sizing verdict (the pool
was OVERSUBSCRIBED; d(lambda)/d(capacity) <= 0) remains a LOCAL one and
nobody should touch db_max_readers on alpha on its authority. Guard risk 3
is also unchanged and now visible in the same window: the doorway is a
second, ungated client of the same pool, so these occupancy readings are
elohim-storage's share only. Green means the pool is honestly
REPRESENTED on the fleet, not that it is correctly SIZED there.
DELTA 2026-08-18b (batch closed on dev — pre-push review pass, 57688ae4a):
the last api-layer launderer of ADMISSION_SHED_MARKER is closed. The wrap
is now canonical in conductor_admission::zome_unavailable_error (source_chain
delegates to it) and attention.rs rides it, so the check clause "no handler
launders ADMISSION_SHED_MARKER on the way there" is true across the api
layer rather than at the two seams cured below. Red-first test, gates green.
STATUS STAYS RED: the LIVE-FLEET leg is untouched by this.
DELTA 2026-08-18 (LIVE-LOCAL leg GREEN; the sizing question is ANSWERED,
and its documented decision rule was falsified). Measured against a real
conductor in the devspace — island peer (bootstrap/signal pointed at a
dead port, so no uncontrolled gossip load), storage release binary,
load driven through GET /api/v1/source-chain/{agent}/entries = one
imagodei query_my_source_chain per request. 15 runs, capacities 4/8/17/
24/34, concurrency 8..128.
(1) THE SERIES ARE REAL: elohim_conductor_admission_in_flight present and
elohim_conductor_admission_hold_ms populated per zome from live calls —
the leg that had never run against a conductor.
(2) LITTLE'S LAW CLOSES: |L_computed(lambda*W) - L_observed(time-avg
in_flight)| <= 0.9% in EVERY run, across every capacity and machine load
(best 0.0%: lambda 1130/s x W 6.12ms = 6.917 vs 6.918 observed). L, W and
lambda are emitted independently, so agreement to under 1% is a real
cross-check, not a tautology. The pool is honestly represented.
(3) THE GATE BOUNDS A CONDUCTOR, not just a semaphore: occupancy never
exceeded capacity in any run and pinned exactly at it under excess demand
(7.83-7.97 at cap 8, 16.84-16.92 at cap 17, 32.60-33.93 at cap 34), with
acquired_total{arrival="saturated"} counting 30,993 of 31,036 arrivals at
cap 17 / concurrency 32.
(4) SHED BEFORE DISPATCH FIRES: at capacity 4 / wait 100ms, 4,538 sheds
counted at the gate, labelled by class+zome, conductor untouched.
(5) THE SIZING VERDICT — the pool was OVERSUBSCRIBED, not undersized.
Doubling permits 17->34 at the same offered load left throughput FLAT and
doubled hold-time (lambda 924->897/s, W 18.2->37.6ms; reproduced on a
second pair, and 1560->566/s with W 10.8->59.7ms on a third). Raising
db_max_readers is CONTRAINDICATED by measurement. This corrects this
module's own written decision rule ("in_flight pinned at capacity with
demand behind it IS the evidence for raising db_max_readers"): pinned
occupancy is NECESSARY but NOT SUFFICIENT — the sufficient test is
d(lambda)/d(capacity) > 0, and here it is <= 0. Correction written into
the conductor_admission module doc so the next reader cannot inherit the
wrong inference.
(6) DEFECT FOUND AND FIXED (red-first, then proven live): a shed was
legible INSIDE the process and illegible outside it. The api/v1 handler
collapsed it into a generic "conductor unavailable" (destroying
ADMISSION_SHED_MARKER), and http.rs's top-level error sink answered every
escaped error with a bare plain-text 500 — bypassing response::
error_response entirely. So a call the conductor NEVER SAW arrived as
"we tried and it broke", with no Retry-After, unclassifiable by any
client. Cured at two seams: HttpServer::escaped_error_response classifies
admission sheds, and source_chain::zome_error stops laundering the marker;
the wire shape has ONE home (response::admission_shed_backpressure) shared
with storage's own request-admission ceiling. Verified live: the same
probe that produced `500 Error: Conductor error: ... conductor
unavailable` now produces `503` + `retry-after: 2` +
`x-available-permits: 0` + {"status":"catching-up","retryAfter":2} — the
exact shape the a2o shed-ride primitive (getRawRidingCatchUp) already
honors, so the saga probes inherit it for free. 6 new tests, red before
green; cargo clippy --lib --all-targets -D warnings exit 0.
STATUS STAYS RED ON PURPOSE: the LIVE-FLEET leg has not run. Local proof
cannot carry it — on alpha the doorway is a second ungated client of the
same pool (guard risk 3), gossip/validation spends permits this gate
cannot see, and the cgroup CPU ceiling binds. The flip evidence wanted is
an alpha pod's /metrics carrying in_flight + hold_ms, and the capacity
A/B re-run there before anyone touches db_max_readers.
Also found, NOT fixed, filed against guard risk 1: ConductorClient::
call_zome (src/conductor_client.rs) is a FOURTH door to the same conductor
with no gate at all, as is src/conductor/client.rs. Both are dormant today
(ContentServerBridge is never constructed in main.rs, and conductor/
client.rs has no callers), so no ungated traffic reaches a live conductor —
but the guard's "a fourth door silently un-gates whatever uses it" already
exists in the tree, one construction call away from being live.
RED WRITTEN 2026-08-11, born red on purpose. Local legs GREEN on the dev
tree (host-verified, not CI): cargo test --lib 2638 passed / 0 failed;
clippy -D warnings and fmt --check both exit 0. What is NOT proven is the
whole claim: nothing has run against a live conductor, so the sizing
(db_max_readers − 3), the wait bounds (5s interactive / 1s background), and
the premise that we were oversubscribing rather than undersized are all
still hypotheses. The measurement is the point — occupancy pinned at
capacity with real demand behind it is the evidence-backed case for raising
db_max_readers in the conductor fork; occupancy well under capacity while
calls still fail says the pool was never the binding constraint and the
diagnosis moves elsewhere. This habit flips green on the live leg, never on
the unit tests.
