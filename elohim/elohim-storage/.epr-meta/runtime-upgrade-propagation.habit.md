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
  - "a2o @concern:runtime-upgrade-propagation (genesis/a2o/features/delivery/runtime-upgrade-propagation.feature — Stations 1-8 RUNNABLE (r12 2026-09-02, 8/8): steps/delivery/runtime-upgrade-propagation.steps.ts composes the three drivers against the household mesh; the two constitutional scenarios stay scenario-level @wip / pending)"
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
