# Delegated feedback sweettest operation

## Jenkins and the Adam fixture

Matthew is the accountable steward of this deployment. Adam is a test-fixture human
playing the provider role, and `agent:ci-runner-jenkins-matthew` is the delegated
service agent. The native grant and completion retain the actual provider/requester
keys. `runnerGrant.stewardOfRecord` records Matthew's deployment authorization; it is
not a separately notarized human-to-fixture stewardship relationship.

This delivery's milestone is native delegated CI: sponsored runner, provider grant,
bounded execution, authenticated completion and the verified report consumed by
Jenkins. The landed Ark task and berth contracts are pure interfaces. Connecting
their admission decisions to materialization and launch is a separate runtime
milestone; this worker does not establish placement-controller graduation.

The edge pipeline builds the source worker when `BUILD_COMPUTE_WORKER=true` or the
commit carries `[compute-worker:build]`. `compute-worker-image/image.json` records
the immutable worker digest, source commit, component digests and hApp hash. Activate
only that image in `genesis/orchestrator/data/adam-compute-worker.json`, with its
actual provider key, `privateMesh: true`, and matching `sourceCommit`. The declared
slice is 24 GiB / 8000m, one stage at a time; the runtime manifest accounts for it
alongside storage, conductor and proxy. It does not increase their individual caps.
These are declared limits until a live worker demonstrates its cgroup ceilings and
measured footprint, including its private mesh. A grant does not reserve resources.

Edge #1672 published the tools and fleet images but failed the final worker assembly:
storage and doorway implement `--help`, not `--version`. The assembly now checks all
six binaries with bounded `--help` calls. Recovery uses the recorded immutable inputs
in `genesis/scripts/ci/edge-1672-inputs.json` via `[compute-worker:package]`, paired
with `[deploy-only] [build:edge]` to deploy the already-published sponsor fix, or
`[edge:validate-only]` to package alone. It does not rebuild the fork. The helper
verifies the embedded tools checkout and records both its original `sourceCommit`
and the new `assemblyCommit`; activation uses the original source commit. An automatic
validation sibling still measures normally and does not repeat packaging.

Bootstrap retains its private household under `/var/lib/compute/mesh` and pins its
checkout under `/var/lib/compute/repos/<sourceCommit>`. Its storage, conductor,
P2P, metrics and Mongo port bases are 18090, 14444, 19701, 19464 and 37017.
Doorways use 18888/18889; their health ports are 18079/18089, and relay uses 13340.
The mesh launcher supports `MESH_STORAGE_BASE_PORT`, `MESH_CONDUCTOR_BASE_PORT`,
`MESH_P2P_BASE_PORT` and `MESH_METRICS_BASE_PORT`; ordinary household defaults remain
8090, 4444, 9701 and 9464. Mongo's Unix socket lives with its persistent database,
so the worker's read-only root filesystem requires no writable `/tmp`.

G1 is the sponsor's hosted-cell grant. G2 is issued on the provider's loopback API
from the explicit `runnerGrant` declaration: actual provider and runner public keys,
fixed issuance/validity timestamps, `measure-stage` scope and finite bounds.
Bootstrap verifies the live provider and native response, and keeps
`/var/lib/compute/runner-grant.json`. It refuses on mismatched parties or invalid
authority. It never invents keys or grants, and a restart reuses the fixed declaration.

Enable `genesis/orchestrator/data/jenkins-peer-stage.json` only after recording the
native G2 action hash and image digest. The initial profile permits exactly
`features/dataplane/peer-mesh.feature`; it casts no hosted people. The runner compares
source identity, exact scenario inventory, receipt bindings and log hashes before
publishing the real Cucumber report. Process readiness does not prove convergence:
an unconverged mesh produces a failed BDD result. A local run proves the household
lane; only an Adam completion and Jenkins report prove offload. Before broadening
the profile, measure completed scenarios, elapsed time and footprint against the
additional capacity. For a DNA change, also require matching successful DNA-build
evidence for the packaged hApp.
Keep refusal and retry evidence beside that successful run: another runner cannot
spend the grant, withdrawal refuses the next launch, and an indeterminate launch
response cannot create a duplicate run. Withdrawal does not promise cancellation
of an already-running stage. Peer evidence does not replace a household T2 receipt.

