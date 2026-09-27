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
DELTA 2026-09-27 round 2 (rung H, jessica; status stays RED — check 2 `berth_test` Ran 43 OK, check 1 still 23 `pending` stubs, and guard (3) forbids greening off rung H): the three blockers from round 1 are fixed with tests (53/53 across compute suites): stage-runner sets `process.exitCode` so a >72 KiB frame keeps its END sentinel; `measure.sh` refuses before submit a lane whose bound step code asserts `processControl` or reads another UID's `/proc/<pid>/{environ,exe,fd}` (epr-app-deliverability, epr-app-channel-isolation, name-routing, accountable-correction now refuse, with evidence lines); `$MESH_DIR/scenarios` joins the write-set only for a feature whose steps use owned-doorway-pair; the preflight names why a runtime-config.toml grant lapsed (the storage rewrites the file by temp file + rename(2) on each channel enrollment, and the new file is 0644). The first delegable lane `features/dataplane/doorway-fixture-readiness.feature` passed on the peer twice, with `mesh free` both times: request uhCkkKW4b8VSNqyi82JOBr8R7ke5mUao61_QAjwgi6juDsd6l3XY1 (taskCid bafyreihrcqnrwzmkfuk4szvbar5i4rzmgem2v2wgnb5nzqjbrdxae7jvme, submit 04:51:36Z → completed 04:52:35Z, guest 15 s, `reports/peer-stage/2026-09-27/a804597e…`) and, after the runner also routes the inner `sprint-report-household-*` to guest scratch so it no longer counts as a pre-push T2 receipt, request uhCkkg3o0iGcf5D4A83rjh6p4evdP-MzbzQYtWXdyKNj4ATO-9-0L (taskCid bafyreiagiesi2uqvwqisvasda5dhnkf3je24ybjecxy7yx5jv6t343olpe, 04:53:47Z → 04:54:16Z, guest 12 s, `…/8d44ae1e…`), both 2/2 observedTests. Saved about 0.2 min of berth hold per run: this lane is short, and the requester spends ~3 s. Offload itself still needs rung A.

DELTA 2026-09-27 (rung H, jessica; status stays RED): the chain delegated the serving-receipt lane `features/dataplane/epr-app-deliverability.feature` twice. Grant renewed (grantAction uhCkk5fhCAnTQXigqJDhXHjfe5mTzqkAXdOWwwEb-hRN_K8y0sihz), accepted and completed with receipts, and `mesh` stayed free throughout: requests uhCkkW3yiEb9DJYBwwmJnX8MrZFdbFeU3EirSlrzT4Om0OuqbYx-o and uhCkkEIXxktulKRA5IhiJP_NnTUskFRIzvq7EZk_TFu4pqqGlI695 → `reports/peer-stage/2026-09-27/{ee4f2d90…,c1835400…}`, status failed exit 101 after a 12 s guest run, 5/5 observedTests FAILED. What stopped them: (1) guest EACCES on `mkdtemp $MESH_DIR/scenarios/` — the write-set measure.sh preflights does not list this path; (2) guest EACCES on `readlink /proc/<root mongod>/exe` — the feature's owned-doorway-pair fixture asserts processControl=true, so it cannot be delegated to a UID-separated guest; (3) the stage-runner frame is cut at 72 KiB (`process.exit` right after a large stdout write), so the listener reports "did not carry a framed ELOHIM STAGE REPORT" even for a passing run. The local run took about 4 min; the peer saved 0 min.

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
