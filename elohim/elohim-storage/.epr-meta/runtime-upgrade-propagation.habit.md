---
epr-habit-version: 1
id: runtime-upgrade-propagation
invariant: >
  A runtime release (coordinator bundle first; config, binary, hApp bundle behind it) reaches
  every peer by ELECTION on its release channel — staged, soaked by a canary, promoted on
  attested evidence, converged, and revertible by re-election — with conductors never restarted
  or re-keyed for the coordinator class and mixed-version peers still talking throughout.
status: green
active: false
checks:
  - "a2o @concern:runtime-upgrade-propagation (genesis/a2o/features/delivery/runtime-upgrade-propagation.feature — Stations 1-9 RUNNABLE on holochain 0.7 (it8/it9 2026-09-04, 9/9 twice; r12 2026-09-02 8/8 on 0.6): steps/delivery/runtime-upgrade-propagation.steps.ts composes the three drivers against the household mesh; the two constitutional scenarios stay scenario-level @wip / pending)"
  - "manual chain (mesh): genesis/a2o/scripts/epr-release-package.ts (T1) → release-ceremony.ts channel create / publish / promote / revert <manifest> (T2) → GET /admin/adoption on every peer (T3/T4) → release-attestation-probe.ts EXIT=0 (T5)"
refs:
  - "spec: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md (§5 verify floors, §10 receipt chain)"
  - "arc: genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md (cycle-time table)"
  - "receipt atom: genesis/data/timeline/backlog/task-runtime-upgrade-a2o-receipt.md"
  - "controller: elohim/elohim-storage/src/services/release_adoption/{watch,verify,state,apply}.rs"
retire-when: >
  when three consecutive coordinator releases reach the alpha fleet through their release
  channel with no Jenkins roll and no operator hand on any pod, each with its cycle-time row
  in the arc doc — the CI roll is then no longer a delivery path for the class and the
  register describes a product, not a practice.
---
DELTA 2026-09-06: Nachalah #1a's non-actuating signed candidate/compatibility contract passed `just gate elohim-ark` (EXIT=0, 126 tests) and independent review; report `genesis/docs/superpowers/plans/task-nachalah-candidate-contract-report.md`. Grant proof/current revocation (#1b), activation, rollback and repeated household ceremonies remain open; no adoption or earned-arc claim is added.

DELTA 2026-09-05 (Nachalah prerequisite): `just gate elohim-ark` passes 120 tests including kernel-observed executable identity and witnessed readiness failure; `hc-mesh.sh` declares the identity rung. This is not a conductor-adoption or rollback receipt; the existing coordinator-only green scope is unchanged and `nachalah-allotment` remains unwired.

