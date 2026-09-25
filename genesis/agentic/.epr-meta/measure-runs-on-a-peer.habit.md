---
epr-habit-version: 1
id: measure-runs-on-a-peer
invariant: >
  A claimed measure-class run is executed, stored and attested by a peer within its
  declared envelope, and the dev berth is never held by a measure.
status: red
active: false
checks:
  - "a2o @concern:measure-runs-on-a-peer (genesis/a2o/features/compute/measure-on-a-peer.feature; default profile: cd genesis/a2o && npx cucumber-js --tags '@concern:measure-runs-on-a-peer')"
  - "python3 -m unittest genesis.agentic.berth_test (the --class measure refusal case: a measure-class claim on the dev berth is refused exit 3, naming just measure as the alternative — landing in parallel under lane D1)"
guard: >
  Regression risks: (1) a RUN_CLASSES vocabulary that drifts from `pipeline-registry.mjs:45`
  (`build|deploy|verify|measure|profile`) — the berth `--class` flag and this habit's checks
  must both read the same enum, never a locally-invented one; (2) a task envelope that grows a
  cluster object, URL or queue name to route a measure to a peer — R6 refuses this at the gate,
  and the habit must not green off a lane that smuggled one in; (3) never green this off rung H
  alone — rung H (household stand-in, jessica as provider) proves author -> provision -> run ->
  receipt -> attestation -> habit delta, but NOT offload; only rung A (adam on shem, dev berth
  free while a measure runs on the peer that holds the stores) proves the invariant this habit
  actually states.
refs:
  - "plan: /projects/.claude-config/plans/we-ran-into-a-cryptic-robin.md"
  - "tevah spec: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md §3.1 (berth offer as REA intent matched against under-held commitments) and §5.3 (defaultWritablePaths, stale against MESH_DIR — amend owed)"
  - "berth spec: genesis/docs/superpowers/specs/2026-09-03-workspace-berth-carrying-capacity-design.md (\"It does not gate\" :158-160 becomes false under D1 — amend owed)"
  - "stage spec: genesis/docs/superpowers/specs/2026-09-08-peer-executed-stage-design.md (the envelope D1's `just measure` rides; graduation trigger is a populated `reports/peer-stage/`)"
  - "sibling habit: elohim/elohim-storage/.epr-meta/operator-runtime-surface.habit.md (compute-chain deltas keep landing there; this atom owns the offload property alone)"
retire-when: >
  when measure-class runs have no dev-berth path at all.
---
DELTA 2026-09-25 (rung H — household stand-in, no offload proven; status stays RED): first evidence: a claimed measure-class run was executed, stored (durable copy + receipt) and its completion verified by the requester on a peer (jessica) within its declared envelope (task bound derived from the real cgroup ceiling; timeout 1500 s; ark death-witness recorded) — run 5 request uhCkka-xfWbXI3fMDOn0Ea6vVhBz4bsj-3xjoAt3gE_FQKV8d0uyx pass, run 6 request uhCkkIbIARyZIZHRvEQeRfLtyUEcxlN_JleGU1cBfhpCXmMqkKZUF pass with the listener-written DELTA on doorway-failover. The invariant's second clause (the dev berth is never held by a measure) is NOT proven: same host, one mesh, the orchestrator session held the verify lease throughout; berth refuses --class measure on the dev berth (45f0e6a9c) but offload needs rung A. The attestation leg is not exercised (no brit-build-ref here). The a2o scenarios of this habit are still pending stubs. Status stays red per R5/R8.

DELTA 2026-09-25: born RED by design — both `checks:` are nameable but neither is green yet.
The a2o concern's steps in `genesis/a2o/steps/compute/measure-on-a-peer.steps.ts` are `pending`-
returning stubs so the feature compiles without claiming evidence it does not have; `--class` on
`genesis/agentic/bin/berth` (and its `berth_test.py` case) is landing in parallel under lane D1 of
the same sprint plan, not built by this delta. The covenant prefers this red, checked commitment
over `unwired` now that both checks are nameable. Status flips only on rung H evidence first
(household stand-in, jessica as provider — proves the chain, not offload); any rung A claim
(adam on shem, the dev berth actually free while a measure runs elsewhere) is a separate, later
delta and is never inferred from rung H alone. A `matthew -> adam` `trusted-friend`/`trusted`
fixture edge was added the same delta in `genesis/data/humans/humans.json` so the relationship
this habit's eventual rung-A run rides is declared where relationships live (imagodei), per R7/R10
of the plan — no pricing shortcut is taken from it.
