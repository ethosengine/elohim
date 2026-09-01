---
epr-habit-version: 1
id: notary-authority
invariant: >
  Converged state can be notarized: green tier is reachable for any content
  (HEAD-DAG declare_content_head, reach re-certified at republish, author
  signatures). Authority answers come from the notary, never from LWW order.
status: green
# Stood down from active 2026-08-18: it went GREEN on edge #1362 (3p/0f on
# the deployed fleet) and a green habit holding a WIP slot is exactly what
# the max-2 fence exists to prevent. Its named watch items (doorway-failover
# regression, the A-converged gauge reading 0.0 through a passing
# convergence scenario) live on as evidence below and as their own habits —
# not as a reason to keep this one in flight.
active: false
checks:
  - "a2o @concern:notary-authority (genesis/a2o/features/dataplane/notary-authority.feature — runs in the edge Dataplane Validation tag set, no filter change needed)"
  - "sweettest rea_commitment_replication (elohim/holochain/tests/sweettest — unignored 837b772c9; isolated late-join conductors, bounded 60s peer-B retrieval; RED but MIS-ATTRIBUTED per DELTA 2026-08-09 — it measures one-time cold-cell Wasm warm-up, not REA retrieval; do not read it as a fetch-seam probe until it warms peer B)"
refs:
  - "genesis/docs/superpowers/specs/2026-07-01-crdt-authoritative-content-state-dht-notary-decouple-design.md"
  - "genesis/docs/superpowers/plans/2026-07-01-crdt-content-dataplane-full1c-implementation-plan.md"
retire-when: >
  when notarization is a PRECONDITION of serving rather than a tier above it — content
  that cannot be notarized cannot be served — at which point "green tier is reachable"
  stops being a separate question anyone can answer wrongly.
