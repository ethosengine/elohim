# Bounded delivery shifts

The shift launcher carries one authorized task through a retained worktree, durable
evidence, independent native acceptance, and publication. Claude and Codex share
the launcher and its private `.claude/shifts/<id>/` state. The deadline is an
absolute UTC time, with a maximum objective budget of 2,400 minutes. Downtime counts.

```bash
just dev shift start .claude/shifts/my-task/declaration.json --runtime codex
just dev shift status my-task
just dev shift resume my-task
just dev shift resume my-task --runtime claude
just dev shift stop my-task
just dev shift recover
```

Run these from the owning isolated Git worktree. `start` takes JSON or YAML. Copy
`shift-contract.example.json` and fill its placeholders from the user's task and
the existing native flow records. The objective is separately validated against
`.claude/schemas/objective.schema.json`; its `measure.run` must print one finite
number and run in this worktree. Keep all private input files under the owning
worktree. Never embed runtime credentials in declarations or evidence.

The original authorization file needs `build: true`, `source` identifying the
user instruction, and the same `deadline` as the declaration. `landing: true` is
separate authority for gated landing and push. Household restarts additionally
need `household_lifecycle: true`. A separate revocation file starts with
`{"revoked": false}`; changing it to true stops execution. Do not rewrite original
authority to simulate a fresh grant. A covenant variance grants neither build nor
publication authority.

`intent`, `commitment`, `scope_cid`, and `habit` name the native claim made with
`epr flow claim --serves`. Preserve the intent's existing scope CID. A sprint ID,
branch name or conversation ID is never a substitute. The journal may evolve;
the objective, execution contract and original authorization are frozen.

List private executable dependencies in `input_files`, including the objective
helper, household/CI wrappers and their private imports. `start` freezes their
SHA-256 hashes. Missing or changed bytes refuse readiness, dispatch, recovery and
closure. Direct private command targets must be listed; produced reports and
proof files are evidence outputs, not executable inputs. Tracked source scripts
remain bound to the verified Git revision.

Readiness requires an isolated worktree, installed tools and ark, Linux user/PID
namespace support, actual Claude/Codex authentication, successful tool canaries
under their normal approval/sandbox policies, available resource leases,
manifest-selected gates, and a completed numeric baseline. No permission bypass
flags are used. Each failed initialization retains its own evidence directory.
Retrying `start` with the identical input is allowed only before publication of
execution state. Changed input, corrupt state or an expired deadline refuses.

The exact five generated SessionStart observer ledgers in
`shift-observer-paths.json` may remain modified and unstaged. Their set is pinned
in the contract. Their bytes are preserved; they are never staged with source.
All other source changes make readiness unclean, as do staged observer edits.
This exception is needed because authenticated runtime canaries execute the
repository's normal observation hooks. It does not waive any source gate.

Each operation has a frozen ID and arguments. Available kinds are:

| Kind | Declaration and result |
| --- | --- |
| `agent` | `prompt`, optional `readonly`; saved exact session ID and JSONL transcript |
| `measure` | `argv`; or `objective: true` to run the frozen objective oracle; optional `artifact` records built binary SHA-256 |
| `gate` | `project` from manifest-selected `verification`; runs `just gate <project>` |
| `household` | `argv`, `oracle`, `proof`, and `binary` linking a build operation and host PID record |
| `lifecycle` | Authorized `just mesh storage-restart <peers>` only; deterministic host execution |
| `native` | Frozen `epr flow ...` command; `commit_revision: true` on fulfillment appends the exact dispatch SHA as `--commit` |
| `commit` | `message`; stages explicit paths within frozen scope, refuses unrelated index contents |
| `land`, `push` | Current native acceptance, exact-revision evidence, separate authority, clean landing checkout on `dev` |
| `ci` | Frozen read-only inspection command and structured `proof` for declared `downstream` pipelines |

