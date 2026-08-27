---
epr-habit-version: 1
id: blob-durability
invariant: >
  Blob bytes survive peer loss and churn: custody commitments honored,
  RS-quilt placement household-diverse, salvage re-places without data loss.
status: green
active: false
checks:
  - "a2o @concern:blob-durability — 8 scenarios across genesis/a2o/features/resilience/ (governed-distribution, salvage-placement, chaos-peer-churn, grandma-photos-survive-node-loss, app-blob-heal-on-read, observable-distribution, resilience-dimensions, household-diversity-dataplane)"
refs:
  - "genesis/data/timeline/backlog/dataplane-peer-fallback-and-blob-replication.md"
  - "memory: project_dataplane_next_lens_diversity_placement (1a+1b landed; prod join dormant)"
retire-when: >
  never: custody of bytes a household entrusted to peers is a floor. The failure mode is
  silent and irreversible — nobody reports the blob they can no longer fetch — which is
  exactly the class that must not depend on someone remembering to look.
---
DELTA 2026-08-23c (swarm rows landed by Codex, 4009362f0: parity-aware
completion, bounded snapshot+delta inventory, data-shard-first placement;
re-verified live on the mesh in DUAL transport on a p2p-iroh build:
app-blob-heal-on-read 2/2, doorway-failover 10/10). The curve number
itself (roadmap S3: 1/2/3 holders, wall-clock falls) is still unmeasured —
the spec's new scenarios land born-red and will red this habit until they
pass. Mesh left running in dual.
DELTA 2026-08-23b (observed RED is real, status stays green pending the
fleet lane): ANY blob over RS_THRESHOLD (64 MiB) panicked `PUT /blob` on
every storage peer (http.rs:2617 — the rs-4-7 manifest was hand-sliced as
raw chunks; index 4 starts past the body), so the RS band had never worked
through ingest and every >64 MiB artifact was silently undurable (client
saw a dropped connection after 100 Continue; blob 404 everywhere). Found
when the landing SSR bundle (71,763,974 B) crossed the line. CURED
8854f6de5: shards from ShardEncoder::create_shards; hash/count mismatch is
a hard 500 with nothing stored; reassembly reconstructs through parity
(serves with up to 3 of 7 shards missing); blob_available_locally agrees.
4 red-first integration tests (tests/rs_blob_erasure_coded_put.rs) +
sharding unit test; gate green. Live: 201 on matthew/jessica/james,
byte-identical GET. New story "An artifact over the erasure-coding
threshold is accepted whole and served whole" (app-blob-heal-on-read.feature,
@requires:owned-substrate) — household lane 20260823T150653Z: PASS.
Seams named: ShardAssignment carries no data-vs-parity marker (placement
cannot keep the 4 data shards diverse on purpose); RS ingest peaks ~4x the
body in memory on an unbounded HTTP route. Remaining reds in this concern:
chaos-peer-churn 3 (custody precondition; backlog
seed-custody-coverage-for-drill-content, Codex-claimable).
DELTA 2026-08-23a (AGENT SPRINT on the 37 failed + 35 pending, household lane,
closing run 20260823T000551Z-32aff87a): 186 passed / 25 failed / 4 pending /
29 skipped in 30m06s (this morning 164 / 42 / 35 / 23); saga 22/22 in 46 s
(run 20260823T000501Z). Three agents, disjoint write-sets, one flock for the
mesh. Doorway (10/14 green, gate green): the 91 s pre-listen SSR boot stall
(render/registry.rs from_env materialised bundles before the listener bound;
now budgeted, boot ~4 s) was the one cause behind 5 reds; --conductor-url
fallback derived admin=4443 so the projection subscriber never connected on
the mesh (fixed by port range); connected/connected_workers coherence;
backoff-ladder attribution; x-ssr-skipped was the a2o measuring the response
cache, not the render queue. Storage (gate green): X-Content-Address always
CIDv1; delivery rows carry declared caps; failed ping closes the libp2p link;
web2 re-seed poison (fabricated blobHash, never restored) and 1200-way fan-out
fixed in steps. Pending: 6 → green, 19 → honest @browser/@requires/@wip holds,
5 named with the lane gap. Two defects the sprint itself surfaced and cured:
(1) the mesh lock fd leaked into re-exec'd storage peers (40 min deadlock;
closerange before exec); (2) 'install in declared order' plus a late-
registering primary (its circuit open mid-drill) made doorway A serve
JESSICA's projection as alpha-A — three saga chapters red on the 23:27 run —
cured by declared steward-peer priority in RouteRegistry (unit-tested). Still
red, classified: identity join 6 (chaos-peer-churn 3, household-formation 2,
identity-coherence 1 — design pass); self-healing-flow-control 5 (2 structural
warm-up arithmetic, 2 blocked on storage queue-not-shed at http.rs:1243, 1
half-open new); conductor-spin 3; delivery-diagnostics 3 (ready_content needs
gossip; fixture count); epr-cross-peer 2 (sync plane wins the race, p2p arm
unreachable); cross-doorway-content 2 + peer-mesh 1 (alpha degraded/caughtUp
false mid-lane — transient under load); content-sync 30 s under load; web2
cache 8/40; conductor-visibility (agents registered only at boot discovery).
Named RED with lines, refused as suggested: doorway blob forward has no pool
failover (storage_proxy.rs:743/971) — the single-target gospel forbids
per-request iteration; the cure is selection-time (skip an Open/Offline peer
when choosing the ONE target) and is a design decision for this habit.
DELTA 2026-08-22d (SPRINT: saga performance + reliability + code quality,
household lane, closing run 20260822T201747Z-3bd326d6): 170 passed / 37 failed /
35 pending / 22 skipped in 20m58s, from 164 / 42 / 35 / 23. Saga 22/22 in 2m16s
(was 4.8 min): chapter 11's exhaustion wait 290 s → 50 s via a declared
ACQUISITION_RECONCILE_SECS knob (prod default unchanged at 60; mesh profile 10).
The dead-peer cascade (~21 reds) is cured at its root and PROVEN: the drill's
fs.copyFile on procfs wrote a 0-byte environ and storage-restart exited 0 on it;
now the capture reads to EOF with the exe recorded beside it, the script restores
a dead peer from that record and returns 1 if any target does not answer by port
(capture+SIGKILL jessica → storage-restart jessica → back with key, cadence and
fixture pid). Zero stale-pid/ECONNREFUSED-by-dead-peer reds remain; all five
services alive at the end of the lane. Chaos drills now red HONESTLY at their
custody precondition: manifesto commitmentBacked 1 / heldBy [] and chaos-ladder's
fossil matthew key are the substrate identity join (membership truth dead —
identity-coherence design pass, held), not the drill; the drills match a provider
by libp2p peerId OR agent key now (reconcile/custody.rs contract). Remaining 37 =
self-healing-flow-control 10 (doorway re-exec >90 s to healthy + 4 ECONNREFUSED
behind it, breaker/half-open, warm-up evidence), peer-conductor-resilience 4,
doorway-pool-degrade 3 (fixture expects 0 rows, mesh has 3), conductor-spin 3
(1 missing dependency on an undisturbed mesh; james authoring failed), identity
join 5 (household-formation 2, identity-coherence 1, chaos 3 precondition),
peer-recovery 2 (needs a wiped projection), peer-loss-failover 2, delivery-diag 2,
web2-absorption 2, content-sync 30 s under load, content-addressing header,
acquisition-pins socket close. Next: the doorway re-exec path in
self-healing-flow-control (one cause, 5 reds), then the identity join.
DELTA 2026-08-22c (CLOSING MEASUREMENT, household lane, run
20260822T183819Z-123cea49 on the committed tree): 164 passed / 42 failed / 35
pending / 23 skipped, against the morning's 159 / 18 / 35 / 52. Read the 42 in
two halves. 17 are the morning's reds still standing (one healed: the wedged
request path); every one is classified in the deltas above. 25 are NEW and every
one of them is a previously-HELD destructive scenario that the unified gate now
lets RUN — self-healing-flow-control (10: doorway re-exec with paused peers,
breaker/half-open, warm-up budgets), peer-conductor-connection-resilience (4),
peer-loss-failover (2), and 7 ECONNREFUSED where a drill killed jessica and its
own restore never brought her back. That is the lane's destructive backlog
MEASURED for the first time instead of silently skipped — the denominator moved,
not the invariant. Two cascades the first whole run exposed are cured in
hc-mesh.sh (123cea498): the re-exec'd doorway's cold membrane banning the
lane's own loopback client (403 x-membrane:deny), and fixture pids going stale
after storage-restart (kill ESRCH). First move next sprint: the drills'
peer-restore path (a killed peer must come back by the drill's own hand, by
port, never by a remembered pid), then self-healing-flow-control's ten. The
habit-bound concerns on this run: doorway-failover 9/0, operator-runtime-surface
3/0, saga-11 3/0, notary-authority 4/0, federation-deploy 2/0,
reach-enforced-http 3/0; blob-durability 1/4 (chaos + heal-on-read, seed gaps
above); content-sync 3/1 (the 30s window, again under lane load — 4/4 scoped).