After activation, one fresh commit carrying both `[build:genesis]` and
`[adam-authority:probe]` runs the bounded authority probe. Fill the reviewed peer
profile's provider, image, paths and capacity first; it may remain disabled while
the probe reads the production grant hash from live activation evidence. A separate
service agent tests disposable wrong-recipient and already-withdrawn grants against
Adam's acceptance and launch endpoints. The probe verifies no acceptance, completion
or admission event, withdraws its disposable grants, and checks that the production
grant remains active. Jenkins archives only `adam-authority-probe.json` on success.
This proves those refusal cases, not revocation after acceptance, cancellation or
live lost-response recovery. The latter still needs its own execution evidence.

The probe has a 15-minute deadline and no automatic retry. Its private journal lives
only in the current Jenkins pod; do not replay the tagged build or archive that
journal. If the pod disappears, remaining temporary grants expire within 15 minutes
of issuance. Wait for that window before a new explicitly tagged attempt. Same-pod
recovery retains the original denial timestamps and rechecks task/admission state;
it cannot turn an expired grant or an old observation into a fresh refusal proof.

The workspace and Adam each talk only to their own loopback storage adapter.
Requests, acceptance and completion use their respective conductor identities;
discovery is DHT-backed. Input bytes enter the workspace BlobStore, and Adam's
local BlobStore retrieves them through the native blob protocols. Inputs and outputs use
transient chunks of at most 1 MiB, leased to the canonical task CID. No task contains a service
URL, cluster resource or queue name.

Build the `compute-executor` and `ark` native binaries using their declared pool
slots. Prepare a task matching `elohim/sdk/schemas/v1/compute-task.schema.json`;
`compute-executor artifact FILE` supplies each exact input descriptor. The
prebuilt test binary must support the runtime DNA artifact directory override.
Its runtime image digest must match Adam's actual pinned execution image.

Configure the local adapter with `ELOHIM_COMPUTE_LOCAL_API=1` and a local
`ELOHIM_COMPUTE_LOCAL_TOKEN` of at least 32 bytes. Export the same token into the
workspace wrapper, together with `COMPUTE_PERFORMER` (its own conductor key),
`COMPUTE_GRANT_ACTION`, and optionally `COMPUTE_EXECUTOR`. This is a local
capability, not a cross-peer bearer credential. The Mishpat coordinator must include signed link authors in its lifecycle
projection; an older coordinator fails closed for compute grants. The grant names Adam as provider,
the requester as recipient, and `sweettest-feedback` as scope (a peer-executed a2o stage
needs `measure-stage` instead — see "The grant a stage spends" below).

On Adam's own local adapter, his operator can explicitly issue the native
grant with `node /opt/compute/workspace.mjs grant grant.json` (or the repository
copy of that script). The file names the requester's actual Holochain agent
key and explicit bounds; for example:

```json
{
  "recipient": "REQUESTER_HOLOCHAIN_AGENT_KEY",
  "validFrom": "2026-09-07T00:00:00Z",
  "validUntil": "2026-09-12T00:00:00Z",
  "bounds": {
    "epr_scope": ["*"],
    "reach_ceiling": "commons",
    "rate_per_hour": 2,
    "rotation_ttl_days": 5
  }
}
```

Choose the dates before issuing. The explicit `*` permits this requester to
submit any artifact for the single feedback-suite capability; replace it with
task CIDs for narrower permission. The command persists `issuedAt` for retry
stability and returns `grantActionHash`, which the requester sets as
`COMPUTE_GRANT_ACTION`. Native grant issuance is separate from task submission.
It never silently renews or reactivates a withdrawn grant.

```
node genesis/agentic/compute/workspace.mjs submit task.json feedback_signal lamad.dna
node genesis/agentic/compute/workspace.mjs start
node genesis/agentic/compute/workspace.mjs poll
```

