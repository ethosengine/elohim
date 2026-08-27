---
epr-habit-version: 1
id: doorway-failover
invariant: >
  The apex name serves a person the landing shell through the loss OR
  catch-up shed of either doorway: siblings are honestly classifiable
  (serving | shedding | dead), at least one always serves, the name rides
  through a sibling's shed, and whichever serves resolves the same
  declared head. Federation failover, not per-host luck.
status: green
active: true
checks:
  - "a2o @concern:doorway-failover (genesis/a2o/features/dataplane/doorway-failover.feature — @act:i, so its authority is the household lane: `just test mesh features/dataplane/doorway-failover.feature` against the built binaries, run-identified report under genesis/a2o/reports/. It is HELD on the edge Dataplane Validation stage by LAYERS.md design (Act II drops owned-substrate) — a fleet build number can never measure it; the fleet contributes only the deploy that carries the same commit.)"
refs:
  - "genesis/docs/superpowers/plans/2026-07-31-doorway-federation-failover-sprint-plan.md"
  - "genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md"
  - "2026-08-06: one layer below this invariant, doorway ZomeCaller grew multi-conductor failover for agent-agnostic DHT reads (reviewed ship, gates green — backlog conductor-source-chain-unwrap-panic-db-timeout.md), so a doorway's DHT routes no longer die with its pinned conductor; the apex-name born-red and its WS3 cure surface are unchanged"
  - "2026-08-07: live evidence toward the invariant — through the Wave-2 iroh transport flip (edge #1313, full 7-conductor restart) BOTH hostnames served the federation route with no visible outage window (measure 0->2 in minutes, vs the ~2.5-3h tx5 pre-fix churn the same night); attribution failover+fixed-binary+fresh-peer-stores, not decomposed; the a2o @concern:doorway-failover scenario remains the flip-to-green authority"
retire-when: >
  when the apex no longer depends on a doorway pair: a person's client resolves the
  declared head from the mesh directly. At that point "either doorway can die" is a
  statement about a component that no longer sits in the path.