DELTA 2026-09-02 00:43Z (local mesh receipt, r2 channel; transcript
`genesis/a2o/reports/release-ceremony/2026-09-01/transcript.md`): the full §10 chain
PASSED by hand-composed drivers — publish→3/3 staged ≤19 s; canary (james, mode
`canary`) hot-swapped ~12 s after his first sweep, conductor PID unchanged, wasm hash
flipped; soak attestation after 30 s read 1/1 by both observers; promote→3/3 applied
in 75 s; revert via `revert <channel> <manifest>`→3/3 restored in 31 s; attestation
probe qualifying 2 / builder-excluded 1 / mismatched 0. Five typed refusals became
five controller fixes (bd5d3984b 547c28d62 851ab2fae 2b02dd86f + driver revert) and
two zome/build atoms (update_content star chain; hc-rna cdylib link). Status GREEN on this measured pass (the hand-composed driver chain is the runnable check;
the a2o feature's steps composing the same drivers is the next station).
Preconditions learned: doorway A up (bootstrap/signal home) before any election measure;
candidate = coordinator-only bytes with byte-identical integrity (COORD_BUILD_MARKER, or a
wasm custom section while the DNA workspace build is broken).

DELTA 2026-09-02 03:1xZ: Stations 1-5 pass as cucumber steps on the household mesh — 5/5 scenarios, 62/62
steps, 4m56s (publish 53 s · staging 3/3 50 s · canary adopt+attest 110 s · promote 2 s · fleet converge
81 s), no refusals; scoped via a one-feature cucumber config + `--name '^Station [1-5]'`. Seam captured: the
story's two-channel model (shared soak channel + commons) is stood in by ONE channel moving staging→earned
because the driver moves a head in place — a missing node between story and driver, not a fake.

DELTA 2026-09-02 09:1xZ (r9, `genesis/a2o/reports/release-ceremony/2026-09-02/cucumber-stations-1-8-r9.{log,json}`):
Stations 1–6 AND 8 pass as cucumber steps — 7/8 scenarios, 97/100 steps, 29m — the first run to reach a REAL
revert-apply (3/3 restored to baseline by re-election) and to assemble the observed version matrix. Three
defects fell to get here, each measured before fixed: (1) the fixture left the fleet heterogeneous between
runs → every run now converges every peer to the baseline before and after (8181d60a8); (2) a revert
packaged at the forward attestation threshold is refused `threshold_unmet` forever — nobody attests a release
the fleet is asked to leave — so a revert manifest declares threshold 0 (fc090d901); (3) the controller kept
serving a node's PRE-apply installed reality for INSTALLED_REALITY_TTL_SECS after ITS OWN apply, refusing the
next release on that node `coordinator_lineage_mismatch` into the backoff ladder — apply now invalidates the
snapshot and a stale-cache mismatch re-reads once (6ae703bd2; on origin/dev, mesh binary rebuilt this shift).
Station 7 (personal channel alongside commons) is the remaining red: a personal variant bound to the baseline
stops matching once james runs the commons candidate — a node runs ONE coordinator per role, so a personal
channel must REBASE when commons moves (its appliesTo is what it supersedes); fixture rebase in flight.
Race learned: james following the personal channel in `canary` mode applied it before the commons canary
reached him (r8) — the story wants it heard (`apply` mode → `waiting`), never applied (b2e69d2d3).

DELTA 2026-09-02 11:53Z (r12, `…/2026-09-02/cucumber-stations-1-8-r12.{log,json}`): **Stations 1–8 PASS —
8/8 scenarios, 100/100 steps** — the whole rung-5 story runner-observable on the household mesh with both
controller cures live in the mesh binary: a node's own apply invalidates its installed-reality snapshot
(6ae703bd2) and a channel's backoff is keyed by the resolved head, so a rebased head is checked on the next
tick (e193ac272; measured: james resolved each rebased personal head in ~40 s where r10 waited 30 min).
Station 7 is honest now: the personal channel rebases when commons moves (118688cf2), its per-station
expected head is recorded (a8e233e35), and james's runtime is converged on commons at every moment while his
channel diverges compatibly. The fixture still waits out INSTALLED_REALITY_TTL_SECS at Station 6 — no longer
needed with 6ae703bd2 live; removing it is the next measured step. The checks line above still says
"Stations 1-5 RUNNABLE"; it now reads Stations 1–8 (this delta is the evidence for that flip).
Fleet caveat, same day: the alpha conductors are not reachable (source chains torn by a drift-reinstall
under a standing ALLOW_DNA_REINSTALL=true after an unintended integrity-hash move — see the escalation
atom), so this proof is household-only until the fleet is re-genesised on the pinned hashes.

DELTA 2026-09-03 16:00Z (holochain 0.7 cutover, F2; receipt `genesis/a2o/reports/sprint-report-household-20260903T154932Z-fcb81456.{md,json}`, log `reports/release-ceremony/2026-09-03/cucumber-stations-1-5-hc07-r6.log`): **Stations 1–5 PASS 5/5 (57/57 steps) on the STOCK holochain 0.7.0 conductor** — household mesh, ark launch, local iroh-relay 1.0.3, storage `dual`; james's controller applied the staged candidate via `sync_coordinators` ("coordinator hot-swap applied (no re-key, no DHT churn)") and attested; promotion + convergence held. The chain survives the conductor line change. Five reruns were spent on apparatus, none on the line: (r1) worktree lacked the packed `workdir/` bundles the packager stats; (r2) the baseline pair was pinned to the 0.6 content_store WasmHash — repinned to the 0.7 bundle N `uhCokta9…` (verified byte-exact by re-deriving Holochain's WasmHash from the packed DNA), now env-overridable; (r3) storage peers had no `ELOHIM_RUNTIME_CONFIG_PATH`, so the rung-4 watcher was never armed (`/admin/adoption` `sweeps:0`); (r4) `hc sandbox call` is gone in 0.7 → `AGENT_PUBKEY=""` → soak attestor had no deviceId. All four are hc-mesh.sh / a2o fixes on `upgrade/holochain-0.7`. Stations 6–8 not re-run this pass (scope = the habit's runnable check, stations 1–5). Status stays green; the 0.7 FLEET receipt (F5) is the next flip.

DELTA 2026-09-03 22:3xZ (scope, from the 0.7 cutover): this habit's green covers COORDINATOR-ONLY releases — integrity
zomes, DNA hashes and the conductor line unchanged. The holochain 0.6→0.7 cutover was three breaks at once (conductor
data layer with no upstream migration, integrity hashes moved on the hdk/hdi bump, kitsune2 0.4↔0.5 cannot talk) and
NONE of the adoption controller's vehicles carried it: the fleet was wiped and re-genesised (operator authorization
2026-09-03). That is not rung-5 evidence and must not be counted as such. The missing rung — carrying a network's data
across a lineage break, mixed-version peers still talking through the storage P2P plane, a migrate-from recipe declared
in the release manifest BEFORE data is held, adoption as a Mishpat commitment with a revert — is filed as
genesis/data/timeline/backlog/2026-09-03-lineage-crossing-migration-rna.md. Today's wipe is the last big-bang by intent.

