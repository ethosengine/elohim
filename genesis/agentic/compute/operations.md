# Delegated feedback sweettest operation

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
the requester as recipient, and `sweettest-feedback` as scope.

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
requires an immutable worker image digest, Adam's actual agent key and an
operator-provisioned `<Adam storage resource name>-compute-local` Secret with
key `token`. The projection adds a sibling container to Adam's storage pod,
with its own 2 CPU/4 GiB request, 8 CPU/8 GiB limit and separate 24 GiB PVC.
It preserves storage/conductor allocations and mounts no service-account token.
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
`genesis/a2o/features/dataplane/delegated-sweettest.feature`.
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