Submission writes a stable invocation nonce and chunk descriptors back to
`task.json`. Plain resubmission recovers the same native request if its response
was lost. Add `--new-run` to intentionally execute the identical suite again;
this replaces the nonce before submission.

This file-backed binding and the accountable-correction contract §8's operation
id, immutable evidence and storage outbox are inventoried as one act class:
**submission binding — one intended act survives a lost response**. The
[reimplementation plan's slice-2 handoff](../../docs/content/elohim-protocol/architecture/2026-09-06-ai-stewarded-commons-reimplementation-plan.md#7-the-implementation-sequence-and-what-gets-deleted)
records both shapes for convergence before another mechanism is introduced. Read
`sprint/accountable-correction` first: its `src/api/feedback_operations.rs`,
`src/db/feedback_operations.rs` and `2026-09-06-101000_feedback_operations`
migration under `elohim/elohim-storage/` already implement the correction outbox.

`start` detaches the listener from the foreground session. The inbox defaults to
`genesis/a2o/reports/compute`; set `COMPUTE_INBOX_ROOT` to another durable local
directory. `poll` recovers missed completion notifications. A listener excludes
another listener and recovers a lock whose PID no longer exists. A workspace
restart requires starting the listener again; the inbox and review delivery
state survive. Successful review deliveries are recorded per completion action
and adapter; failed deliveries retry. A crash between adapter completion and
recording its success may produce a duplicate read-only review.

Both `codex exec --sandbox read-only` and Claude print mode with only
`Read,Grep,Glob` are supported. `COMPUTE_REVIEW_ADAPTERS=codex` is the default;
select `claude`, or explicitly `codex,claude` for both. Review output is saved under
`reviews/` and expires with payload retention; delivery markers remain. Neither adapter updates habit
atoms or promotes the receipt into an attestation. The executable and logs are
untrusted evidence to the reviewer, not instructions.

Adam runs `node /opt/compute/worker.mjs` under a long-lived process supervisor.
It keeps an attempt ID before acceptance, rechecks authorization before launch,
and records the runtime receipt before completion publication. Publication
retries use the existing receipt. The runtime owns interrupted-attempt recovery
and payload cleanup. Input publication uses explicit custody until a terminal receipt or refusal;
a queued run cannot lose its executable to a post-completion retention clock.
Abandoned submissions consume the bounded attachment budget until explicitly
released. Completed input downloads are removed; compact receipts
remain. Retention defaults in the task schema are 24 hours or five runs,
whichever expires first; set task `retention` values before submission.
Local fetched logs and payload leases are released at expiry; compact receipts
and review delivery markers remain. The blob plane is content addressed, not encrypted:
another peer knowing a payload CID can request it. These are expiring developer
artifacts, not a private-data channel. Count-only and conjunctive policies use explicit
leases released by the running worker and workspace retention evaluators; keep those
supervisors running for timely cleanup. Storage enforces a separate byte budget.
Expired per-task lease markers remain for up to 30 days to prevent stale reads
from renewing them while another task retains identical bytes.

The optional packaging projection lives in
`genesis/orchestrator/data/adam-compute-worker.json` and starts disabled. Enabling
requires an immutable worker image digest and Adam's actual agent key. The
node-local token comes from `localTokenSource`: `storage` (the Adam declaration)
reuses the pool sponsorship token already rendered into storage; `per-render`
mints a fresh value on each render and writes it into both containers;
`secret` (the renderer's default and the hardening target) reads an
operator-provisioned `<Adam storage resource name>-compute-local` Secret with
key `token`. The projection adds a sibling container to Adam's storage pod
with a separate 24 GiB PVC.
Its request and limit both equal `config.slice`: currently 8 CPU / 24 GiB for
Adam's private mesh and one stage. It preserves storage/conductor allocations
and mounts no service-account token.
No task or runtime component knows this packaging exists. The image packaging
script is `scripts/ci/build-compute-worker.sh`; its base must already contain
Node and the compatible prebuilt sweettest runtime libraries. Enablement and
resource capacity review are operator-owned; this change does not deploy it.

The wrapper runs as root and the runtime drops the test guest to UID/GID 65534,
clears supplementary groups and prevents privilege escalation. The guest
inherits no local API token and only its scratch directory is writable. The
runtime output tree is traversable; credentials stay outside it. This is a
bounded prebuilt-test runner, not a general hostile-code hosting service.

For example, five-day age retention is
`{"maxAgeSeconds":432000,"maxRuns":null,"expireWhen":"either"}`;
retaining the last five completed runs is
`{"maxAgeSeconds":null,"maxRuns":5,"expireWhen":"either"}`.
Set both limits with `either` to expire when either is exceeded, or `both`
to wait until both are exceeded. Count is scoped to requester, provider,
project and task kind; signed request references break completion-time ties.
Cleanup is evaluated between runs and on idle polls.

The live acceptance probe is
`genesis/a2o/held/features/dataplane/delegated-sweettest.feature` — held by the scope
reconciler while `local-conductor` is unavailable; it returns to the plate when the cap does.
Set `COMPUTE_A2O_CONFIG` to a private JSON file with `task`, `binary`, `dna`
paths and an `env` map containing the workspace variables above. Optional
`unauthorizedTask`/`unauthorizedEnv` supply the refusal fixture;
`expiryRequestActionHash` identifies a completed run with an age limit of at
most 60 seconds. An absent fixture is pending, never a passing live result.
The provider's actual signed grant is required: the legacy
`/admin/seed/delegates-compute` projection-only fixture cannot authorize this
runner. Both the content-store and Mishpat coordinator changes must reach
the participating peers before this probe can pass.

The real two-peer iroh attachment test proves transfer and expiry, not
cross-peer authority. Issuing a grant on Adam's own loopback adapter leaves
its cross-peer use by the requester unmeasured. Run that authority leg with
slice 1's accountable-correction household mesh stations and use their shared
household receipt under `genesis/a2o/reports/`. Both owning habit atoms must
reference the same actual run receipt, with separate named station outcomes;
neither local grant tests nor correction-only success stand in for a passing
compute authority station. The pending run's receipt is not yet claimed here.

The household mesh is shared across worktrees: `/tmp/elohim-local-mesh` and
its ports are not isolated by the checkout. As of 2026-09-07 the slice-1
worktree is running the eight correction stations. Do not run a second
`just mesh start` from this tree. Queue the grant leg after that run finishes
and the mesh is handed over; reuse its household commissioning receipt.

Regression stations between execution and review are measured locally by
`compute.test.mjs` and `recovery.test.mjs`: revocation prevents a new launch
but does not erase an earned receipt; a lost publication response reconciles
before count cleanup; equal completion timestamps still retain exactly the
configured number of runs. The real-ark fixture harness is
`elohim/rakia/scripts/compute-runtime-smoke.py --executor PATH --ark PATH`.
It exercises supervision and expiry using a fixture executable; it does not
claim that the Holochain feedback suite or live peer delegation passed.

## Peer-executed a2o stage

A **stage** — one scoped a2o run: a feature file, a declared scenario inventory, and a
verdict — rides the exact envelope above, with no new field, no new HTTP route and no
DNA change. Full design:
`genesis/docs/superpowers/specs/2026-09-08-peer-executed-stage-design.md`.

`taskKind` stays `feedback_signal` (three independent pins — the executor, the DNA
coordinator, the schema const — all require the literal, and one of them is
operator-owned and unreadable here). The stage rides the fields that are already free:

- **`project` carries the stage identity**, the envelope's only free-form string, one of
  the six fields `verify_receipt` compares byte-for-byte, and already the retention
  bucket key. Grammar (split on the first `@`):
  `project = "a2o-stage:<stage-name>"` for a root stage, or
  `project = "a2o-stage:<stage-name>@after=<upstreamTaskCid>"` for a stage with one
  upstream. The `@after=` arm is reserved so a second stage never invents a second
  grammar; nothing schedules on it yet.
- **`expectedTests` is one entry per scenario**, namespaced
  `feedback_signal::a2o::<feature-dir>::<feature-slug>::<scenario-slug>`. The
  `feedback_signal::` prefix is a namespace token the pinned executor requires — not a
  claim that the scenario is a Holochain feedback sweettest. Slug rule, used verbatim on
  both the requester's builder and the generated guest script so the two computations
  cannot drift:
  `s.toLowerCase().replace(/[^a-z0-9]+/g,'-').replace(/^-+|-+$/g,'').slice(0,96).replace(/-+$/,'')`.
  One entry per scenario is load-bearing: a `passed` receipt is refused unless
  `observedTests` is non-empty and exactly equals `expectedTests`, so a provider cannot
  pass by running nothing and an upstream scenario addition changes the inventory rather
  than silently widening the claim.
- **`binary` is a generated, content-addressed shell script** speaking the executor's
  libtest protocol (`--list` / run); its CID *is* the pinned definition of what the peer
  will do. **`dna` carries the feature file's own bytes**, byte-identical — the slot is
  misnamed for this use (an operator item, not a blocker), but filling it this way pins
  the scenario *text*, content-addressed, so the runner can refuse when the repo copy has
  drifted from the pinned copy.
- **The cucumber report travels inside the `stdout.log` payload lease**, base64 on one
  line between sentinels, rather than as a fourth named payload — the allowed receipt log
  names are hardcoded to `stdout.log | stderr.log | witness.json` in two places this repo
  owns (`worker.mjs`, `workspace.mjs`) and possibly a third the operator owns; widening
  either allowlist alone fails at the provider with an unknown-name throw. Sentinels,
  single line each:

  ```
  -----BEGIN ELOHIM STAGE REPORT-----
  <base64 of the cucumber JSON, no wrapping>
  -----END ELOHIM STAGE REPORT-----
  ```

  Base64 rather than raw JSON so a scenario name containing the substring `test ` can
  never be mistaken for a libtest result line by any parser between the guest and the
  requester.
- **A stage that cannot meet its declared preconditions refuses; it does not report a
  red.** The generated runner asserts, before invoking cucumber, that every declared
  writable path is writable and that the repo's feature file hashes to the pinned
  `dna.sha256`. On failure it prints `STAGE-PRECONDITION-UNMET: <what>` and exits `2`, so
  the receipt records a `failed` run whose stdout names a missing capability rather than a
  false claim about the scenario under test.
- **The requester's verdict reads the decoded report, never the receipt's `status`.** A
  `passed` receipt is a provider claim; the stage is green only when the decoded cucumber
  report names the pinned feature file, contains exactly the declared scenarios, and every
  one of them passed. The substrate makes a provider's claim attributable, immutable,
  bounded and revocable — it does not make it true.

**The grant a stage spends: `scope: measure-stage`.** A stage is a measure-class run, a
different event class from the Holochain feedback sweettest, so it needs its own grant.
The provider's adapter derives the scope a task requires from the envelope, one way:
`project` starting with `a2o-stage:` requires `measure-stage`; anything else requires
`sweettest-feedback` (`required_scope`, `elohim/elohim-storage/src/api/compute_tasks.rs`).
A stage submitted under a `sweettest-feedback` grant is refused at acceptance and at
launch with `compute-grant-refused`, and a `measure-stage` grant never admits a feedback
sweettest. The requester never names the scope on the task — it only names the project;
the scope comes from the grant the provider signed. Issue it by adding the scope to the
grant file shown above:

```json
{
  "recipient": "REQUESTER_HOLOCHAIN_AGENT_KEY",
  "scope": "measure-stage",
  "validFrom": "2026-09-25T00:00:00Z",
  "validUntil": "2026-09-30T00:00:00Z",
  "bounds": {
    "epr_scope": ["*"],
    "reach_ceiling": "commons",
    "rate_per_hour": 2,
    "rotation_ttl_days": 5
  }
}
```

The launch-admission row the provider records (`compute-admission:<request>`) carries the
same event class in its `action` column (`measure-stage` for a stage).

**The in-kind price.** A stage is paid for in kind: no currency is issued, and the price
is exactly what the two parties declared, each term in the one place that already
enforces it. Nothing here is a Kubernetes object; the provider's host budget sits outside
the task.

| Term | Declared in | Enforced by |
|---|---|---|
| Event class | grant `scope: measure-stage` | provider adapter: the task's required scope must equal it |
| Rate | grant `bounds.rate_per_hour` | shared bounds validator at acceptance and launch, counting the provider's admission rows bounded by the grant |
| Which tasks | grant `bounds.epr_scope` (task CIDs, or `*`) | shared bounds validator |
| Duration of the offer | grant `validFrom` / `validUntil`, `bounds.rotation_ttl_days` | shared bounds validator; a withdrawal is terminal |
| CPU, memory, wall-clock | task envelope `resources{cpuMillis, memoryBytes, timeoutSeconds}` | the executor and `ark` on the provider |
| Bytes returned and how long they are kept | task envelope `resources.maxPayloadBytes`, `retention{…}` | the executor; storage refuses a receipt whose retention differs from the task |
| What the requester gives back | nothing in currency | the requester's own storage records the realized flow (below) |

**The requester's record of the flow.** When the requester reads its own completed task
(`GET /api/v1/compute/tasks/<requestActionHash>` on its loopback adapter), its storage
re-authenticates the grant the request pinned — signed by the provider, naming these two
parties and the required scope, activated by the provider and not withdrawn before the
run started — projects that grant into its own commitment table, and records one
`compute-fulfilled:<requestActionHash>` economic event bounded by the grant's entry hash
at the receipt's `completedAt`. The read then carries
`observed{verified, grantCid, scope, grantProvider, grantRecipient, fulfilledEventId}`; a
check that fails reports `observed{verified:false, refused}` and never fails the read.
Only the requester records this (the provider's rate window counts every row bounded by
the grant), a self-grant is never recorded, and the record is observe-and-audit only:
nothing prices, ranks or places on it.

**The capacity grant a provider must make.** The guest runs as UID 65534 and can write
only what the filesystem lets 65534 write. The stage declares exactly which paths it
needs, and the provider operator grants them explicitly (`chmod`, not an ACL — `setfacl`
is unavailable in this container). On a household mesh sharing one host and one checkout,
this landing's grant is:

```
chmod o+w <repo>/genesis/a2o/reports
chmod o+w /tmp/elohim-local-mesh/<peer>/runtime-config.toml   # for each peer the inner
                                                                # scenario writes/restores
```

On a real provider peer this grant is ordinary ownership — the peer runs its own checkout
and its own mesh, and nothing is chmod-ed at all. It is enumerated here because on a
shared host it is a real act with a real blast radius, and hiding it would misstate what
"the peer ran it" cost.

The stage builder is `genesis/agentic/compute/stage/build-stage-task.mjs` (template:
`stage-runner.template.sh`); the a2o binding is
`genesis/a2o/features/compute/peer-executed-stage.feature` with steps in
`genesis/a2o/steps/compute/peer-executed-stage.steps.ts` (its own fixture,
`PEER_STAGE_A2O_CONFIG`, mirroring `COMPUTE_A2O_CONFIG` above but never sharing state with
it). A run's receipt lands under `genesis/a2o/reports/peer-stage/<date>/` (gitignored,
durable), parallel to `genesis/a2o/reports/compute/`.

## The developer verb: `just measure`

`just measure <feature-path> [--on jessica|adam] [--gap <id>]` is the one command that sends a
measure-class run to a peer and returns immediately (`genesis/agentic/compute/measure.sh`). It
refuses before launch, naming the fix, when the provider's grant file, the executor, or a live
worker is absent; it takes ONE feature path (a tag expression is refused: one stage = one feature,
tags reserved), accepting either the `just test mesh` grammar (relative to `genesis/a2o`) or a
`genesis/a2o/`- or repo-root-prefixed path, refusing (exit 2, naming the resolved path) before
anything else prints when the file doesn't exist; it derives the task's cpu/memory/timeout bound
from this pod's own cgroup ceiling (`/proc/self/cgroup` + `cpu.max`/`memory.max`) rather than the
stage builder's understating defaults, unless `MEASURE_CPU_MILLIS`/`MEASURE_MEMORY_BYTES`/
`MEASURE_TIMEOUT_SECONDS` are set; it prints the chmod capacity grant a shared-host provider needs
and refuses until the paths are writable; then it builds the stage (`stage.json` carries the
requester's SUT so the provider's runner refuses on a drifted tree), submits with `--rung H`,
starts the detached listener, and prints the request action hash, task CID and inbox path. `just
measure grant --on jessica`
issues the `measure-stage` grant on jessica's own loopback; `just measure worker --on jessica` starts
her worker; `just measure status | poll` read the inbox; `just measure fixture --on jessica <feature>`
writes the `PEER_STAGE_A2O_CONFIG` JSON the a2o steps expect. `--on adam` refuses today, naming the
operator items (image digest, performer key, Secret, the declared worker slice — the pod already
renders over its envelope bound — and a checkout + mesh on adam's berth). When the listener recovers
a completion it verifies the chain on the requester's own task record (`observed.verified`, scope
`measure-stage`), decodes the report, writes one brit validate ref keyed by the SUT with the peer
chain in `summary.peer`, a `sprint-report-peer-stage-*.json`, fulfils the gap, and appends the DELTA
to the habit that owns the feature's `@concern:` — labelled `rung H (household stand-in, no offload
proven)` when the provider is a household peer. `MEASURE_DRY_RUN=1` prints everything and runs nothing.

## A sponsored runner, proven on the household

`genesis/a2o/features/compute/ci-runner-stewarded.feature` (steps
`genesis/a2o/steps/compute/ci-runner-stewarded.steps.ts`) is the household proof of the CI
runner's standing: a steward sponsors a runner through their doorway, providers lend it
capacity, and nobody's runner spends a promise made to someone else's. The cast is four
stewards, no one in two roles — matthew and james sponsor (doorways A and C), jessica and a
fourth peer provide. Staging it:

```
just mesh join-peer <fresh-name>      # the second provider; note the index it prints
MESH_DOORWAY_ENV_SET=$'DOORWAY_MAX_AGENTS_PER_CONDUCTOR=2\nPOOL_COMPUTE_PERFORMER=<james key>' \
  just mesh doorway-restart c        # james may now host a runner he sponsors
