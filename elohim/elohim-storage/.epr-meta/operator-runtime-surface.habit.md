---
epr-habit-version: 1
id: operator-runtime-surface
invariant: >
  Any peer operator can inspect a runtime, restart, reseed, and trigger
  reconciliation as commitment-gated protocol verbs — with peer-local
  telemetry served by the peer itself. The dev surface and the operator
  OS-settings surface are the same surface.
status: green
active: false
checks:
  - "a2o @concern:operator-runtime-surface (genesis/a2o/features/dataplane/operator-commitment-gated-verbs.feature — @wip DROPPED 2026-08-18 by operator decision: COUNTS in the edge Dataplane Validation byConcern rollup; also runnable locally with npx cucumber-js --tags '@concern:operator-runtime-surface')"
  - "cargo test --test operator_verbs (elohim/elohim-storage — 9 tests: holder accepted + attestation names grant cid AND recorded event id; no-grant refused; revoked refused; no verified performer refused; loopless peer honest 503 with no phantom rate charge; accepted use recorded bounded_by the grant; refusals record nothing; rate_per_hour ceiling refuses the over-limit use)"
refs:
  - "task #10 (operator directive 2026-07-02)"
  - "elohim/elohim-storage/src/api/operator_verbs.rs (verb handler + kick) · doorway src/server/http.rs op_gate_capability/effective_op_gate_mode (per-verb gate) · seam-registry rows in both crates"
  - "storage POST /admin/arc-policy/actuate — the pre-existing commitment-gated peer-local mutation precedent (sets-authority-arc; conductor restart) this verb family generalizes"
  - "memory: project_rea_compute_commitment_primitive (delegates-compute displaces X-API-Key)"
  - "task #7 (adam has zero Loki streams — the symmetry gap, live)"
retire-when: >
  when a peer operator completes inspect / restart / reseed / reconcile from the peer's own
  settings surface on a released steward build, with no developer-shaped tool in the path.
  The habit then describes a product, not a practice.
---
DELTA 2026-09-10 (doorway /status audit, uncommitted, gate doorway EXIT=0, 1212 tests):
apex /status tiles audited against status.json — 4 disconnected (uptime strip hard-coded
nodata; REQUESTS counted one route; federation consensus stub "0/0 unknown"; no dataplane
peer count) + 3 mislabeled (PEERS=federated doorways; Gateway green on unconfigured NATS;
"Projection Cache" = response cache). All wired/relabeled in doorway-service; SUBSCRIBERS 0
is true (apex DEV_MODE without --dev-signal-subscriber, main.rs:~1221) — operator deploy call.
Status unchanged until the next edge deploy renders the strip live.

DELTA 2026-08-22 (household lane, run 20260822T170136Z-519d4f6b):
operator-runtime-surface passed=3 failed=0 on the 3-peer mesh. No change.

