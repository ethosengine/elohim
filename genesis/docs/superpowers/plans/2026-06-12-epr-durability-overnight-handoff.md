# EPR Durability Arc — overnight 2026-06-12 handoff (fresh-session dispatch prompt)

Continuation of `2026-06-11-epr-durability-arc-workstreams-bcdef-pickup.md`
after the 2026-06-12 overnight session (waves 7–9 + the resilience
dimensions proof suite). Paste-able mission for a fresh session follows.

---

## Mission

Drive `propagation.custody-convergence` to 0 (the frozen measure:
`bash .claude/shifts/2026-06-11T12-15-epr-durability-cluster-validation.measure.sh`,
0 = green) and light the resilience surface honestly — the two remaining
traces are NARROW and named below. Then the operator-prioritized
unmeasured-vs-zero view change. CI-gated, story-first, commit-only unless
the push lease is alive.

## DO NOT RE-DERIVE (verified live this night, with evidence)

- **Coordinator hot-swap is real and live.** `happ_manager::sync_coordinators`
  fired twice on every alpha pod (23:53Z single-role; 04:12:59Z
  THREE-role: infrastructure/imagodei/mishpat) — `update_coordinators`,
  no re-key, no DHT churn. DNA hash covers ONLY integrity zomes; `uhCok…`
  in errors is a WASM hash, `uhC0k…` a DNA hash. Gate:
  `ALLOW_COORDINATOR_UPDATE` (defaults to `ALLOW_DNA_REINSTALL`).
- **The attestation lockout chain is dead, layer by layer:**
  "Only the doorway operator" extinct (wave 7); `Role not found: elohim`
  extinct (wave 9 — 12 bridge sites now `OtherRole(LAMAD_ROLE)`; the
  content_store DNA packs as lamad.dna — crate name ≠ role name; no
  single-DNA sweettest can catch that class:
  `dna-bridge-role-name-conformance.md`). **matthew's conductor logs ZERO
  attestation errors — the chain runs end-to-end there.**