bash genesis/agentic/compute/runner-fixture.sh --second-provider <fresh-name>:<index>
cd genesis/a2o && CI_RUNNER_A2O_CONFIG=<printed path> \
  ELOHIM_CLUSTER_STATE_PATH_OVERRIDE=../manifests/cluster-state.act1-household.yaml \
  npx cucumber-js -p ci-runner
```

It runs from the requester's desk, not under `just test mesh`: a stage the runner submits
claims the household lease for its guest. `runner-fixture.sh` refuses, naming the cure, when
doorway C cannot sponsor or the fourth peer is absent.

Two things the steps do that a provider's worker would otherwise do. They ask a provider's
storage to accept each task (`POST …/tasks/{ref}/accept`, the worker's own first call) and
record its decline on a refusal; the admission decision is the provider storage's
(`api/compute_tasks.rs` `authorize`). And they name stages with `cid-executor.mjs`, the
`artifact` and `cid` halves of the compute-executor, because the executor itself is
unavailable (its source is lost: `genesis/data/timeline/backlog/rakia-executor-untracked-in-submodule-pin.md`).
Storage re-derives every CID it is handed, so a naming drift is refused, not accepted. The
scenario that reads a RUN stage's report back carries `@requires:compute-executor` and is
held until the executor returns.

What the runner's own surfaces refuse, live: a hosting promise its sponsor withdrew
(custody answers `hosted-cell-grant-withdrawn`), a capacity grant made to another runner or
withdrawn by its provider (`compute-grant-refused`), any identity-cell call (no capability on
`imagodei`), and stocking bytes at a reach more private than its hosting grant names
(`x-compute-reach` → `reach-narrower-than-hosting-grant`, with `grantReach`). A provider and a
sponsor read what they have promised with `GET /api/v1/compute/grants[?recipient=&scope=]` on
their own local compute API.