DELTA 2026-08-22b (household lane, runs 20260822T170136Z + 180614Z): saga chapter 11
— the one @requires:owned-substrate skip the delta below records — is now GREEN
3/3 on the household lane: a want NO peer holds (`epr:jessica-unheld-want`, the
step proves 404 on every peer before pinning) exhausts the peer-sized 3-probe
budget in three 60s reconcile cycles, retires, increments
elohim_acquisition_pin_retirements_total{reason="exhausted"}, and the queue is
caught-up again by the next cycle. It had been skipped for TWO reasons, neither
the mechanism: every destructive gate re-read A2O_ALLOW_DESTRUCTIVE on its own
(nine step files), so a lane that DECLARED owned-substrate still held every
destructive step — now ONE gate (substrate-scope.ts destructiveAllowed: the
declared cap, env var as override, never fail-open); and the scenario named the
household's own landing page, which every peer holds, so its pin could only be
satisfied, never exhausted. The same full-lane run carries 3 chaos-peer-churn
reds, classified NOT a durability regression: "flapping" + "two-peer loss" red
on `"manifesto" has no custody footprint on this mesh` — no custody-blob row
carries contentId=manifesto, /api/v1/resilience/manifesto/household reads
commitmentBacked 1 / hasHouseholds 0 — a SEED gap (hc-mesh-prologue's
seed-commitments custody leg for the commons EPR did not land); "cascading" reds
on `no custody-blob commitment names matthew (12D3KooW…) as provider` because the
step matches `provider` against the libp2p peerId while the 81 rows on matthew
carry BOTH namespaces (agent keys x49 including two fossil james keys, peerIds
x24) and reconcile/custody.rs documents EITHER-namespace matching — the step
must accept either. observed_status red stands until the seed leg lands and the
step is corrected; status green is unchanged because neither red is the
invariant failing.