---
DELTA 2026-09-01 (bootstrap-steward honest absence, coordinator-only): null progenitor properties now yield `None`/`false` while malformed configured keys still fail closed; content_store 70/70 + sibling coordinator suites 24/24, 55/55, 0/0 + `absent_bootstrap_steward_refuses_earned_declaration_cleanly` sweettest PASS on a freshly packed learning DNA (non-author cleanly refused, no Deserialize leakage); household mesh was down, so no live CLI receipt claimed.
ACT I LANE 2026-08-21 (evening, fresh mesh + corrected Prologue, stock conductor): saga in order 17 pass / 3 fail (ch01 = doorway post-restart warm-up; ch09/ch10 = co-steward cast is an Act II persona); Act I inventory 363 eligible -> 106 pass / 55 fail / 159 undefined (placeholders; @wip-swept) / 35 pending. Largest real red class: the doorway's 2-failure breaker opening on a ~60s storage stall under suite load. Layering landed: @act: resolution, just test mesh, tag plan over 167 features, owned-substrate declared FALSE on the live lane (it was failing open). Status stays RED: the fleet scenario is still the flip authority.
LOCAL PAIR PROOF 2026-08-21 (just mesh, mongod-backed, doorway built from ccf0138a9): @concern:doorway-failover + saga ch04 = 8/8 scenarios, 31/31 steps on the local A/B pair (baseline on the pre-fix doorway: 5/8, the three freshness scenarios red with no x-elohim-freshness header). Same-process shed drill on doorway A (pid pinned): knowledge read green -> matthew storage killed -> 200 amber with stocked-at/served-head/no-store on two consecutive reads (circuit open, no 503) -> head-record 503 + x-elohim-freshness-required: green -> storage restored -> amber through the ~25s cooldown -> green at t+30s, breaker closed; verdict counters 3 serve / 7 serve_amber / 1 shed. Cold-cache herd: 40 concurrent /apps reads straight at storage = exactly 1 extraction, 0 'Directory not empty'. Warm shell: archive_backed=true, hydrated=1, / at 0.9ms with x-elohim-bundle: last-reconciled while storage was dead. Status stays RED: fleet evidence is [build:edge] [edge:validate-only].
DELTA 2026-08-18 (GREEN — edge #1362 SUCCESS): notary-authority 3p/0f
on the deployed fleet — notarized-tier serve, cross-peer canonical-head
convergence, and the earned-authority refusal all passed for the first
time. What flipped it was never the identity plane: the conductor's
post-outage DHT storm was starving storage reads in the shared cgroup
(cured: conductor spawns nice-10, 3146ebdc5) and the probes had no
shed-awareness (cured: bounded catching-up ride, review-hardened
3b1d13322). Same run: saga 10/11 green (ch02/03/05/10 recovered; only
ch07 custody-witnessed red — elohim_custody_class_count{class=stocked}
=0 on A, a real custody gap, the next frontier), quiesce
sustained-PASS 362s. Declared-stakes grants live on matthew+jessica
(genesis seeded stakes:genesis-lamad simulacra; adam failed the PUT —
retries on the next seed run). Watch: doorway-failover regressed
4p->1p/3f alongside B-caughtUp flapping; A-converged gauge still reads
0.0 through a run whose convergence SCENARIO passes — gauge probe
suspect, not the dataplane.
DELTA 2026-08-17d (ch02 red root-caused to CPU-starved storage, shed cured
at the substrate, local-proven): ch02 household-forms leaves the A-side
identity cluster too — the rows were CORRECT live (all three
agentPubKey set, householdId=household-dowell, probed 19:5xZ); #1360's
red was doorway-A breaker OPEN 18:39-18:50+ (storage forward failed
connect/timeout, breaker_open_total 0->29 at 18:40, backpressure_honored
0) while elohim-node sat at its CPU limit with 100% CFS-throttled
periods for hours (still pinned 20:40Z) under a kitsune2 single-op
fetch storm (relay peer 31875c13, ~320 log lines/s, conductor sqlite
read pool Util 750-1700%). caughtUp flap = same starvation seen from
reconcile. Substrate cure: conductor child now spawns CPU-deprioritized
(nice 10, ELOHIM_CONDUCTOR_NICE, process_manager.rs) so storage's read
handlers outweigh conductor churn in the shared cgroup — local mesh A/B
(storage pinned 1 core, 256-way contention): /db/humans p90 349ms
max 536ms at nice-0 vs p90 23ms at nice-10, near-baseline; spawn
niceness pinned by /proc test. ch02 humans-row steps also ride the
catching-up shed (getRawRidingCatchUp) like ch03/ch05 with 165s
ceilings. With ch02 reclassified, the identity cluster's saga evidence
is ch07 alone; the A-side reds' common cause is the fetch-storm
saturation — next fleet lever is bounding that storm (fork: single-op
fetch batching + kitsune2 debug-log volume), not identity repair.
Deploy proof wanted: conductor pid at nice 10 on matthew, breaker
stays closed through the next post-churn Dataplane run, ch02 green.
DELTA 2026-08-17c (saga ch05 red — same shed class as ch03, desk-proven):
edge #1360's ch05 co-steward-agreement red (Station A, 2p/1f) was also
NOT the A-side identity cluster — pin #433 (epr:elohim-host-landing,
active/item) and its pull rollup (caughtUp:true) held live; the red was
the catching-up admission shed 503 on the chain's ONE unretried read
(GET /api/v1/pins — "single read, not a poll" was correct for the pin
claim, wrong for surface availability). The pin step now rides the shed
via the same getRawRidingCatchUp primitive (shed-only; any other non-200
still fails on first read; step ceiling 105s over the 90s ride bound).
Live saga-profile run 3/3 PASS 2026-08-17. With ch03+ch05 reclassified,
the A-side identity cluster's remaining saga evidence is ch02/ch07 —
the identity-coherence reds stand on their own probes, not on these.
DELTA 2026-08-17b (saga ch03 red mis-attributed; measure hardened, desk-proven):
edge #1360's ch03 eprfs-upload red was NOT the A-side identity cluster —
the chapter's own invariants held live (blobHash attached, served head ==
declared sha256-7ce8…ec93 status=current); the red was the mid-run
catching-up admission shed (503+retryAfter while /health stays healthy —
runbook class, quiesce gate only covers stage START). probeContent now
rides the shed bounded 90s honoring retryAfter (getRawRidingCatchUp:
only the shed body signature is ridden; 200/404/plain-503/connect-error
still fail on first read; deadline expiry returns the shed 503 honestly).
Unit-pinned (7 new tests, 187/187 suite), gate green, live ch03 run 3/3
PASS. ch03 (and its step reuse in federation-deploy/served-projected-head
A-baselines) leaves the A-side identity cluster; the identity-coherence
reds stand on their own evidence. Next banked edge run is the flip proof.
DELTA 2026-08-17 (heal cure DEPLOYED-PROVEN, edge #1360): the proof asked
for below arrived — matthew, restored after a 13h node outage (ethosengine
NFS-hardmount wedge, operator-fixed), drained its divergence backlog
12->0 actionable in ~15min under the new budget/AIMD path;
fleet-quiesce sustained-PASS 368s, deploy-churn->quiesce ~27min. Habit
stays RED on its own legs: notary-authority 0p/3f, all A-side —
blobHash null / trust not notarized / heads unconverged / household
agentPubKey missing-or-fossil post-restore — and A-converged=0.0
throughout the PASS window: SECOND reproduction of the 2026-08-13
gate-predicate gap (gate anchors on exhaustion-quiet, not convergence).
Next lever is convergence + A-side identity coherence, not heal
throughput. DELTA 2026-08-16 (head-batch budget starvation cured, integrator batch):
live fleet reads showed matthew healing 0/hr against a 2,510-row backlog
— unattempted 2,447/hr all budget_exhausted, batch pinned at floor 8:
the 4s BATCH_EXTERN_BUDGET admitted id #0 (a saturated cascading()
read-permit acquire costs ~10s), blew the whole budget, refused the rest,
while the AIMD controller read each instantly-admitted partial-Ok as
headroom. Two-part cure in elohim-storage: observe_batch_outcome routes
a refused (unattempted) tail down the decrease path, and the extern
budget rises 4s→12s (above one permit-acquire, below the coordinator's
15s ceiling) with attempt_timeout 15s→25s to outlast budget + one
in-flight id's overshoot. Pinned by
a_budget_starved_batch_shrinks_even_with_instant_admission +
the_extern_budget_outlasts_a_saturated_read_permit_acquire; 2743 lib
tests green. Proof wanted next deploy: healed/hr > 0 on matthew,
unattempted{budget_exhausted} collapsing, head_batch_size lifting off
the floor. DELTA 2026-08-15b (admission relief deployed edge #1353 ~16:15Z, shift
admission-throughput-to-banked-ch06): both demand-side levers live —
sweep declares ride Background class w/ shed-aware routing (bccdc643b;
also cures a shed→Author competing-root mint), ghost-witness election
probes batched through the landed L1 extern (aff6c78bb). Mechanism
VERIFIED on matthew ~1h post-restart: interactive lane CLEAR (5 sheds
total vs 106/7h baseline), background 169 acquired ≈all <=1ms + 30 cheap
1s sheds, AIMD election batch 94 (floor 8, no two-arm collapse). ch06
recording still HELD: post-restart conductor catch-up in progress
(head-batch call_failed ~45%, deadline class — conductor-side, not gate),
healedTotal flat at 68 through ~17:49Z; verdict runbook in shift sprint
result. Two CI wiring reds found+fixed in-flight (edge #1351
MethodTooLargeException 5c7bd3cbf; #1352 sdk-gate exit-127 → Docker
sdk-check target 5af289846); SDK feature-matrix gate now exercises green
in CI.
DELTA 2026-08-15 (SPIN discharge deployed edge #1350, overnight verified):
the flip evidence wanted below did NOT arrive — known_divergent{content}
on matthew/jessica/james oscillates 2<->13 (outstanding-set window, no
restart), NOT a held drain. Id-level attribution (106 failure lines, all
distinct ids): the arm works as designed but is THROUGHPUT-BOUND by the
fleet-wide conductor admission ceiling (content_store class=interactive
capacity=5=max(2*cpus,8)-3 on household pods; 5s shed + conductor ws
timeouts, verbatim-identical on all three pods). Small terminal subset:
3 real-seed ids peer-confirmed no_record (ghost candidates) + 18 e2e-*
fixtures. Ranked levers + full evidence:
genesis/data/timeline/backlog/conductor-slow-batch-starvation-jessica-class.md.
ch06 recording stays HELD (converged flaps 0 during churn).
DELTA 2026-08-14 (SPIN discharge landed, local gates green): the A-side
blocker named in the deltas below now has its canonical channel — the
undeclared two-way root conflict supplies the DHT election by symmetric
self-candidacy (contest_divergent; Refreshed-site admission; decide arm
ContestDivergent; default ON, kill-switch CONTEST_UNDECLARED_DIVERGENCE;
C3 liveness witness proves flag-OFF spins and flag-ON discharges;
2,671 tests green). Peer-head candidacy was proven impossible by the
hint-inflation guard, which is why self-candidacy is the shape. Flip
evidence wanted next: post-deploy known_divergent{content} on
matthew/jessica/james drains to <=2 and HOLDS across a pod restart, then
a banked validate-only run flips ch06.
DELTA 2026-08-14 (leg 2, run #1348 banked — SAGA 5/11 -> 6/11): ch10
card-tells-truth flipped regressed -> GREEN and ch04 HELD green across
two consecutive banked runs, so the doorway cure is stable, not a
one-run artifact. Same wave shipped the storage pprof endpoint and
Pyroscope now ingests elohim-alpha/elohim-node with symbolized Rust
frames — the profiler's eyes are open on the dataplane for the first
time. First finding: susan's STORAGE burns 3.3x matthew's CPU (44.86s
vs 13.65s / 25min), dominated by P2PNode::run 58% ->
handle_behaviour_event 49% -> sqlite3VdbeExec 38%, where matthew shows
no P2PNode dominance at all. That forces a correction to the
2026-08-11 host-placement pass: 'the conductors themselves are the
load' was read off CONTAINER CPU, but conductor and storage share one
cgroup, so storage's own P2P path was never separated out. The
host-placement partition evidence itself is unaffected. The four still-
regressed chapters need A-side convergence, which is gated on the SPIN
canonical-channel design decision (above the iteration ceiling).
DELTA 2026-08-14 (leg 2, run #1347 banked): ch04 doorway-serves flipped
RED -> GREEN — the f125282a8 pooled-client cure is now proven in a
BANKED measure, not just a live probe; ch05 co-steward-agreement
regressed in exchange, so the saga holds 5/11. The A-side blocker is
now NAMED rather than counted: the ethosengine pods' divergent content
rows are the SPIN class — anchor-divergent but UNDECLARED, and both
existing levers are structurally unreachable for them (ContestPeer
admission requires local_declared; ghost-decay requires an observed
Answer::Absent), so sweeps mint Refreshed no-ops forever. Stuck-stable,
NOT self-healing: the lever is a canonical channel, which is a design
decision in this habit's own domain (heads move via canonical channels
only) — backlog/spin-divergent-undeclared-rows-block-a-convergence.md.
Do NOT tune sweep cadence/retries/dormancy at it. Overnight the count
fell 13/13/13 -> 2/1/14 across a pod-restart cluster, but MissLedger is
in-memory so each restart re-climbs from 0 — a post-fix drain only
counts if it survives a restart.
DELTA 2026-08-14 (run #1345 banked, post ethosengine thermal crash +
cure deploy #1344): notary-authority 0p/3f — all three legs red on
the CRASH-RECOVERED A side, which never converged during the quiesce
window (A-converged=0.0 throughout; the gate anchored on actionable=0,
an exhaustion-quiet, not convergence — gate-predicate gap worth
naming). Same run banked ch11 first-green + recorded honest crash
regressions on ch02/03/07/10 (matthew-side member rows, served-head/
blob coherence, shard custody, stewarding counts). The ch04 doorway
code cure (f125282a8, one pooled client per storage upstream — the
SSR client's doomed fetches were poisoning the shared breaker) is
live-proven (both doorways GET / 200 x-ssr-rendered:1 at 22:52Z);
its chapter re-greens when the A-side substrate re-converges.
DELTA 2026-08-11 (decay-reach measured, edge #1341 deployed): the
overnight soak's open question — why susan/eve/gertrude authored ZERO
decays in 17h while holding the largest phantom cohorts — is answered,
and BOTH standing hypotheses are refuted. New meter
elohim_content_ghost_decay_blocked_total{leg} (the refusal twin of
decay_author_total; 5 pre-touched legs) reads, ~2.5h post-restart:
leg="disabled" is 0 on ALL SEVEN pods, so the flag is live fleet-wide —
it is NOT a missing-flag/deploy problem. Nor is the predicate
evidence-starved: it is barely CONSULTED on the shem side. Total
considerations vs known_divergent{content} held: matthew 169/11,
jessica 28/0, james 12/0, susan 2/1660, gertrude 1/597, eve 0/647,
adam 0/27. So the decay arm is reached ~zero times on precisely the pods
holding the phantom stock; the bottleneck is UPSTREAM of the predicate
(the sweep never brings those rows to a Hold/ContestPeer pre-flight),
not inside it. Second finding: where it IS consulted, the dominant
refusal is local_not_observed_absent (matthew 109 of 169) — the C4
positive-absence leg, which LocalResolve::Probe callers can never
satisfy by design (only the ghost-witness caller passes
Resolved(Absent)). STATUS STAYS RED: unchanged flip condition (one edge
run whose quiesce gate records 3/3); this delta re-aims the next
lever from "decay tuning" to sweep REACH on the shem pods.
DELTA 2026-08-10 (batch-3 root cause, evening): validate-only run
#1339 came 28s from banking — gate anchored PASS at count=2,
sustained 302/330s, reset by matthew's actionable jumping 2→11 on
the next sweep. Live diagnosis re-framed the batch-3 residual: the
"~2000 unanchored rows" are 2028 ANCHORED rows (1987 reach=familiar)
whose declared heads are PHANTOMS — they outlived every conductor
incarnation, so contest fails no_local_chain (130 on matthew),
record fetches degrade budget_elapsed 202/202 on adam (zome
get_record_for_action used a network get that searches the fleet for
bytes that exist nowhere), elections are invisible (no_election 279),
adopt-before-author (already ENABLED fleet-wide) starves byte-less,
and ghost-witness sweeps run all-held forever. Cure landed this
branch: (1) get_record_for_action -> GetOptions::local() (honest
ms no-record answers; coordinator hot-swap, no DNA-hash move);
(2) ghost-declaration decay (ELOHIM_GHOST_DECLARATION_DECAY,
operator-reserved, alpha-enabled via the adopt-before-author
placeholder) — Hold/Contest downgrades to Author only on POSITIVE
double falsification (own conductor observed empty + advertiser
stated no-record within the evidence-absent window, no local
election); metric elohim_content_ghost_decay_author_total. Flip
condition unchanged: deploy, watch decay-author + witness authored
go nonzero and actionable collapse to 0-2 band, then bank via
validate-only. DELTA 2026-08-10 (wedge audit, midday): the flip path was
MIS-MODELED — the fleet-quiesce gate's predicate reads storage-A
(matthew) ONLY (caughtUp + actionable<=tol 2 + unmeasured=0 + both
doorways 200; scripts/ci/fleet-quiesce-gate.sh scope note), so the
shem-trio stock never gated banking; and the banking verb is a
NO-DEPLOY validate run — empty commit tagged [build:edge]
[edge:validate-only] (mode live since b2dfd0de2, 2026-07-30) —
because a deploy-coupled [build:edge] banking run restarts the
fleet it measures. Four-leg preflight (doorway /p2p/status +
/db/content x2 + per-pod Prometheus actionable/unmeasured, ~20s)
read ALL GREEN with lanes idle post edge #1338; the banking push
is prepared and classifier-blocked pending the operator permission
rule (sprint wishlist item 1). Preflight-before-push is the
discipline: never fire a banking run the four-leg read wouldn't
pass. DELTA 2026-08-10 (limit-cycle sprint, overnight): DONE-CANDIDATE —
@concern:notary-authority 3/3 PASSED locally TWICE across two
independent fleet deploys (post-declare 13.4s run, then 0.3s re-run
after edge #1337's full restart: same canonical head on both
doorways, notarized tier serving, authority refusal holding). Cure
chain: heal-leg break fix + outcome visibility (b96861c1b+31f8a9e89,
99% of actionable divergence sat on three pods whose heal leg broke
silently on the first failed batch call), CPU/k2Gossip relief for
the shem trio (f43e23aa5, 61118ace9), trust-gradient adopt
(639ef94e6), and the authorHeadOnce declare cycle (feb0ceb0a) that
busted the elohim.host ghost anchor (uhCkkDYpMV… superseded by the
elected uhCkkPqUQj…). Actives drained to single digits
(james+jessica reached known_divergent=0). STATUS STAYS RED on the
strict rule: the named check is the EDGE Dataplane Validation
measure and the banking runs have not yet recorded 3/3 there —
#1335 measured 1/3 PRE-declare, #1336 was double-fire-aborted,
#1337 SUCCESS but its quiesce gate skipped the suite (no-measure).
Flip condition: one edge run whose gate completes recording 3/3.
Residuals on the board: shem-trio conductors still CPU-pinned
(k2Gossip profile deployed, arc-to-full unconfirmed — relay-path
gossip rounds still time out fleet-wide, round-2 investigation
seeded); matthew converged=0 on failed/pending terms + ~10
known-divergent; seed-pod unanchored backlog (~2000 rows, H3
confirmed via /db/stats parity) is the named batch-3 cure. Prior
delta preserved: DELTA 2026-08-09 PM (convergence-serve-path shift
close, edge #1327-#1332): still red BUT the blocking seam is CURED
five layers deep — serve 503s, authority-before-shed (storage ab316cad7 +
doorway 85a128997, both test-pinned), gate-banner honesty,
node-local-dns hairpin (iroh's resolver ignores /etc/hosts), and
the cross-relay preflight fail-closed defect (fork e4a1c9bb2,
vendored kitsune2_transport_iroh patch — the overloaded error
string hid it through three investigations). Post-#1332 deploy:
BOTH relay error classes at zero fleet-wide (553K-line scanned
negatives). CORRECTED same day: the initial "divergentAnchor
falling" read was a sampling artifact (pool-fanned health endpoint)
— 6h of per-pod Prometheus shows content divergence OSCILLATING in
bounded bands with no drain (the content-gap limit cycle, evidence
in its backlog item). Convergence, caughtUp, the shed lift, and
this habit's measure are all gated on the content-gap plateau
objective landing — not on time. Retrigger [build:edge] only after
that objective moves the cycle. The REA
sweettest check is GREEN locally after the oracle warm-up fix
(a183c1a01) — see next delta for the falsification.
DELTA 2026-08-09 (dev@b3c58d9ca, measured locally): the REA leg of this
node is MIS-ATTRIBUTED and must stop being read as a fetch red. Direct
instrumentation shows REA replication is HEALTHY — ops integrate on both
cells 0.5-2.0s after peer exchange and peer B answers get_rea_commitment
in 12ms. The 60s budget is spent BEFORE the read: peer B's first zome
call costs 94.37s, and Alice pays 100.97s on HER first call while
bootstrap-disabled with no peers at all (6.9ms on her second) — a
one-time, per-cell, purely LOCAL Wasm instantiation of the 13.4MB
content_store.wasm, which cannot be a network cost. The green sibling
lamad.rs::resolve_content_head_local_is_nonblocking_and_converges is
structurally identical and passes only because it warms B's runtime
first, with a comment naming exactly this. The oracle was NOT edited (it
is this check); making it measure retrieval is a habit-owner decision.
One real substrate finding stands, unproven as the alpha cause: the same
reconcile sweep heals content via resolve_content_heads_local (Local)
and REA via get_rea_commitment (Network) — projection_reconcile.rs:447
vs :2389. A local-first zome variant was built, packed, measured and
deliberately REVERTED: it does not fix this red, no test reproduces the
empty-arc condition it targets, and its Err->Ok(None) fall-through would
re-define the conductor_missing counter the live diagnosis is reading.
Full evidence + next legal moves in backlog
genesis-pair-cross-conductor-fetch-blocks-canonical-convergence.md.
Node stays RED on the a2o scenarios, which are the authoritative probe.
DELTA 2026-08-09 (edge #1327): MEASURED red — the cucumber suite ran
(64 scenarios, 37 passed) and @concern:notary-authority went 0
passed / 3 failed, all three on alpha-A: GET /db/content 503
{"status":"catching-up"} (cannot serve the notarized view; no
canonical head comparable), and the non-author HEAD-move arm got 503
instead of a 401/403 authority refusal. Caveat for the next reader:
the same build's fleet-quiesce gate declared FLEET QUIESCENT with
B-caughtUp=False throughout (backlog
fleet-quiesce-pass-not-convergence.md) — the gate's PASS is not
convergence; the scenario's 503s are the authoritative probe.
DELTA 2026-08-08 (edge #1326): no-measure — the fleet-quiesce gate
exceeded its 2700s deadline (node B never caughtUp; fleet churning), so
the Dataplane Validation cucumber suite was skipped entirely and the
@concern:notary-authority scenario did not execute. Red stands
unmeasured, not re-confirmed. Next measurable window: the next edge
deploy whose quiesce gate completes (the T5-flip batch is queued).
DELTA 2026-07-24 (convergence-day, dev@b83447bf7): the REA face of this
red is now runnable AND metric-visible — adam heal outcome missing:372
with rea local_total:0 (heal pacing cured starvation, exposing that B
cannot RETRIEVE A-authored REA entries at all); sweettest reproduces it
isolated (0 passed 1 failed, B cannot fetch A's commitment in 60s);
backlog genesis-pair-cross-conductor-fetch REOPENED with the full
Prometheus/Loki/diagnostics bundle. Asymmetry judgment: same Lamad cell
both streams — content's existing-row admission masks what REA's
remote-advertised IdToCommitment lookups expose; leading seam is
Matthew→Adam link-op/target integration or fetch. Prior delta below.
DELTA 2026-07-12 (shift convergence-bank, dev@bcc023322): FIRST FULL
3/3 GREEN — edge #1187 Dataplane Validation passed ALL notary-authority
scenarios including scenario 2 (seam-smoke: "landing canonical head
CONVERGED"); adam adopted the declaration after finishing its
post-restart heal-backlog catch-up. The convergence-test sweettest pair
is BANKED green x2 fresh triggers (DNA #1359/#1360) under isolated
conductors (bcc023322 — the pre-exchange partition the tests assume is
now real; mem-bootstrap thread-keying defect class closed for the whole
family). Edge #1188 (the x2 attempt) regressed scenario 2 — measured
DURING its own post-deploy churn: elohim.host caughtUp=false and
serving its legacy root (f41d blob) hours into catch-up, live-probed
08:3x UTC. The binding constraint is HEAL THROUGHPUT (~10s/row x
thousands post-restart), not election/declaration/fetch — all proven.
Node stays red on the strict x2 rule; flip condition = edge validation
on a settled fleet (elohim.host caughtUp=true), which the next
app-deploy declare + heal completion produces without code change.
Seam-smoke --gate flip HELD until validation is churn-aware (gating
now would institutionalize churn-window false-reds).
DELTA 2026-07-11 LATE (dht-unity cure, dev@270dbafac): CONVERGED LIVE —
both doorways resolve the IDENTICAL canonical head + blob for
elohim-host-landing (verified 20:40:35 UTC: uhCkkPVC7g…/84e1d803…,
trust=notarized on A and B). ROOT CAUSE of the fetch seam: the conductor
webrtc_config key was `ice_servers` (snake) — Holochain passes it
verbatim into tx5's serde-camelCase WebRtcConfig, so the fleet ran with
ZERO ICE servers since inception; host-candidates-only died at the
2026-05-27 shem split (F-T19 era). One-word rename to `iceServers`
(+ TURN fallback) cured cross-conductor fetch; elohim.host's conductor
verified-adopted the declared head minutes after deploy. Render-time
ICE validator + per-seam substrate smoke added to the pipeline.
Scenario 2 flip pending ×2 fresh edge validations (measurement, not
substance). Earned-authority gate (scenario 3→social grant) remains the
node's open red.
DELTA 2026-07-11 (shift notary-scenario2-green, dev@8953fa423): the
ELECTION + DECLARATION layers are DONE and live — tier-aware
partition-deterministic cross-root selector (earned>staging at resolve,
target-id gate, deterministic tiebreak) hot-swapped onto BOTH genesis
conductors (functionally proven: scenario 3 green = new guard refusing
unauthorized moves; elohim.host's conductor answers the new fn's own
retrievability refusal), the declaration act WIRED (deploy designates:
authorHeadOnce -> POST /db/content/{id}/canonical-head, propagated to
EVERY doorway each app deploy, idempotent-by-content), declaring side
converges within the same deploy (row+resolve = declared hash). The
SOLE remaining cause of scenario 2 is SUBSTRATE: elohim.host-side
conductors cannot RETRIEVE matthew-authored actions ('target action
not retrievable' from the declare propagation, every app deploy — the
live per-deploy diagnostic) — the F-T19 cross-conductor fetch gap,
with the signal bus now VERIFIED live on both doorways
(signalShared:true), so the gap sits below the relay layer. Substrate
cure lands -> convergence is automatic on the next deploy. See
genesis/data/timeline/backlog/genesis-pair-cross-conductor-fetch-blocks-canonical-convergence.md.
DELTA 2026-07-04 (shift notary-authority-land, dev@068c917cf): Phase C
core LANDED and measured — 2 of 3 scenarios STABLE-GREEN across three
consecutive fresh-trigger edge builds (#1153/#1154/#1155): the HEAD
authority surface is live (GET /db/content/{id}/head 200 notarized view;
unauthenticated non-author move refused 401; DNA declare/resolve with
author-filtered election, coordinator-only hot-swap verified applied on
both genesis peers; sweettest declare_head_notarizes_and_supersedes green
in DNA #1350) and the baseline holds. Scenario 2 (elohim.host
trust=notarized) is the SOLE remaining red: the cross-peer anchor
propagation mechanism is PROVEN LIVE on matthew (Loki: "HEALED content
anchor from own conductor (peer discovery)" x3 for the landing EPR; 300+
anchors healed mesh-wide after the view-federation 1MiB payload +
self-truncation fix) — adam's stamp is blocked by node-level outbound
view-federation degradation (F-T19 Timeouts to ~11 peers outside its
post-boot window; leads + morning verification path in
genesis/data/timeline/backlog/view-federation-request-flakiness-mesh-wide.md
and the notary-authority-land sprint result). Prior red decode preserved:
RED WRITTEN + verified failing live 2026-07-03 (against alpha federation):
3 scenarios — baseline green (alpha-A serves trust=notarized with
dhtAnchorHash, proving the check is non-vacuous; Phase A/B trust field
already landed), 2 honest reds: (1) elohim.host serves the same EPR at
trust="published", dhtAnchorHash null — the notary HEAD anchor never
propagates to federation peers; (2) /db/content/{id}/head returns 404 —
declare_content_head (Plan C1/C3) unwired, so nothing can refuse a
non-author HEAD move. Both pass unchanged when Phase C lands. The node is
now schedulable: implement C1-C5 against these reds.