---
DELTA 2026-08-25b (RED -> GREEN on measured evidence; the flip authority
was mis-assigned): the fleet's first Dataplane Validation admitted by the
quiesce gate since the acts layering (edge #1381, validate-only on
286089679/47fb60f58, A-QUIESCED sustained 06:54:51Z) HELD 81/87
scenarios — `owned-substrate (act i baseline)` is unavailable on alpha by
LAYERS.md design, and doorway-failover.feature is @act:i. So "flip
authority is the fleet lane" (2026-08-23h) named an authority that
cannot exist for this check; the household lane is it. Evidence for
green: @concern:doorway-failover 10/10 (41 steps) on the household mesh
from the p2p+p2p-iroh binary built from 608a1ceff (run
2026-08-25 push-verify, report genesis/a2o/reports/push-verify/doorway-failover.json;
the same doorway + storage commit deployed to alpha by edge #1380
SUCCESS and confirmed live: irohNodeId, 6 peers, syncDocuments 5356),
after 10/10 on 2026-08-23h and 2026-08-23b. Both named reds stay cured
(primary-scoped health classification 530431088, guarded half-open
probe 99df9f72c). What the fleet DID measure: substrate seam-smoke OK on
both doorways and both relays; the one fleet red is inventory-convergence
(alpha-A p2p.caughtUp flap — matthew's heal-leg class), not a failover
property. Correction recorded: `epr flow note --kind correction` on the
feature. Re-red condition: any household run with a failing
@concern:doorway-failover scenario, or a fleet incident where the apex
name fails to ride a sibling's shed.
DELTA 2026-08-23h (both NAMED REDS cured; status stays red — flip
authority is the fleet lane): (a) health classification scoped to the
DECLARED PRIMARY (530431088) — ServingHealth::observe takes the primary
from the same declared order select_route ranks (one derivation,
config.rs declared_storage_peers); pool upstreams stay visible with an
additive role field; red-first proven by reverting the filter (2 tests
fail on the old any()). (b) periodic guarded half-open probe
(99df9f72c) — open pool circuits are trialed via the breaker's own
begin() RAII guard from the 30 s refresh loop (separate 2 s client, no
second admission path, trial-theft suite re-green; disable knob
DOORWAY_BREAKER_PROBE_DISABLED). 1086 doorway tests green. Independent
adversarial review: probe + scoping CLEAN (sequential probe N x 2 s
noted as follow-up); all five deployed doorway manifests declare a
primary so the None fallback never fires deployed. Local lane on the
cured binaries: @concern:doorway-failover 10/10 (run
20260823T212812Z-ad5d8f49, sut 9a94e137, transport dual). Confirm with
[build:edge] — this arc has real code to deploy, not measure-by-deploy.
DELTA 2026-08-23b (continuity session — selection-time blob failover + RS
ingest + roadmap; household lane closing run 20260823T150653Z-3a77a458):
189 passed / 25 failed / 4 pending / 28 skipped in 31m15s, from 186 / 25 /
4 / 29 this morning. @concern:doorway-failover 10/10 incl. the NEW story
"A blob rides through its primary's bad hour" (SIGSTOP the primary, trip
the breaker with a burst, Range-read past the pantry, bytes arrive from a
sibling; doorway_blob_target_failover_total 0->2 live). CURE: select_route
(server/http.rs:115) walks the declared priority order for /blob/ paths and
takes the first endpoint whose breaker would_shed()==false; all-shedding
falls back to the declared primary; non-blob routes keep matches.first()
(projections never float). 6 red-first unit tests; seam-registry row C7.
Gate green under rustc 1.98 after clearing result_large_err x3 +
items_after_test_module (toolchain drift, untouched files). TWO DEFECTS THE
PROOF SURFACED, both fixture-side and both cured: (1) the landing SSR
bundle (71,763,974 B, now carrying source maps) crossed RS_THRESHOLD and
PUT /blob panicked storage at http.rs:2617 (rs-4-7 manifest hand-sliced as
raw chunks; the RS band NEVER worked through PUT) — cured 8854f6de5:
shards come from create_shards, hash/count mismatch is a hard 500 with
nothing stored, reads reconstruct through parity (>=4 of 7), 4 red-first
integration tests; live: 201 on all three peers, byte-identical GET; a2o
">64 MiB artifact accepted whole, served whole" added to blob-durability.
(2) hc-mesh.sh since 123cea498 severed DOORWAY_ID/DOORWAY_HEALTH_PORT from
doorway A's launch (a comment block ended the continued assignment list)
— A booted with a random doorway_id, matched ZERO project-epr rows, / 503
and /lamad 404; the 14:30Z lane showed it as 13 cascading reds (38 failed);
cured 3a77a458b with a rail comment. Also: seed-forward budget now scales
with size (596a1c928); beacon shared lanes repeatable (Codex, 906d7b159);
MESH_TRANSPORT_BACKEND knob (Codex, f86dd32d5; three-mode boot proof
pending). NAMED RED, lines, not yet cured: (a) routes/health.rs:377-378
computes serving.shedding/degrading with any() over ALL upstreams, so an
open circuit on a POOL peer (8091, errorStreak 3 after a drill) demotes
doorway A to degraded/shedding while its primary is closed — the two
cross-doorway-content "alpha degraded" reds and a sibling-classification
lie (the invariant says siblings are honestly classifiable); (b) nothing
ever trials a pool peer's open circuit (warm-up skips open, select_route
skips open, projection fallback fires only on a primary miss) so it stays
open until the next boot — needs a periodic guarded half-open probe.
Still red, classified: identity join 3 (household-formation 1,
conductor-spin 2) · self-healing-flow-control 5 · delivery-diagnostics 3 ·
epr-cross-peer 2 · chaos-peer-churn 3 (custody precondition — backlog
seed-custody-coverage-for-drill-content) · web2 cache 1 · conductor-visibility 1
· content-sync 1 + peer-mesh 1 (caughtUp under load) · stewardship-allocation 1
· "same truth" 2 (ch10 stewardingCollectives alpha-A=1 vs elohim.host=2;
footprint convergence) — green this morning, red in both runs today;
not localized (the storage rows themselves read None on all peers).
Status stays RED: flip authority is the fleet lane
([build:edge] [edge:validate-only]). Roadmap:
genesis/docs/superpowers/plans/2026-08-23-doorway-federated-continuity-roadmap.md.
DELTA 2026-08-22 (household lane, run 20260822T170136Z-519d4f6b, sut
sha256:2c012553c2e79dfb, 3-peer mesh + two local doorways): @concern:doorway-failover
passed=9 failed=0 — the first MEASURED green for this concern since the cures
above landed. Status stays red on purpose: this habit names the edge Dataplane
Validation lane as its flip authority, and the fleet has not run since #1376
(11:08Z, 76/80 skips). Confirm with `[build:edge] [edge:validate-only]` — the
cluster is quiesced (matthew caughtUp, both doorways 200 on the canonical head),
and a bare [build:edge] would throw that away to measure it.