2026-08-22 DELTA (local-mesh evidence; no status flip — already green): the
resiliency-saga scoped lane reached 21/22 passed + 1 @requires:owned-substrate
skip, 0 failed on the 3-peer household mesh (wave3c, commit 833ba4c58 serving) —
the T21 double-wrap cure restored CID-addressed byte replication (evolution-of-trust
bytes 200 on all three peers, zero T21 rejections, invalid_markers:0), and ch05/06/10/11
flipped green via consent-pin seeding, served-head adoption + per-peer serverBlobHash
stamp, and household-id canonicalization. Fleet confirmation rides the next edge roll.
2026-08-20 MEASURE DELTA (scope, not a status flip): the #1489 red-triage
pass retagged observable-distribution.feature. "Content-viewer resilience
tooltip is live" gained @wip — openContentViewerStub() in
steps/resilience.steps.ts returns 'pending' unconditionally, so both Then
assertions were unreachable and the scenario measured nothing. Under
scripts/ci/run-dataplane-validation.sh:135 (--tags '@dataplane and not
@wip and not @browser-only') this file now contributes 2 scenarios where
it contributed 3. The same pass moved that file's @local from feature
level onto the two ingest-then-place scenarios that actually write into
the mesh they measure, returning the rest of the file to both CI gates.
The checks clause names files, not a scenario count, so it stands
unchanged; status stays green on the 2026-07-03 evidence below. Recorded
because a habit-bound measure moved, and a measure that moves silently is
how a green stops meaning anything.
GREEN 2026-07-03 (edge #1148, dev@9ade36dee): blob-durability passed=3
failed=0 pending=7 — ZERO failures with the operator-approved
@browser-only exclusion live (c387260cb); every pending is an
honesty-gated skip (unseedable live-discovery preconditions +
seed-lever substrate timing), not a defect. The non-browser measure's
executable ceiling is 3/3 passing; the browser-mode run (Playwright
tier) is the named expansion path, not a gap in this check. Prior
decode below preserved for the arc:
MEASURED LIVE 2026-07-03 (shift blob-durability-suite-green, edge #1146,
dev@6d142be79): blob-durability passed=3 failed=5 pending=12 (of 20
scenarios in this concern) — the harness-gate/seed-lever/CI-dispatch work
below converted the prior 0/19 fiction into a real, fully-decomposed
signal. Every failure and pending is now individually accounted for:
(a) 5 failed — Playwright-device hard-fail in steps/ui/topology.steps.ts
+ one viewport-archetype step; this CI job never runs in Playwright mode.
Fix is known (tag @browser-only + extend run-dataplane-validation.sh's
--tags filter) but changes what the measure counts, so left as a
deferred operator/next-shift design call, not made unilaterally
(backlog: a2o-playwright-device-hardfail-topology.md). (b) 12 pending —
honesty-gated skips working as designed: unseedable /api/v1/peers/delivery
preconditions (live-discovery-only, provably unseedable) plus
Playwright-gated browser steps that correctly return 'pending' rather
than hard-fail. Not defects. (c) 3 passed — up from 0 (harness-gate fix)
then 1 (edge #1144, first real run) then 2 (edge #1145) then 3 (edge
#1146, after fixing a 403-vs-404 idempotency bug in the test fixture's
GET-first content-create check). Storage side proven defect-free for
every scenario that actually executes; remaining gap to target (≥5) is
structural (needs Playwright) or substrate-timing (seed-lever precondition
not met this run), not a code defect. Along the way: fixed the
E2E_STORAGE_URL harness gate, flipped ALLOW_SEED_SHARD_MANIFEST live
across matthew/jessica/james, fixed 3 real bugs in the seed-activation
step chain, and fixed a CI dispatch-correctness gap
(elohim/holochain/build-manifest.json had no source path matching
genesis/a2o/** — a2o-only pushes silently never triggered elohim-edge).
Full arc: .claude/shifts/2026-07-03T00-00-blob-durability-suite-green.journal.md
and its sprint result. Edge #1147 (2026-07-03 15:28) reproduced 3/5/12
byte-identically — deterministic; the operator-approved fix (c387260cb,
@browser-only excluded from the non-browser run) is on
feat/frontend-eyes-sprint awaiting the next dev-merge + edge run, which is
the flip-to-green measurement. Sibling reds in #1147 (peer-mesh
caughtUp=false on elohim.host) and app #1585 (Upload SPA Blob /lamad
syncing) were the 2026-07-03 shem network-degradation window — live-probed
recovered 17:0x UTC (caughtUp=true both doorways, /lamad 200)."