SEAM 2026-09-05 (found delivering the Holochain Evolution Epic's Task 2 on the 0.7 mesh): chain runtime-upgrade-propagation /
between Station 4 (earned promotion) → "the NEXT release on the same channel" / missing node: `release-ceremony.ts publish`
refuses any new staging candidate on a channel whose head is EARNED (adopt-before-author preflight, no override; `revert`
is the only earned-tier verb), so a long-lived channel cannot take a second candidate — the a2o fixture hides this by
minting a fresh channel per run (`runtime:coordinators:elohim:a2o-<stamp>`). The story's "ONE head with two tiers" implies
publish-over-earned must be admissible once the publisher has ADOPTED the earned head (AUTHOR-THEN-ADOPT arm). Current
state: workaround = fresh channel per release (epic Task 2 fix round 1); the driver fix is an open rung-5 item.

DELTA 2026-09-04 10:2xZ (overnight shift `rung5-long-lived-channel-0-7-mesh`; receipts `genesis/a2o/reports/release-ceremony/2026-09-04/shift-it*.{log,json}`):
**Stations 1–8 PASS 8/8 on the holochain 0.7.0 household mesh** (it1 06:24Z, 100/100 steps, 12m39s — the first
8/8 on the 0.7 line; F2's 5/5 was Stations 1–5) after one fixture cure: every non-first release names its lineage
parent (5e4d97633; the 0.7 content_store orders the release chain, so a null-parent revert is honestly
`lineage_parent_mismatch`). **Station 9 — the NEXT release on the SAME long-lived channel — PASS** (it7 10:1xZ,
198 s): the seam filed 2026-09-05 is closed by four composed cures, each observed live before the run: (1) zome
`content_store` election carries a STAGING CANDIDATE beneath the earned winner and admits a staging declaration
over an earned head when the manifest names it as lineage parent (438443cf0 + wire 7e17a2d96 + sweettest
096017d75; packed DNA hash unchanged; delivered to the mesh BY RUNG 5 on channel `c3-20260904t062743z`:
publish→3/3 earned-applied 300 s, PIDs unchanged); (2) the canary follows the candidate (`watch.rs` 86445bd08 —
"canary follows the staging candidate standing beneath the earned head", 8 unit tests); (3) a peer already running a
release's TARGET bytes is `already_installed` (a73897f77 — james's live `coordinator_lineage_mismatch` at kickoff
became `alreadyCurrent` after the restart) and release artifacts are PULLED from peers, shard manifests followed
(35f0746ad + c8930c2f2 — the c3 ceremony needed two hand `PUT /blob`s; the fixture's personal-channel rebase now
lands without one); (4) the driver admits `publish` over an earned head once the steward has ADOPTED it
(`--adoption-url`, ab3bf32bc) and the packager never defaults a discipline — declared or inherited from the
channel, carried through reverts as `channelDiscipline` (9e1ac7a7b, 6e0b89285). Unit tests: `cargo test --lib
services::release_adoption` 104/104 (80b79717e). Fixture stations read a staged candidate's evidence BY CID
(`release-ceremony.ts attestations <cid>`, 312d4f05d) because no `/admin/adoption` row reports it (backlog
2026-09-04-rung5-c3-mesh-findings §4). Status stays **green**; scope now includes the second release on a
long-lived channel. Pending the integrator: this batch is committed, not pushed (no push credentials in the shift
session; rakia gitlink 4947469 must be pushed first) — the fleet leg of `retire-when` is unchanged.
STABILITY 2026-09-04 11:2xZ: two consecutive full passes on the same binary — it8 `shift-it8-20260904T101912Z` 9/9
117/117 (28m05s) and it9 `shift-it9-20260904T104809Z` 9/9 117/117 (28m12s); Station 9 200 s / 190 s. The `checks:`
line's "Stations 1-8 RUNNABLE" now reads Stations 1–9 on the 0.7 line (this delta is the evidence). Not yet a
Jenkins-confirmed batch (no push from the shift session).

DELTA 2026-09-05 (a peer joins a channel through its own API). rung-5 Task 2 (ee12a04a2, approved):
`POST /admin/runtime-config/follow` rewrites only the `ELOHIM_RELEASE_CHANNELS` line of the WATCHED
runtime-config (temp + rename), reloads, refuses malformed ids/modes by name, 503 without a watched file;
both delivery fixtures enrol through the route (byte-restore kept as the safety net); unit tests for the
line rewrite. Stations 1–9 re-measured on the rebuilt mesh next. Status unchanged.

DELTA 2026-09-06 (Nachalah #1b-a, exact signed grant record): coordinator ActionHash readback and acting-node action/entry/author authentication verified; full storage gate 4386 passed / 0 failed / 59 ignored, focused coordinator test 1 passed, schema-DNA / manifest-hygiene / sweettest-check green, independent review approved. Preserved app prerequisite gate: 4570 tests passed. Evidence: genesis/docs/superpowers/plans/task-nachalah-grant-record-report.md. Status and scope unchanged: full delegation authority/freshness, activation, automatic rollback and both household upgrade runs remain open; earned-arc actuation remains disabled.

DELTA 2026-09-06 04:39Z (FIRST WORKSPACE→FLEET CROSSING — rung 5 leaves the house; receipt
`genesis/a2o/reports/workspace-release/2026-09-06/shift-it2.json`, manifest `…/workspace-release-20260906043835.json`,
candidate `…/candidate-20260906043835-workspace/`): `workspace-to-fleet-release.feature` **Stations 1–5 PASS 5/5, 36/36
steps, 59 s** against the LIVE alpha fleet (edge #1432 deployed 04:2xZ with every active human enrolled
`runtime:coordinators:elohim:workspace=observe`). The workspace peer = the fleet's own fork conductor (holochain 0.7.0,
extracted from harbor `elohim-edgenode:conductor-25dd2d0be144` — no 45-min build) joined via doorway-alpha's bootstrap and
`relay.alpha.elohim.host`, installed the deployed bundle, and sits in the SAME five spaces the fleet's peer store lists
(`/db/p2p/conductor-diagnostics`: imagodei RGwtz…, infrastructure 5a385…, lamad Zezl4…, mishpat QwOEw…, node_registry
51dR_…; agentCount 35→43 as it joined) — not a partition. Its storage follows the channel at observe. Stations: (1) mint
4.1 s — coordinator-only candidate bound to the installed reality, marker section `elohim.coord-build-marker`; (2) publish
7.0 s — one act by the developer's own peer, no pipeline; (3) crossing 1.2 s — the election read back through his own
conductor; (4) 46.7 s — the staging head `uhCkkqmSrk6…` verifies `ok` against the reality the fleet declares and NOBODY
applies it (appliedRelease null, attestations 0/0 at threshold 1 — observe mode attests nothing, as designed); (5) five
commands or fewer, the manual step named. Iteration 1 was apparatus (hc-start.sh cold-built a release storage binary,
then its failure path deleted `.hc_ports` under a live conductor — filed as a seam on the T3 rung). Status stays
**green**; scope now includes the workspace→fleet observe rung. NOT yet a stability receipt: one pass; the re-run after
tonight's edge redeploy is the fresh-trigger measurement. NOT observed from here: the fleet peers' own controllers
resolving the head (their `/admin/adoption` is storage-local; a doorway-projected adoption summary is the missing
observability node). The `retire-when` fleet leg — three coordinator releases REACHING the fleet by election — needs the
=observe→canary/apply data flip the operator holds; this receipt is what that flip was waiting on.

STABILITY 2026-09-06 07:1xZ (workspace→fleet, four releases on ONE long-lived channel across a fleet redeploy; receipts
`genesis/a2o/reports/workspace-release/2026-09-06/shift-it{2,5,6,7}.json`): it2 5/5 (04:39Z, edge #1432 fleet) · edge #1434
redeployed alpha (fresh trigger, 06:1xZ) · it5 4/5 — the SECOND release on `runtime:coordinators:elohim:workspace` was
refused `lineage_parent_mismatch` (envelope declared no parent; the channel chain supersedes release 1) · driver cure
9f2856b48: `epr-release-package.ts` derives `lineageParentCid` from the workspace peer's own `/admin/adoption`
resolvedHead (explicit `--lineage-parent` wins, `--first-release` opts out) · it6 5/5 (71 s) · it7 5/5 (81 s) — two
consecutive passes on the cured driver, releases 3 and 4 each naming the head they supersede, every one verifying `ok`
against the fleet's declared reality with appliedRelease null on a fleet that follows at observe. The `checks:` line's
scope now reads: Stations 1–9 on the household mesh AND Stations 1–5 workspace→fleet (observe rung) on alpha. Seam
closed at the driver, not the fixture: a long-lived channel's Nth release names its parent by construction
(the household sibling had cured the same refusal at its fixture, 5e4d97633). Task 4 of
`2026-09-05-rung5-workspace-orchestration-plan.md` has its deliverable receipt (a plan checkbox flip is the integrator's).
Status **green**; the fleet leg of `retire-when` still waits on the operator's =observe→canary data flip.

DELTA 2026-09-06 11:5xZ (FLEET-SIDE OBSERVABILITY CLOSED; james at canary; the last seam named). Edge #1437 (from
push 950c3964f) deployed the doorway projection `GET /db/p2p/adoption?peer=<name>|all` and the deployments.json
render with james `=canary`. Read through doorway-alpha at 11:5xZ: ALL SEVEN alpha peers (matthew, adam, jessica,
james, gertrude, susan, eve) resolved the workspace channel's head `uhCkkU-TS…` (staging, release 4 of the night,
published from the workspace at 06:51Z) — the fleet-side receipt the morning's crossing could not show now exists,
without pod access. james reads `mode: canary` (sweeps 7 on the new render). Every peer's verdict is `refused
artifact_unavailable` (transient, arm `fetch`): "blob bafkrei… not held locally — asked 6 peers we could reach; none
served the bytes". Cause: the release ARTIFACT was PUT to the workspace peer's own storage (:8090), which joined
nothing — its `/p2p/status` says "P2P networking not enabled" — while the workspace CONDUCTOR is on alpha's DHT. The
election crosses on the T1 plane; the bytes ride the T2 plane, and the workspace peer is a member of only one of
them. This is the seam between "the fleet observes" (Task 4, done) and "james adopts" (next): chain
runtime-upgrade-propagation / between Station 4 → "a canary applies" / missing node: the workspace peer is a T2
member too (its storage on alpha's byte plane, or the artifact published where the fleet pulls) / current state:
7/7 refuse artifact_unavailable, retrying. Not a fixture defect and not a fleet fault — a rung the T3 peer has not
climbed. Status green (scope unchanged); the canary receipt waits on this node.