DELTA 2026-08-21c (freshness verdict + amber pantry landed host-green,
commits 5da4c3b0e + ccf0138a9): status stays red — the a2o scenario is
the flip authority.
DELTA 2026-08-21b (CORRECTION to this habit's own 2026-08-18 evidence,
and the amplifier of the 503 above): the entry below claims "Task 3.4
warm-boot shell cache LANDED desk-proven ... / is cache-first ... both
local doorways shed in 1-4ms (was 10-20s class)". The local proof was
real; the DEPLOYED claim was false, for the whole of that cure's life.
main.rs builds AppState through with_pool (343) or with_services (345),
and BOTH set warm_shell: WarmShellStore::inert() because no archive
exists at construction; the only constructor that ever built a live
store, with_projection, has ZERO production callers. The archive arrives
later in init_projection, which installed app_file_cache and never
rebuilt the store. An inert store's lookup_with_declared returns Cold
BEFORE it consults the hot map, so stock() writes are unreadable,
hydrate() returns 0 unconditionally (the boot log's "hydrated: 0" reads
as a cold archive, not a disabled one), decide_shell_serve(Cold, true) =
Fetch, and `/` paid a full EPR_DISPATCH_TIMEOUT_SECS fetch on EVERY
request plus a second one through the ProjectedEpr fallback. That is the
20.751s `/` measured on 2026-08-21, and those paired 10s failures are
what opened the breaker in the delta below. The suite had NINE tests for
this feature and every one built a store WITH an archive — nothing tested
the shape production actually used. Cured: init_projection now calls a
named bind_warm_shell_to_archive() (named, because the invariant is that
every archive-installing path must rebuild the store), with rails
stocking_an_inert_store_still_serves_nothing,
an_inert_store_hydrates_nothing, and
binding_the_archive_lights_the_warm_shell_and_the_invariant_holds.
`just gate doorway` exit 0. THE GENERAL LESSON, which is why this
correction is written here rather than only in the commit: a local proof
of a CACHE says nothing about whether production CONSTRUCTS that cache.
"desk-proven" is not "wired".
DELTA 2026-08-21 (STAYS RED — cause NAMED and four cures landed
host-green; the a2o @concern:doorway-failover scenario is still the flip
authority and has NOT run): the apex 503 was never the projector and
never adam. Four links, each measured. (1) STORAGE's /apps extraction
flight leaked a herd: a waiter whose post-wait cache re-check MISSED fell
through and extracted WITHOUT ever registering in in_flight, then created
a guard that broadcast finish_extraction for a flight it never held. So
when the first extractor's put_app failed, every waiter missed and all
became simultaneous extractors, and their concurrent put_app calls raced
evict_app's remove_dir_all against each other's directory writes ->
"Failed to cache extraction (non-fatal) … Directory not empty (os error
39)", logged TWICE 8ms apart, identifier=elohim-host-landing, 2026-08-20
20:11:27Z. put_app clears the index entry BEFORE writing and restores it
only on success, so each failure left the app permanently uncached and
every later request re-extracted the whole bundle — self-sustaining.
(2) The SSR shell fetch rides EPR_DISPATCH_TIMEOUT_SECS=10, SHORTER than
the proxy's STORAGE_PROXY_REQUEST_TIMEOUT_SECS=12, so the homepage render
trips the breaker first; three timeouts in ~25s = the threshold.
(3) The breaker is keyed by ENDPOINT, not route, so it then sheds EVERY
route on that peer — /db/content answered in 40-114ms throughout and was
shed as collateral. Two sites also recorded a Failure for an upstream
429/503, against ProxyOutcome::classify's explicit rule that answered
backpressure proves liveness. (4) UpstreamBreakers::is_open() is a GATE,
not a read — it advances Open->HalfOpen and CONSUMES the one half-open
trial — and two `/`-path planners called it with no BreakerTrial guard,
so no outcome was ever recorded and the circuit re-latched every
STALE_HALFOPEN_COOLDOWN_MULTIPLIER x cooldown = 120s. Fixed 2026-07-21
(f5e22baa2), REINTRODUCED 2026-08-18 (f0b908660, warm-boot shell cache),
whose in-tree comment asserted "is_open only READS the circuit" while
sitting directly on the bug it named.
LIVE PROOF of the shape, 2026-08-21 11:59Z: elohim.host / -> 503 in
20.751s carrying x-ssr-skipped:shell-fetch-failed and
x-elohim-hop-serve-ms:10004.681 — the 10s budget expiring to the
millisecond, twice sequentially — while its OWN
/db/content/elohim-host-landing -> 200 in 0.114s and its /health still
read healthy:true,status:"online". doorway-alpha in the same minute sat
circuit:closed/errorStreak:2 (the blind window) with /db -> 503 after
12.034s. BILATERAL, never an adam defect: breaker_open_total climbed
A 56->303 and B 47->295 over 13h, and doorway_upstream_backpressure_
honored_total reads 0 on BOTH — the opens are never-answered hops, not
storage backpressure.
CURES (host-green, NOT deployed): the extraction flight re-enters instead
of falling through and only its owner guards it
(MAX_EXTRACTION_COALESCE_ROUNDS); evict_app deletes the prefix
unconditionally so a failed put cannot leave an orphan the next put
inherits; both `/`-path planners use a new non-mutating would_shed() and
the raw gate is #[cfg(test)] so production cannot reach it a fourth time;
both classify violations defer to the one classifier; snapshot() stops
reporting a HalfOpen circuit as skipped:false; /health gains a `serving`
block and demotes on BOTH the shedding and the slow regime (errorStreak>0
— the blind window above is exactly where a shedding-only signal lies),
and /health/serving carries the status code /health cannot, because
/health on :8080 is simultaneously the startup, readiness AND liveness
probe and must never flip. Gates: `just gate doorway` exit 0,
`just gate elohim-storage` exit 0, elohim-cache-core 47/47, 6 new breaker
tests + 5 new health tests + 2 new cache tests, all red-first.
OPEN: fleet verification via [build:edge] [edge:validate-only], and
whether the endpoint-keyed breaker should be route-class aware — backlog
doorway-breaker-trial-theft-fleet-verification.md.
DELTA 2026-08-18c (batch closed on dev — pre-push review pass, 57688ae4a):
both cures below were hardened before landing, red-first — warm_shell keys
the stocking on the head declared when the fetch was DECIDED (a projection
advance mid-fetch can no longer relabel old-era bytes AtHead), and
custody_rotation's successor check is state-aware with an idempotent author
(a create-succeeded/activate-failed row now converges instead of stranding
the pledge into invisibility). Gates green; a2o scenarios remain the flip
authority, unchanged.
DELTA 2026-08-18 (ch07+failover session): Task 3.4 warm-boot shell cache
LANDED desk-proven (doorway gate green: 943 lib/bin tests, clippy, fmt) —
/ is cache-first (ServeWarm / UpgradeThenWarm-2s / Fetch / instant-Shed
decision matrix in render/warm_shell.rs, x-elohim-bundle:last-reconciled
marker, boot hydration from app_file_cache, mongo-less degrade = today's
path, 9 new tests incl x-ssr-fetches-free warm serve). Kills the live
defect measured this session: / stalled 10s (A, 200 shell-fetch-failed)
/ 20s (B, 503) on EVERY request through catch-up — the suite's
HeadersTimeoutError. Local-mesh proof: both local doorways shed in 1-4ms
(was 10-20s class) and the @concern suite runs 0.18s with the
classification scenario green. Also: E2E_DOORWAY_BETA/_B/_STORAGE_URL_B
now reach the cucumber env (reconcile-inventory's beta-leg red was env
plumbing); Task 3.2 apex-build client fallbacks configured
(environment.prod.ts). Task 3.1 apex multi-A attempted and REVERTED per
the WS3 revision — new hard evidence in the plan: relay-addr-beacon's
shared-record lane is single-slot (clap last-value-wins), so apex
multi-A needs a beacon change or sacrifices doorways.elohim.host;
operator menu item 2 now carries that constraint. Sibling saga ch07
(custody-witnessed): rotation cure landed (elohim-storage
services/custody_rotation.rs, gate green 2772 tests) and PROVEN on the
local 3-peer mesh end-to-end through public surfaces: stale pledge ->
rotation tick 150s -> notarized+ACTIVATED successor (origin:rotation,
dht-anchored) -> predecessor superseded -> self-held evidence ->
elohim_custody_class_count{class="stocked"} 0->1. Both cures reach alpha
on the next edge deploy; the a2o scenarios remain the flip authority.
Prior evidence below.

RED WRITTEN 2026-07-31 (doorway-federation-failover sprint session,
live-probed in-session): elohim.host / -> 503 catching-up shed (adam
post-deploy arc-convergence window) while doorway-alpha / -> 200. The
pair floor and honest-classification scenarios hold; the apex-name
scenario is the born-red — the name is pinned to doorway-B, so B's
hours-long catch-up sheds the apex while a healthy sibling holds the
identical converged content. Cure surface named by the sprint plan:
apex multi-A + client fallback (WS3), warm-boot shell cache
(x-ssr-fetches:0 invariant), and the operator ceiling on adam's
hosted-agent provisioning.