DELTA 2026-08-19b (GREEN — edge #1366 deploy run, in-pipeline quiesce
gate A-QUIESCED then validation): operator-runtime-surface passed=4
failed=0 pending=0 — the positive path now returns 200 with the
attestation naming the exact grant cid. Cure = three stacked defects,
each proven locally on the isolated stack before push (commit 1b8c80f9c):
(1) doorway op-gate authorized performer=claims.human_id while grants
name recipient=claims.agent_pub_key — new op-gate performer resolver
prefers the verified JWT's agent_pub_key; (2) forward_to_storage rebuilds
outbound requests from a header allowlist, silently dropping the injected
x-elohim-verified-performer — now an explicit ForwardCtx field; (3) the
dev-seed lever now supersedes prior ACTIVE dev-seeded grants for the same
(recipient, scope), making revocation observable on an accumulated DB
(never touches non-dev-seeded rows). Local proof arc: register →
no-grant 403 → grant-to-agentPubKey 200 attestation (commitmentCid,
attestationEventId) → revoke 403; a2o locally 3/4 (james refusal red
LOCALLY ONLY — dev_mode hands every hosted register the shared
uhCAk-dev-mode-agent-key; live alpha logins verified DISTINCT keys, and
james 403'd in edge #1366). Confirmation caveat: three follow-up
validate-only runs (#1367-#1369, same evening) all DID-NOT-MEASURE on
chronic A-side quiesce oscillation (measurement-availability debt filed:
genesis/data/timeline/backlog/quiesce-gate-measurement-availability.md);
the green stands on #1366's quiesce-gated measurement + the local proof.
DELTA 2026-08-19 (FIRST LIVE MEASUREMENT — edge #1364 validate-only,
quiesce gate A-QUIESCED sustained 363s, zero fleet churn): the 4
scenarios entered the byConcern rollup as operator-runtime-surface
passed=3 failed=1. PASSING LIVE: no-grant refusal (james, explicit 403),
revoked-grant refusal (matthew, 403), peer-serves-own-telemetry. The one
red is the positive path (holder gets 200+attestation): Loki
2026-08-19T01:44:37Z doorway-alpha logs "op-gate ENFORCE deny …
performer=ba3a0a01-… reason=no active delegates-compute grant for
(performer, capability)" — the seed lever grants recipient=agentPubKey
(uhCAk…) but the op-gate derives performer=claims.human_id (matthew's
doorway UUID; http.rs:1014 defers human_id→agent_cid resolution). The
remaining red is the identity-plane join owned by identity-cross-signed,
not a scenario defect; op-gate ENFORCE is confirmed live on operator
paths. Also proven by the same run: grant lever reachable in-cluster
(Given seeded 2xx), saga rollup refreshed 6/11→8/11 (ch02 recovered,
ch07 banked; ch04/ch06/ch10 red).
DELTA 2026-08-18c (login-realm RCA, desk-proven): the "persona API-mode
auth gap" was FALSIFIED — the login 401s came from the scenarios
addressing peer elohim.host (doorway alpha-B: separate unseeded identity
realm, own user DB + JWT secret); scenarios re-addressed to alpha-A (the
seeded realm) and the desk run now logs both personas in, with scenario
2 (no-grant refusal) PASSING LIVE against alpha — /api/v1/operator/
reconcile answered an explicit 401/403, the first live evidence of the
deployed op-gate. Scenarios 1/3/4 desk-blocked only by in-cluster
storage-URL unreachability from the dev container (CI's runner is
in-cluster); measured by the 2026-08-18 [build:edge] [edge:validate-only]
push.
DELTA 2026-08-18b (operator decisions executed: @wip dropped + slice 2
landed desk-proven): the concern now COUNTS in CI — @wip removed from the
feature (operator authorization this turn; the born-red 3f/1pending joins
the next edge run's byConcern rollup, stage advisory). Slice 2 emit-then-
act: an accepted verb RECORDS its use as an economic_events row
(bounded_by = grant cid, the COLUMN DieselRateHistory counts —
db::economic_events::record_operator_verb_event, the first legitimate
producer against the dormant compute-fulfilled emitter's HELD boundary:
the peer records its OWN authorized execution, parties pre-verified,
consumption scoped to rate accounting + audit; economic-consequence
consumption stays HELD pending cross-signing per identity-cross-signed)
BEFORE kicking, so bounds_validator check 6 is LIVE on this path: 9/9
integration tests incl. the_rate_ceiling_refuses_the_use_past_the_grant_
limit (rate_per_hour=2 grant: uses 1-2 accepted+recorded, use 3 refused
"rate-limit-exceeded", not recorded, no kick) and no-phantom-charge legs
(refusals + loopless-peer 503 record nothing). Attestation body now
carries attestationEventId. Blind-reader loop run per a2o .epr-meta
(2 rounds): round-2 verdict REVISE with NO blockers; comment-level
repairs applied; the remaining MAJORs all want step-TEXT observability
(observable reconcile side-effect step; attestation/directness phrased
positively in steps) — surfaced to the operator as the named deferral
set, since they re-wire live step definitions.
DELTA 2026-08-18 (unwired -> RED, slice-1 cure landed desk-proven; first
local measure recorded): the red is now RUNNABLE — step definitions wired
for all 4 scenarios (steps/dataplane/operator-commitment-gated-verbs.steps.ts;
A2O_RUN_WIP=1 override added to the common.steps @wip hold for local
measurement, CI counting untouched) and the live run measured 3 failed /
1 pending against alpha: scenarios 1-3 fail at persona login (fixture
humans have no API-mode credentials on the deployed doorway — the
verified-performer path IS part of the contract), telemetry scenario
honesty-pends. Cure chain landed the same day, desk-proven: storage
POST /api/v1/operator/reconcile (manifest-declared, doorway-proxied)
authorizes the caller ON THE PEER via the wired authorize_operation gate
(delegates-compute grant, scope=operator-reconcile, 7-check bounds),
kicks the EXISTING projection-reconcile loop through a capacity-1
coalescing channel (no second reconcile implementation; disabled-loop
arms drop the receiver so the verb 503s honestly), and answers with an
attestation naming the commitment cid (5/5 integration tests green,
tests/operator_verbs.rs; kick boundedness unit-pinned). Doorway op-gate
generalized: per-verb capability strings (op_gate_capability), operator
paths force Enforce regardless of DELEGATES_COMPUTE_OP_GATE/dev_mode,
unknown operator verbs deny fail-closed, verified performer rides a
strip-then-inject internal header (22 op-gate kernel tests green). Grant
writer wired to the fleet: ALLOW_SEED_DELEGATES_COMPUTE stamped per-env
in the edgenode template (true alpha/staging, false prod) and the lever
now accepts the template's true/false vocabulary (gate_value_allows,
shared with its stakes sibling — it previously read only "1", so the
stamp would have been silently inert). Flip-to-green condition: fixture
personas can authenticate API-mode against alpha (harness auth gap),
grants seeded via the lever, then the 4 scenarios pass locally; the
@wip drop (joining the CI byConcern rollup) stays an operator decision
per the feature file's own note. Slice 2 (named, not started): emit the
REA economic event per executed verb (bounded_by = grant cid), which
also lights the currently-inert rate check; then operator-restart /
operator-reseed verbs on the arc-actuate precedent, and the peer-local
log seam (/debug/stream is written but unrouted — the adam-invisibility
log axis).
DELTA 2026-08-16: the dev-loop substrate for this habit's verbs now
exists — app/elohim-app/scripts/hc-mesh.sh brings up the alpha-shaped
3-peer mesh in one Che container (3 hc-0.6 sandbox conductors on a LOCAL
island DHT via the doorway's own bootstrap+signal, 3 storage peers on a
libp2p mDNS mesh with proven cross-peer blob replication, dev-mode
doorway), and the facade runner substrate-verify.ts scores 5/7 green
against it — the same floor as CI's Dataplane Validation, with the two
reds in CI's own env-state classes. Runtime verbs built on .dataplane()
can now be proven at the desk before any fleet deploy. Status was unwired
at the time of writing: the first_move red was not yet written.
DELTA 2026-08-16 (overnight): the mesh proved full convergence at desk
scale (3,439 rows × 3 peers → converged=1, known_divergent=0, ~90min,
adopt-before-author minting live) and cornered the >16MB chunked-blob
durability/serving bug the fleet masks (backlog
chunked-blob-over-16mb-not-durable-mesh-repro, HIGH — pins saga
ch03/04/05/09/10 on any bundle over the threshold).

- 2026-08-31 — rung-1 coordinator hot-swap vehicle landed as an operator verb: `POST /admin/coordinators/sync` (dry-run/apply, per-role DNA-lineage guard, embedded+external conductors) + `scripts/ci/fleet-coordswap.sh` rolling driver. Local-mesh proof: upgrade → revert → upgrade, 3 peers, ~40s/peer/pass, conductor PIDs unchanged. Fleet leg = warn-only DNA-pipeline stage; endpoint reaches the fleet on the next edge roll.

- 2026-09-01 — rung-4 operator verbs live: `GET /admin/runtime-config` (effective values + provenance + boot-only reasons) and `POST /admin/runtime-config/reload`; watched-config flip proven on a RUNNING mesh node (same PID, WARN-logged old→new, boot restore on key removal). Fleet wiring: per-human runtime-config ConfigMap → mounted file → watcher.

DELTA 2026-09-02 (local mesh, adoption ceremony r2): the rung-4 runtime-config surface
proved a FIRST follow lands on a RUNNING node — three peers booted following nothing,
one reload each, all three controllers following within 35 s, no restart (fix
bd5d3984b: the controller ticks while idle). Mode flips observe→canary→apply landed
the same way at stations 6 and 7. No status flip (already green); the surface now
carries `/admin/adoption` with typed verdicts + attestation summaries.

DELTA 2026-09-02: slot released — green with wired checks, so it holds no attention; the WIP-fence
slot passes to runtime-death-witnessed (spec 2026-09-02-compute-envelope-tevah-design §12 item 22).
Status unchanged; the checks still run.

DELTA 2026-09-07: chain delegated-sweettest / between accepted → executed /
missing node launch remains nonterminal: a signed refusal or completion prevents
launch even when the attempt was previously accepted / measured locally by
`api::compute_tasks::tests::accepted_attempt_cannot_launch_after_either_terminal_outcome`
in the storage gate. Native grant and result publication, ark supervision, and
age/count retention are implemented; the live Adam journey is not yet measured.
No habit status change or attestation promotion.

DELTA 2026-09-07 (delegated compute, local evidence): storage gate 4,419 passed,
0 failed; Rakia gate 42 passed; real ark fixture covers execution, duplicate
recovery, failure, timeout, interruption and expiry; real two-peer iroh test
proves attachment transfer → lease release → not-found → independent new-owner
reuse without renewing the old lease. Workspace recovery and review adapter
checks pass. Evidence: `genesis/a2o/reports/compute/validation-2026-09-07/`.
The signed-grant → real feedback suite → workspace review journey on Adam
remains unmeasured; worker packaging is disabled pending its pinned image,
actual identity and coordinator deployment. Status unchanged.

DELTA 2026-09-07 (scope reshape): submission binding joins correction contract
§8's operation-id/evidence/outbox and compute's task-file invocation under one
act class in the reimplementation plan's slice-2 inventory handoff; convergence
is deferred. The real two-peer test proves payload transfer/expiry only.
Cross-peer grant authority remains unmeasured and shares slice 1's household
mesh receipt with `dataplane-convergence` when that run lands, with each
concern citing its own station outcomes in the same receipt. No duplicate
commissioning claim and no status change.

DELTA 2026-09-07 (delegated-compute AUTHORITY leg — HOUSEHOLD STAND-IN, measured):
the cross-peer signed-grant journey is measured for the first time on the 3-peer
household mesh, with **jessica standing in for Adam as provider** — this is a
household result and NOT a shem/Adam result; the shem leg stays unmeasured.
Receipt: `genesis/a2o/reports/delegated-compute/mesh-20260907T094431Z/`
(cucumber.json/html + run.log + task/grant/receipt JSON + worker log + inbox trees
+ env.txt, tokens redacted). Live keys: matthew (requester, :8090)
`uhCAkOceT1mHsz3TNXGOQiT4w-SCOgT7iGpsxMVJhoC4tfBg8amFh`, jessica (provider, :8091)
`uhCAk2Z2F7Bzoh_PNj7Zz2jqFOaMmYFJbZIi_0qU_hNvRYSygTYf8`, james (unauthorized, :8092)
`uhCAkHPfTbVOUyPVWvYWGQoSW_iRZM89ZzVBhqW1hm67Flhg4z3UI`; grant issued on jessica's
own loopback adapter, `grantActionHash uhCkk9_xOdVVtcTIYHzP30vgcFADzPB10GafKZnlfQgJPEe017GbE`
(state active, recipient=matthew, rate_per_hour=8). `features/dataplane/delegated-sweettest.feature`
ran via the a2o escape hatch with `ELOHIM_REMOTE_COMPUTE_STATUS=available`
(NOT `just test mesh`, which forces the cap unavailable and skips the whole feature):
**3 scenarios passed / 20 steps passed, 0 skipped, 0 pending, 0 failed.**
(1) recover-completion-starts-review PASSED — submit → client exit → reconnect without
notification → discovered jessica's signed completion (accept
`uhCkk4BBkfzcr2N7epOEwrv3wd59yHPjgz0Qa4f0TUwG8dMsgF_Zo`, complete
`uhCkkXOC6zZtSAocLxwhZLserjIMQYueHtQC4gl4Y2iYcx5RMBmvO`, status=passed exit=0,
observedTests=[`feedback_signal_update_rejected`]) and the codex review saved.
(2) unauthorized refusal PASSED — james's request
`uhCkkJIbjTLTWzYUq9Xts_jQ4vKLoC3Yow7371qqVEQRdzvh8ttSd` refused
`uhCkkVFUK_Ufne18aC3omnShSc8kZoO96pCpp-ILUy2TYY5ri5sbt` reason=`compute-grant-refused`,
with acceptance and completion both absent (the 403 lands on `/accept`, not only on
`/authorize-launch`). (3) receipt-after-expiry PASSED — request
`uhCkkK72BPjaoTQEqxlFEqeAoVg7hpRc3s9_6L536hYLPs5RuYW17`, payload GET 410 after the
60 s lease, compact receipt byte-identical before and after.
Timings (scenario 1): submit 89.1 s (261×1 MiB chunk publish of the 269 MB suite +
3.5 MB DNA); submit→acceptance ≤4.1 s; acceptance→completion ≈243 s (cross-peer blob
fetch + 103 s suite execution, startedAt 09:48:34Z → completedAt 09:50:17Z);
completion→review saved ≈62 s (report mtime 09:51:19Z). Environment notes: the
conductors are the pinned 0.7 fork, storage runs the merged dev binary (compute API +
p2p/p2p-iroh) so no binary staging and no coordinator hot-swap were needed — the
installed DNAs already carry `compute_task` and mishpat's signed-author links;
task descriptors were regenerated against `dna/elohim/workdir/lamad.dna` (09:04Z pack,
sha256 `e18c9bb1…`) and the suite was re-proven locally against it (1 passed, 108.6 s).
Review adapter is `codex` — `claude` print mode exits 1 with "Credit balance is too low".
One substrate defect found and filed, not fixed:
`genesis/data/timeline/backlog/compute-executor-cid-refuses-null-retention-field.md`
(`rakia-executor/src/main.rs:59` + `contract.rs:142` CID a `serde_json::Value`, so an
explicit `"maxRuns": null` dies in `ipld-core` `serialize_unit`; the expiry fixture used
`maxRuns: 1` instead). Habit status unchanged (green): this measures the compute-authority
station on owned household substrate; the shem/Adam leg and worker packaging remain
unmeasured.