Declare `resource: cargo|mesh|disk-heavy` on operations that use those leases,
and `measurement_resource` for the readiness baseline. Baseline execution claims
and releases that lease on the host, including failed readiness attempts.
Nested commands in the PID namespace delegate checks to the guardian's live host
lease; they cannot renew its TTL, release it, change class, or use another resource.
The household verify lease retains its original 1,800-second limit. The only
persisted workload environment overrides are `MESH_DIR`, `MESH_TRANSPORT_BACKEND`,
and `CARGO_TARGET_POOL_FAMILY`; runtime authentication remains in the runtime's own
configuration. Pin `MESH_DIR` when using a retained household outside this worktree.

Household and CI commands receive `SHIFT_CONTRACT_SHA256`, `SHIFT_OPERATION`,
`SHIFT_ATTEMPT`, `SHIFT_PREPARED_AT`, `SHIFT_REVISION`, and `SHIFT_PROOF_PATH`.
Write the declared proof as JSON with matching `contract_sha256`, `operation`,
integer `attempt`, `revision`, `success: true`, current numeric UTC `observed_at`,
nonempty `observations`, and `evidence: [{path, sha256}]` pointing to retained raw
artifacts inside the worktree. The launcher snapshots the proof immutably and
checks those raw artifacts again at closure.

A household proof also names the exact declared `oracle` and
`scenarios: {passed: N, failed: 0, skipped: 0}`, where N is positive. Its `binary`
declaration names `build_operation` and the absolute `pid_file` written by the
mesh (`<host-pid> <start-ticks>`). The guardian checks the running executable
against the build receipt before and after the oracle. A CI proof has `builds`
with exactly one selected build per required pipeline: `pipeline`, `number`,
HTTPS `url`, `revision`, and `result: "SUCCESS"`. Conflicting builds, stale proof,
skipped scenarios and a command that merely exits zero do not establish delivery.

The guardian runs outside contained PID namespaces and owns the execution flock,
original deadline and operation children. Ark restarts the contained controller
under its bounded policy. Heartbeats are 15 seconds; a controller confirmed
stopped for 60 seconds is recovered. Output silence is not a stall. A controller
can disappear while a host-owned measurement finishes and persists its receipt.
Loss of the guardian kills contained execution, including detached descendants.
This containment does not change conductor supervision defaults.

`resume` uses the exact saved same-runtime session. A runtime switch creates a new
conversation over the same durable evidence. `stop` writes a persistent marker;
automatic recovery never clears it. `recover` scans Git's registered worktrees,
so there is no second registry. Legacy journals remain readable but cannot launch.
The devfile startup action runs this command. Existing Che workspaces must adopt
the updated configuration normally; command testing is not a verified host reboot.

Before retrying an interrupted operation, inspect existing task, build, commit,
receipt and native flow records. A prepared operation is different from one whose
dispatch was persisted. Arbitrary shell effects may remain unresolved; the
launcher does not promise exactly-once execution. A failed operation retains its
evidence and an explicit frontier. Never delete state to make an uncertain act
look undispatched.

The launcher can reconcile an exact commit trailer, observed landing/push, or
native fulfillment whose original report, actor, commitment and commit were pinned
before dispatch. It records a new witness without replaying the act. Known failed
agent/gate operations and measures explicitly declaring `readonly: true` can retry
within the original objective's iteration ceiling per operation. Gates and such
measures refresh after a revision change. Each attempt keeps its own log and
receipt; a later uncertain dispatch cannot hide behind an older successful receipt.
Objective stability reads every attempt in completion order. A failed or invalid
sample resets the consecutive-success count; an explicitly read-only objective
can be measured again within its original attempt budget to establish stability.

Closure follows claim → immutable implementation report/fulfillment → technical
review → appointed independent acceptance → native reconciliation. The acceptor
runs or inspects the declared oracle at the exact revision. Later verification
creates new evidence and acceptance reports, with explicit supersession when
needed; it never rewrites original fulfillment. Current native acceptance is
rechecked even when the checkpoint says complete. Stable objective measurements,
all required gates, household proof, authorized publication and downstream CI
are required. A cross-runtime mutation rehearsal is a separate verification run;
it cannot fulfill, publish, or manage the shared household.

Run `just gate agentic-shift` for the launcher regression suite. It requires real
ark and namespace support and fails explicitly if either is unavailable. Process
tests use deterministic adapter executables; readiness canaries exercise the
authenticated real CLIs. Keep both forms of evidence distinct in sprint reports.