- **Infrastructure late-connect works** (matthew/gertrude/adam wired
  heartbeat + 5 signal subscribers late after losing the CellDisabled
  boot race; `connect_role_forever` in-task, genesis-#1122 pattern).
- **Frontend delivered:** eyes-sprint merged to dev; app build #1534
  UNSTABLE-green (the pre-existing lint-debt shape — branch delta was
  verified lint-clean: zero errors in branch-touched files).
- **Resilience dimensions proof suite landed** (spec:
  `2026-06-12-resilience-dimensions-proof-suite-design.md`, operator-
  approved): D1–D9, **32 deterministic tests green** —
  `tests/household_resilience.rs` (threshold ladder, peer counts,
  commitments, diversity, regions), `cluster_view.rs` in-module D8
  (free/used/committed triptych), `tests/chaos_dataplane.rs` (4
  protocol-chaos tests, REAL libp2p node kills, all bounded +
  sha256-verified). Two a2o features: `resilience-dimensions.feature`
  (matrix home; @wip rows = workstream D acceptance) +
  `chaos-peer-churn.feature` (live drills). TDD catch fixed in prod:
  `peer_statuses::list_by_household` was a stub returning ALL peers —
  counts multiplied per household and 3h/1p read "protected"; un-stubbed
  (deployed in the #1066 storage image).
- **Retracted/decided earlier in the arc still hold** (PVC diagnosis
  falsified; conductor DBs healthy; never kubectl from dev).

## FIRST ACTION — the two narrowed traces (in this order)

1. **custody-convergence (the measure).** Run the frozen measure script.
   The failure is now PURELY the DHT/projection leg: "custody-blob
   commitment missing on: adam jessica after 300s (anchor → conductor
   gossip → projection_reconcile)". The mishpat bridges fixed in wave 9
   are ON this leg's live path (CommitmentByState links) and only went
   live at 04:13Z — **the first post-fix projection_reconcile sweeps may
   simply have needed wall-clock.** Check the latest genesis run first;
   if still red: (a) does matthew's custody commitment carry a
   `dht_anchor_hash`? (b) Loki on adam/jessica during the genesis window:
   `projection_reconcile` sweep outcomes + `ProjectionInventory: serving
   local inventory`; (c) the side-link — adam still rejects `Subject
   doorway 'alpha-elohim-host' not found`: did that doorway re-register
   after edge #1066's restart, and does the registration entry gossip to
   shem-node conductors?
2. **resilience.peer-statuses (dark, root narrowed to the signal seam).**
   The heartbeat task runs, ticks 60s, publishes WITHOUT warnings — yet
   `peer_statuses` stays empty (genesis `resilience.matthew.peer-statuses`
   red). Suspect: `PeerStatusRecorded` is either not emitted by the
   (hot-swapped) infrastructure coordinator or silently dropped by
   `subscribe_infrastructure_signals` decode. **Add a loud decode-miss
   counter/log to the subscriber FIRST** (same observability lesson as
   the drift-check silence), then grep the zome's `record_peer_status`
   emit path vs the subscriber's signal-shape match. Bounded fix
   candidate; this is what lights `onlinePeerCount` in the EPR tooltip.

## NEXT — operator-prioritized demo honesty (filed, designed, not built)

`resilience-unmeasured-vs-zero-honest-denominators.md` (backlog, HIGH):
every bulk-seeded content (incl. the demo `elohim-host-landing`) returns
the DEGENERATE at-risk snapshot because the seed path never creates
`shard_manifests` rows — compute() bails at its first join, and "never
measured" renders identically to "measured zero". Two parts:
1. **View honesty (bounded wire change, land first):**
   `distributionState: unmeasured|measured` + live/known peer-count
   pairs (`{live, known}` — operator: "0/0 num types"). Source of truth:
   NONE new — this is a Category-C per-request PROJECTION over existing
   truth layers (`shard_manifests` presence, `stewarded_nodes`,
   `peer_statuses` — themselves projections of DHT entries); no new
   table, entry type, or identity. Schema-first:
   `elohim/sdk/schemas/v1/views/` → Rust → `schema_contract` test →
   `INTERFACE_FILES` codegen → snapshot component. The existing D1
   degenerate boundary test flips to assert `unmeasured`.
   Diagnostic tell to keep: `regionalDistribution.unknown == 0` with
   all-zeros = missing manifest (a measured-but-regionless content puts
   the steward count in `unknown`).
2. **Distribution-plane decision (p2p-design-gate REQUIRED before
   building):** who authors manifests for bulk-seeded content — the
   seed/import path, or manifest-on-first-stock heal. This is what makes
   the demo's numbers real; junction/commitments/regions gaps behind it
   remain workstream D (the matrix feature's @wip rows are its
   acceptance gate).

## Standing rails (sharpened by this night's scars)

- **Push lease:** `.claude/data/push-lease.json` expires
  **2026-06-12T12:00:00Z** — after that, commit-only; AskUserQuestion to
  renew. Scope: `git push origin dev` only, local gates green first.
- **Single-dispatcher, verified EVERY push.** Check
  `elohim-orchestrator/dev lastBuild building:false` immediately before
  each push (one slip this night pushed into a building #1229 — no
  damage, but the mutual-abort risk is real). Verify the run SPAWNS after.
- **Fire-and-forget DNA races edge — it bit twice tonight** (#1062/#1325
  and #1065/#1327): the orchestrator dispatches holochain fire-and-forget
  and edge at Level-1 in the same run, so edge ships the PRE-fix happ.
  Sequence: DNA wave → wait terminal → edge wave (`[build:edge]`).
- **Orchestrator pod eviction loses builds silently** (#1226
  taint-evicted; app #1533 ABORTED; baselines advanced optimistically) —
  after ANY failed/evicted wave, force tags (`[build:app,edge]` comma
  syntax).
- Genesis runs read **UNSTABLE as the standing shape** — the frozen
  measure script is the judge, not the build color.
- Container: pool-slot fingerprint ENOENT → `/tmp` target dirs for
  native cargo; plain cargo in DNA workspaces (WASM RUSTFLAGS ambient);
  no nextest here. Loki: storage container=`elohim-node`; fetcher init
  container=`happ-fetcher`; the no-drift path is silent BY DESIGN —
  silence after "App already installed" means no drift, not no check.
- a2o: parse-validate any feature edit (`@cucumber/gherkin` snippet in
  genesis/a2o) — one parse error aborts the WHOLE E2E run.
- Never kubectl from dev; destructive pod-op drills are operator-ratified
  and live as bash in `genesis/scripts/ci/` (CPS cap).

## Done

- Gate-level: measure script prints **0**; then 3 consecutive green
  genesis runs (stability gate) → Workstream A done.
- Surface-level: `resilience.peer-statuses` lights; the EPR tooltip shows
  live/known peers instead of bare zeros; the demo content reads
  `unmeasured` honestly until the distribution-plane decision lands.
- Arc-level (unchanged): published content survives multi-peer loss —
  wipe drill green, failover green, aggregates truthful, crutch deleted,
  resiliency scenarios passing every genesis build.
