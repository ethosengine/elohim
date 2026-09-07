---
title: "The first Jenkins stage executed by a peer — an a2o feature run as a delegated compute task, attested by the provider"
id: peer-executed-stage-design
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: "Draft→Active when a Sonnet agent lands §8's tasks and the two-run experiment of §7 produces a receipt under genesis/a2o/reports/peer-stage/<date>/ carrying task CID, payload CID, attestation hash, provider agent and wall-clocks; Active→Canonical when the adversarial check of §6.4 has been run by an Opus reviewer and the a2o scenario of §9 passes on the household mesh with matthew as requester and jessica as provider, with the DELTA written on the operator-runtime-surface habit atom"
actor: agent:rust-architect@fable-5.1
written: 2026-09-08
domain: T10
habits: [operator-runtime-surface]
boundary: "Design only. This document decides how ONE existing envelope carries a CI stage, how a household provider peer executes it, and how the requester reads the result back and finds the attestation. It adds no envelope field, no HTTP route, no DHT entry type, no zome change and no change to the operator-owned rakia executor. It does not design a stage DAG scheduler, does not move the compute worker onto Adam, and does not decide whether the storage cargo gate becomes a compute task."
cites:
  - genesis/docs/superpowers/plans/2026-09-08-sprint-velocity-quiescence-holochain-close.md
  - genesis/docs/content/elohim-protocol/architecture/2026-09-06-holons-are-spaces-how-we-use-holochain.md
  - genesis/docs/superpowers/specs/2026-09-08-cell-qualified-projection-and-sync-contract.md
  - genesis/agentic/compute/operations.md
  - elohim/elohim-storage/.epr-meta/operator-runtime-surface.habit.md
  - genesis/data/timeline/backlog/rakia-executor-untracked-in-submodule-pin.md
  - genesis/data/timeline/backlog/compute-executor-cid-refuses-null-retention-field.md
---

# The first Jenkins stage executed by a peer

## 1. Goal

Run one CI stage on a peer instead of on a build server, and end up with evidence a
stranger could check.

Concretely: matthew (requester) asks jessica (provider, a household mesh peer) to run
`genesis/a2o/features/dataplane/federation-version-convergence.feature` — one scenario,
`@requires:multi-node`, satisfied by the household mesh — as a delegated compute task.
jessica runs it on her own substrate, returns the cucumber report as a payload lease,
and authors a signed completion on the DHT. matthew reads the report back and the
attestation the grant flow already mints.

Nothing new is built at the protocol layer. The whole point is that the delegated-compute
primitive measured on 2026-09-07 (jessica as provider, `delegated-sweettest.feature`
3 scenarios / 20 steps passed — see the DELTA on
`elohim/elohim-storage/.epr-meta/operator-runtime-surface.habit.md`) already carries a
CI stage without modification, if the stage is shaped to fit it.

**Vocabulary.** A **stage** is one scoped a2o run: a feature file, a declared scenario
inventory, and a verdict. The **envelope** is the JSON task object defined by
`elohim/sdk/schemas/v1/compute-task.schema.json`; its **task CID** is the DAG-CBOR CID of
that object and is the stage's identity. The **binary** and the **dna** are the envelope's
two content-addressed input artifacts. The **receipt** is the provider-authored JSON that
binds the run to the task; the **attestation** is the signed DHT action that carries it.

---

## 2. What is actually pinned (measured, not assumed)

Three components sit between this design and the substrate, and two of them are frozen.

**The rakia executor is operator-owned and its source is not in this tree.**
`elohim/rakia/` at the current submodule pin contains no `rakia-executor/` directory
(`genesis/data/timeline/backlog/rakia-executor-untracked-in-submodule-pin.md`). The only
artifact available is the built binary at
`/projects/.cargo-target-pool/family/dev/elohim__rakia/dev/debug/compute-executor`.
Every constraint below was measured against that binary on 2026-09-08. **This design
requires no change to it.**

Measured constraints:

| Probe | Result | Consequence |
|---|---|---|
| `compute-executor run --task <task-with-extra-field> …` | `unknown field 'stage', expected one of 'invocation', 'schemaVersion', 'taskKind', 'requester', 'provider', 'project', 'runtimeImage', 'binary', 'dna', 'expectedTests', 'resources', 'retention'` | `contract::Task` is `deny_unknown_fields`. **Any new envelope field breaks execution.** |
| `compute-executor cid <task-with-extra-field>` | succeeds | `cid` runs over `serde_json::Value` and tolerates what `run` refuses — a new field would pass submission and fail at launch. Do not be reassured by `cid`. |
| `compute-executor run --task <taskKind: a2o_stage>` | `unsupported task or empty/non-feedback inventory` | `taskKind` is pinned to `feedback_signal`. |
| `compute-executor run --task <expectedTests without "feedback_signal">` | `unsupported task or empty/non-feedback inventory` | every `expectedTests` entry must contain the literal `feedback_signal`. |
| `compute-executor run --task <project: "a2o-stage:…">` | advances to `runtime image identity unavailable or mismatched` | **`project` is free-form** and is the only free string in the envelope. |

`strings` on the same binary shows the guest contract: `--list`, `terse`,
`--include-ignored`, `--nocapture`, `--test-threads=1`, `inventory stdout missing`,
`inventory.log`, `SWEETTEST_DNA_DIR`, `lamad.dna`, `COMPUTE_RUNTIME_IMAGE`,
`COMPUTE_RESULT_BYTES`, `stdout.log`, `stderr.log`, `witness.json`, `outcome.json`,
`env_scrub`, `/usr/bin:/bin`, `memory.max`, `cpu.max`,
`actual cgroup ceiling exceeds offered task bound`. **The guest is a libtest-protocol
executable**, run twice: once for inventory (`--list --format terse`), once for
execution.

**The DNA coordinator also pins `taskKind`.**
`elohim/holochain/dna/elohim/zomes/content_store/src/compute_task.rs:198-205` refuses any
envelope whose `taskKind != "feedback_signal"`. That is coordinator-only (DNA-hash
neutral, hot-swappable) but it is a second independent pin, and the plan's write-set for
T10 does not include the DNA.

**The schema agrees with both.** `elohim/sdk/schemas/v1/compute-task.schema.json:5`
(`additionalProperties: false`), `:10-12` (`taskKind` const), `:127-135` (`expectedTests`
items must match `feedback_signal`).

Two live facts about the mesh this stage runs on, probed 2026-09-08:

- The 3-peer household mesh is up from this worktree — matthew `:8090`, jessica `:8091`,
  james `:8092`, doorway A `:8888`, doorway B `:8889`.
- `GET http://127.0.0.1:8091/api/v1/compute/tasks` answers
  `503 {"error":"local compute API disabled"}` — `ELOHIM_COMPUTE_LOCAL_API` is **not**
  set on the running peers, and no `worker.mjs` is running. Both must be started
  (§8, task 5).

---

## 3. Decisions

Numbered, each with the evidence that forced it.

**D1 — `taskKind` stays `feedback_signal`. The stage does not get its own kind.**
Three independent pins (executor, DNA coordinator `compute_task.rs:200`, schema const)
would all have to move together, and one of them is operator-owned and unreadable here.
The stage identity lives in fields that are already free.

**D2 — the stage name and the upstream-task CID ride `project`.**
`project` is the envelope's only free-form string (measured, §2), it is inside the CID
preimage, it is one of the six fields `verify_receipt` compares byte-for-byte between
receipt and envelope
(`elohim/elohim-storage/src/api/compute_tasks.rs:108-121`), and the retention count is
already scoped by `(requester, provider, project, taskKind)`
(`genesis/agentic/compute/operations.md:128-129`) — so each stage gets its own retention
bucket for free. Grammar, one line, parsed by splitting on the first `@`:

```
project = "a2o-stage:<stage-name>"                        # a root stage
project = "a2o-stage:<stage-name>@after=<upstreamTaskCid>" # a stage with one upstream
```

For this landing: `project = "a2o-stage:federation-version-convergence"`. The `@after=`
arm is specified now so a second stage does not invent a second grammar; it is not
exercised by this landing.

**D3 — no new envelope field, and the reason is not taste.**
`contract::Task` is `deny_unknown_fields` (measured). A field added to the schema would
pass `compute-executor cid`, pass submission, pass the DNA, and then fail at
`authorize-launch → run` with `unknown field`. That is the worst possible failure shape:
it looks like a runtime fault, not a contract violation. See §4 for the p2p-design-gate
answers.

**D4 — `binary` is a generated, content-addressed shell script that speaks the libtest
protocol.** The executor's contract is `--list` / run; a POSIX shell script satisfies it
exactly as well as a compiled test harness. The script is generated per stage by a
requester-side builder, so its CID *is* the pinned definition of what the peer will do.

**D5 — `dna` carries the feature file's own bytes.**
`dna` is required by the schema and materialized by the executor to `<scratch>/lamad.dna`
with `SWEETTEST_DNA_DIR` pointing at its directory. Filling it with the exact `.feature`
file being run means the receipt pins the scenario *text*, content-addressed, and the
runner can refuse when the repo copy has drifted from the pinned copy. The slot is
misnamed for this use; renaming it is a rakia change and therefore an operator item
(§10), not a blocker.

**D6 — `expectedTests` is one entry per scenario, namespaced with the required token.**

```
feedback_signal::a2o::<feature-dir>::<feature-slug>::<scenario-slug>
```

For this landing, exactly one entry:

```
feedback_signal::a2o::dataplane::federation-version-convergence::two-doorways-that-disagree-about-a-page-converge-on-the-elected-version-without-anyone-re-upload
```

The `feedback_signal::` prefix is a **namespace token the pinned executor requires**
(measured, §2) — it is not a claim that this scenario is a Holochain feedback sweettest,
and the spec says so out loud rather than letting a future reader infer it. Slug rule,
used verbatim on both sides so the two computations cannot drift:

```js
s.toLowerCase().replace(/[^a-z0-9]+/g,'-').replace(/^-+|-+$/g,'').slice(0,96).replace(/-+$/,'')
```

One entry per scenario is load-bearing: `verify_receipt` refuses a `passed` receipt whose
`observedTests` is empty or differs from `expectedTests`
(`compute_tasks.rs:91-107`), so a provider cannot pass by running nothing, and adding a
scenario upstream changes the inventory rather than silently widening the claim.

**D7 — the cucumber report travels inside the `stdout.log` payload lease, base64 on one
line between sentinels.** The allowed receipt log names are hardcoded to
`stdout.log | stderr.log | witness.json` in **two** places we own —
`genesis/agentic/compute/worker.mjs:179` and `genesis/agentic/compute/workspace.mjs:72` —
and possibly a third we do not (the executor's payload collector, unreadable). Widening
the allowlist on our side alone would fail at the provider with an unknown-name throw.
Framing the report inside stdout needs no allowlist change anywhere, and stdout is
already published as a chunked, digest-verified payload lease
(`worker.mjs:168-195`, `payloads.mjs:58-103`). Sentinels, single line each:

```
-----BEGIN ELOHIM STAGE REPORT-----
<base64 of the cucumber JSON, no wrapping>
-----END ELOHIM STAGE REPORT-----
```

Base64 rather than raw JSON so that a scenario name containing the substring `test ` can
never be mistaken for a libtest result line by any parser between the guest and the
requester.

**D8 — a stage that cannot meet its declared preconditions refuses; it does not report a
red.** The runner asserts, before invoking cucumber, that every declared writable path is
writable and that the repo's feature file hashes to the pinned `dna.sha256`. On failure
it prints `STAGE-PRECONDITION-UNMET: <what>` and exits `2`, so the receipt records a
`failed` run whose stdout names a missing capability rather than a false claim about
federation convergence. This is the difference between "the peer could not run your
stage" and "your stage is broken", and only the first is honest.

**D9 — the requester's verdict reads the report, never the receipt's `status`.**
See §6.3: the substrate makes a provider *accountable*, not *honest*. `status: "passed"`
is a provider claim. The stage is green only when the decoded cucumber report names the
pinned feature file, contains exactly the declared scenarios, and every one of them
passed. The a2o scenario asserts on the report.

**D10 — no changes to `genesis/a2o/steps/dataplane/federation-deploy.steps.ts`.**
The inner scenario writes `/tmp/elohim-local-mesh/<peer>/runtime-config.toml` and POSTs
`/admin/runtime-config/reload` (`federation-deploy.steps.ts:316-329`), restoring bytes in
`AfterAll`. That is the mechanism under measurement; the stage adapts to it by declaring
those paths writable, not by rewriting it.

---

## 4. P2P design gate

Run before deciding on any field. Two entities are in play.

### Entity: StageRecord (stage name + upstream task CID + declared scenario inventory)

- **Classification**: **Ephemeral (C)** — but the precise statement is stronger than "C":
  it is not a separate record at all. It is three field values *inside a
  content-addressed envelope that already exists*, whose CID is already the task's
  identity and is already re-verified on every read
  (`compute_tasks.rs:133-145` recomputes `content_cid(envelope) == taskCid`).
- **Justification**: delete it and it rebuilds from the envelope; the envelope rebuilds
  from `binary` + `dna` + the requester's task file. Nothing in it needs peer validation
  *separately* — it is validated as part of the CID it lives in.
- **Head-plane cost budget**: **zero.** No new entry, no new head, no new Kad record, no
  new election candidate, no contribution to the quiesce number — at seed or at 1 year.
  The existing REA `Commitment` the submission mints
  (`compute_task.rs:226-235`) is the only head, and it is one per *distinct* task CID,
  which the invocation nonce (§7) bounds to one per intended run.
- **Network stakes**: the stage must behave under a partitioned requester (recovered by
  `poll` from the DHT — measured in `delegated-sweettest.feature` scenario 1) and under a
  partitioned provider (attempt id preserved in the worker's `state.json`,
  `worker.mjs:45-53`; a terminal refusal or completion prevents relaunch,
  `compute_tasks.rs:273-278`). This is the D4 obligation from the sealed decisions:
  named successor authority is **the provider's own cell and no other**
  (`compute_task.rs:241-243`), and stated partition behavior is *recover, never
  re-execute*.
- **Content address strategy**: **Content-Derived (CID)** — identity is the task CID,
  computed by `compute-executor cid` over the envelope
  (`workspace.mjs:221`). Not a slug and not an agent-scoped composite: two requesters
  submitting the identical stage produce the identical CID, which is exactly the property
  the idempotency contract rests on.
- **Address justification**: the applicable head is declared by the requester, in the
  task file, as the invocation nonce — see §7. Nothing elects it; there is no election
  here, only equality.
- **Transport affinity**: **auto**. The stage's bytes move as compute payload chunks over
  the existing blob race (`http_compute_payloads.rs:93-152` — libp2p candidates plus
  iroh book entries, whichever answers). This design expresses no preference.
- **Source of truth**: **SQLite (operational)** for the payload leases
  (`compute_payload_store.rs` — "local operational custody leases (C), reconstructable
  from the task/receipt retention policy"); **Holochain DHT** for the request and the
  attestation, both of which already exist.
- **Integrity zome + DNA-hash class**: none touched. **DNA-hash-NEUTRAL** — and not
  merely neutral, *absent*: this design changes no zome at all, integrity or coordinator.
- **Coordinator zome**: `content_store::compute_task` — unchanged, already shipped, and
  already installed on the household mesh (habit DELTA 2026-09-07: "the installed DNAs
  already carry `compute_task`").
- **Projections**: none new. SQLite: the existing `economic_events` row minted by
  `record_operator_verb_event` (`compute_tasks.rs:412-421`); Automerge sync: **no** —
  compute payloads deliberately never finalize into permanent content
  (`http_compute_payloads.rs:1-2`).
- **HTTP route**: **none new.** Reads use `GET /api/v1/compute/tasks/{requestActionHash}`
  and `GET /api/v1/compute/payloads/{chunkCid}`, both already registered in
  `elohim/elohim-storage/src/http.rs:2123-2145`, both deliberately absent from
  `build_manifest()` and never doorway-proxied
  (`http.rs:2136-2137` comment). The `{id}` these accept is the **ActionHash** of the
  request and the **raw CID** (`bafk…`) of a payload chunk respectively.
- **Anti-pattern check**: caught and corrected — the first shape considered was a
  `stage: {name, upstreamTaskCid}` object on the envelope. `deny_unknown_fields`
  (measured) refuses it at launch, and the gate's own rule applies: a new field on a
  content-addressed envelope is a schema change to a notarized preimage, which is not a
  free act. Corrected to D2.

### Entity: StageResult (the cucumber report)

- **Classification**: **Ephemeral (C)**. It is reconstructable by re-running the stage,
  it expires with the payload retention policy, and the compact receipt outlives it by
  design (`delegated-sweettest.feature:32-36`, measured).
- **Head-plane cost budget**: zero. Payload leases are a local custody directory
  (`compute_payload_store.rs:1-4`), tombstoned for 30 days
  (`compute_payload_store.rs:15`) so a stale read cannot renew them.
- **Content address strategy**: **Content-Derived (CID)** — the `stdout.log` artifact CID
  in the receipt's `logs[]`, plus its per-chunk CIDs. `bytes` at 1 year: bounded by
  `resources.maxPayloadBytes` per run and by `retention.maxRuns` per stage.
- **Source of truth**: the provider's runtime output tree; SQLite lease is a copy.
- **HTTP route**: none new — `GET /api/v1/compute/payloads/{cid}` with headers
  `x-elohim-compute-token` and `x-compute-owner: <taskCid>`
  (`http_compute_payloads.rs:21-40`).
- **Anti-pattern check**: caught and corrected — the first shape considered was a fourth
  payload name, `cucumber.json`. Two allowlists we own plus one we cannot read
  (§3 D7) made that a cross-boundary change disguised as a one-line edit. Corrected to
  the sentinel framing.

### Design constraints discovered

1. `compute-executor cid` and `compute-executor run` **disagree** about what a valid task
   is. Anything that validates a task must validate it with `run`'s parser, not `cid`'s.
   This is the same class as the null-retention defect already filed
   (`compute-executor-cid-refuses-null-retention-field.md`): `main.rs` reads a
   `serde_json::Value` for `cid` and a typed `Task` for `run`.
2. `retention` must not carry an explicit JSON `null` — `compute-executor cid` dies with
   `Unit is not supported` (filed, above). Use
   `{"maxAgeSeconds":86400,"maxRuns":5,"expireWhen":"either"}`.
3. The guest environment is scrubbed to `PATH, LD_LIBRARY_PATH, COMPUTE_RUNTIME_IMAGE,
   TMPDIR` (`worker.mjs:16-22`) and the guest is dropped to UID/GID 65534 with
   supplementary groups cleared (`operations.md:117-121`). Every other input the stage
   needs must be baked into the content-addressed script.
4. `setfacl` is **not installed** in this container (probed). The capacity grant of §5.3
   is `chmod`, not an ACL.

---

## 5. The envelope

### 5.1 The task file

Written to `genesis/a2o/reports/peer-stage/<date>/task.json` (gitignored, durable).
Every value below is what the builder of §8 task 1 emits.

```json
{
  "schemaVersion": 1,
  "taskKind": "feedback_signal",
  "requester": "<matthew agent key, uhCAk…>",
  "provider": "<jessica agent key, uhCAk…>",
  "project": "a2o-stage:federation-version-convergence",
  "runtimeImage": "sha256:<64 hex>",
  "binary": { "cid": "…", "sha256": "…", "bytes": 0 },
  "dna":    { "cid": "…", "sha256": "…", "bytes": 12162 },
  "expectedTests": [
    "feedback_signal::a2o::dataplane::federation-version-convergence::two-doorways-that-disagree-about-a-page-converge-on-the-elected-version-without-anyone-re-upload"
  ],
  "resources": {
    "cpuMillis": 8000,
    "memoryBytes": 8589934592,
    "maxPayloadBytes": 8388608,
    "timeoutSeconds": 2400
  },
  "retention": { "maxAgeSeconds": 86400, "maxRuns": 5, "expireWhen": "either" },
  "invocation": "<uuid, written back by submit>"
}
```

Field notes that are not obvious:

- `binary.cid/sha256/bytes` and `dna.*` are produced by
  `compute-executor artifact <file>` and re-verified against the real bytes at publish
  time (`payloads.mjs:63-67`, "Input artifact mismatch") and again at materialize time on
  the provider (`payloads.mjs:126-139`, per-chunk digest plus whole-artifact digest).
- `runtimeImage` must equal the provider worker's `COMPUTE_RUNTIME_IMAGE`, or the
  executor answers `runtime image identity unavailable or mismatched` (measured). On a
  household peer there is no container image, so the honest value is the digest of the
  executor binary itself: `sha256:$(sha256sum <compute-executor> | cut -d' ' -f1)`.
- `resources.timeoutSeconds: 2400` — the inner scenario waits for an *organic* reconcile
  sweep; `PROJECTION_RECONCILE_SECS` defaults to 300s and the 2026-08-31 proof budgeted
  6 minutes (`federation-deploy.steps.ts:121-127`). The worker allows
  `timeoutSeconds * 1000 + 120000` (`worker.mjs:161`).
- `resources.cpuMillis` / `memoryBytes` must not be *under*-declared relative to the
  provider's real cgroup ceiling — the executor refuses with
  `actual cgroup ceiling exceeds offered task bound`. If that error appears, raise these
  to match `/sys/fs/cgroup/cpu.max` and `/sys/fs/cgroup/memory.max` on the provider.
- Envelope size: the whole object must stay under 64 KiB (`workspace.mjs:217-220`, and
  again `compute_task.rs:202`). At one scenario it is under 2 KiB.

### 5.2 The two artifacts

| Slot | File | What it is | Why |
|---|---|---|---|
| `binary` | `stage-runner.sh` | generated bash, ~5 KB, libtest protocol | D4 — the CID *is* the stage definition |
| `dna` | `federation-version-convergence.feature` | the feature file, byte-identical, 12162 bytes, `sha256:543c99377d3bd08df03e2d1aeccbcbade6d4701b4edbb470c3c50331d5870daf` at time of writing | D5 — the receipt pins the scenario text |

Submission publishes both into the requester's own BlobStore in ≤1 MiB chunks and leases
them to the task CID; the provider retrieves them through its own BlobStore's P2P heal
(`workspace.mjs:200-229`, `operations.md:3-8`). No source-workspace address enters the
task.

### 5.3 The capacity grant the provider must make

The guest runs as UID 65534 and can write only what the filesystem lets 65534 write. The
stage declares exactly which paths it needs, and the *provider operator* grants them.
On this household mesh, where provider and requester share one host and one checkout:

```
chmod o+w /projects/elohim/.claude/worktrees/sprint-0908/genesis/a2o/reports
chmod o+w /tmp/elohim-local-mesh/matthew/runtime-config.toml
chmod o+w /tmp/elohim-local-mesh/jessica/runtime-config.toml
chmod o+w /tmp/elohim-local-mesh/james/runtime-config.toml
```

Why each: the `just test mesh <scope>` recipe writes its generated scoped cucumber config
and its sprint report into `genesis/a2o/reports` (`justfile:126-147`, `:174-177`); the
inner scenario writes and restores the three peers' runtime-config files
(`federation-deploy.steps.ts:316-329`, `:354-359`). `setfacl` is unavailable (§4), so
this is `chmod`, and it is deliberately per-file rather than a recursive sweep over the
mesh tree — sqlite databases in `/tmp/elohim-local-mesh/<peer>/` must stay unwritable to
the guest.

On a real provider this grant is ordinary ownership: the peer runs its own checkout and
its own mesh, and nothing is chmod-ed at all. The grant is enumerated here because on a
shared host it is a real act with a real blast radius, and a design that hides it would
be lying about what "the peer ran it" cost.

---

## 6. Provider execution

### 6.1 What the provider has, and what it must not assume

**Has:** a scrubbed environment (`PATH`, `LD_LIBRARY_PATH`, `COMPUTE_RUNTIME_IMAGE`,
`TMPDIR` — `worker.mjs:16-22`); a writable scratch directory; the two materialized
artifacts; `SWEETTEST_DNA_DIR` pointing at the directory holding the staged `dna` copy;
cgroup v2 limits; UID/GID 65534 with no privilege escalation; read access to the shared
host filesystem, which on this mesh includes the repo worktree
(`/projects` is mode 2775, the worktree 2755, `node_modules/.bin/cucumber-js` 755 —
probed) and loopback access to the mesh's storage and doorway ports.

**Must not assume:** that `PATH` contains `/usr/local/bin` — the executor's own fallback
is `/usr/bin:/bin` (strings) and `node`, `just` live in `/usr/local/bin` on this host
(probed). That `HOME` exists or is writable. That it may write anywhere outside its
scratch and the paths of §5.3. That it holds the local compute token — it does not, by
construction (`operations.md:118-120`). That any repo path is stable — every path it uses
is baked into the content-addressed script, so a moved worktree changes the stage's CID,
which is correct.

### 6.2 The runner script

Generated by the builder; this is its shape, with the values for this landing filled in.
The `@@…@@` markers are what the builder substitutes.

```bash
#!/bin/bash
# Peer-executed a2o stage runner. Content-addressed: this file's CID IS the stage.
# Speaks the minimal libtest CLI the pinned rakia executor drives.
set -uo pipefail

STAGE_NAME='@@STAGE_NAME@@'                 # federation-version-convergence
REPO='@@REPO_ROOT@@'
FEATURE='@@FEATURE_REL@@'                   # features/dataplane/federation-version-convergence.feature
FEATURE_SHA256='@@FEATURE_SHA256@@'
TEST_PREFIX='@@TEST_PREFIX@@'               # feedback_signal::a2o::dataplane::federation-version-convergence
SCENARIOS=(@@SCENARIO_SLUGS@@)              # one quoted slug per declared scenario
WRITABLE=(@@WRITABLE_PATHS@@)               # absolute paths the stage must be able to write
NODE_BIN='@@NODE_BIN@@'                     # /usr/local/bin/node
JUST_BIN='@@JUST_BIN@@'                     # /usr/local/bin/just

export PATH="$(dirname "$NODE_BIN"):$(dirname "$JUST_BIN"):/usr/bin:/bin"
SCRATCH="${TMPDIR:-/tmp}"
export HOME="$SCRATCH"

# ---- inventory mode: any argv containing --list -------------------------------
for a in "$@"; do
  if [ "$a" = "--list" ]; then
    for s in "${SCENARIOS[@]}"; do printf '%s::%s: test\n' "$TEST_PREFIX" "$s"; done
    printf '\n%d tests, 0 benchmarks\n' "${#SCENARIOS[@]}"
    exit 0
  fi
done

refuse() { printf 'STAGE-PRECONDITION-UNMET: %s\n' "$1" >&2; exit 2; }

# ---- preconditions (D8) --------------------------------------------------------
[ -x "$NODE_BIN" ] || refuse "node not executable at $NODE_BIN"
[ -x "$JUST_BIN" ] || refuse "just not executable at $JUST_BIN"
[ -r "$REPO/genesis/a2o/$FEATURE" ] || refuse "feature unreadable: $REPO/genesis/a2o/$FEATURE"
have="$(sha256sum "$REPO/genesis/a2o/$FEATURE" | cut -d' ' -f1)"
[ "$have" = "$FEATURE_SHA256" ] || refuse "feature drifted: $have != $FEATURE_SHA256"
# The pinned copy travels in the dna slot; when the executor exposes it, cross-check.
if [ -n "${SWEETTEST_DNA_DIR:-}" ] && [ -r "$SWEETTEST_DNA_DIR/lamad.dna" ]; then
  staged="$(sha256sum "$SWEETTEST_DNA_DIR/lamad.dna" | cut -d' ' -f1)"
  [ "$staged" = "$FEATURE_SHA256" ] || refuse "staged feature != pinned feature"
fi
for p in "${WRITABLE[@]}"; do [ -w "$p" ] || refuse "not writable: $p"; done

# ---- run -----------------------------------------------------------------------
REPORT="$SCRATCH/cucumber.json"
rm -f "$REPORT"
printf 'running %d test\n' "${#SCENARIOS[@]}"
cd "$REPO" || refuse "cannot cd $REPO"
CUCUMBER_JSON_REPORT="$REPORT" "$JUST_BIN" test mesh "$FEATURE"
rc=$?
[ -s "$REPORT" ] || { printf 'STAGE-NO-REPORT: just test mesh exited %d and wrote no report\n' "$rc" >&2; exit 101; }

# ---- libtest result lines + the framed report ----------------------------------
"$NODE_BIN" -e '
const fs=require("fs");
const slug=s=>s.toLowerCase().replace(/[^a-z0-9]+/g,"-").replace(/^-+|-+$/g,"").slice(0,96).replace(/-+$/,"");
const prefix=process.argv[1], want=process.argv.slice(2);
const doc=JSON.parse(fs.readFileSync(process.env.REPORT,"utf8"));
const seen=new Map();
for(const f of doc) for(const e of (f.elements||[])){
  if(e.type!=="scenario") continue;
  const ok=(e.steps||[]).every(s=>s.result&&s.result.status==="passed");
  seen.set(slug(e.name), ok);
}
let failed=0;
for(const s of want){
  const v=seen.get(s);
  const verdict = v===true ? "ok" : "FAILED";
  if(v!==true) failed++;
  process.stdout.write(`test ${prefix}::${s} ... ${verdict}\n`);
}
process.stdout.write("\n-----BEGIN ELOHIM STAGE REPORT-----\n");
process.stdout.write(Buffer.from(fs.readFileSync(process.env.REPORT)).toString("base64")+"\n");
process.stdout.write("-----END ELOHIM STAGE REPORT-----\n");
process.stdout.write(`\ntest result: ${failed?"FAILED":"ok"}. ${want.length-failed} passed; ${failed} failed; 0 ignored; 0 measured; 0 filtered out\n`);
process.exit(failed?101:0);
' "$TEST_PREFIX" "${SCENARIOS[@]}"
node_rc=$?
if [ "$rc" -ne 0 ] || [ "$node_rc" -ne 0 ]; then exit 101; fi
exit 0
```

Two things this script does *not* do, deliberately. It does not transliterate the
`just test mesh` recipe's env derivation — it calls `just` so that the peer runs the same
gate command a developer runs, and the mesh env comes from the single source
(`hc-mesh.sh` sourced by the recipe, `justfile:56-60`). And it does not touch the report
path the recipe writes for the household lane; `CUCUMBER_JSON_REPORT` is honored as a
default by the recipe (`justfile:139`) so the stage's own copy lands in scratch while the
household's sprint report still lands where the lane expects it.

**Fallback if `chmod o+w genesis/a2o/reports` is refused:** replace the `just test mesh`
call with a direct scoped `cucumber-js` invocation — write the scoped config into
`$SCRATCH` and pass it as a path *relative to `genesis/a2o`* (cucumber does
`path.join(cwd, configFile)`, so `../../..`-style relatives work and absolutes are
silently mangled — `justfile:128-147`). The env block then has to be transliterated from
`justfile:56-100`. Take this only if the grant is refused; it duplicates a source of
truth.

### 6.3 What the substrate verifies, and what it does not

`elohim/elohim-storage/src/api/compute_tasks.rs` is the verified-performer path. In
order:

| Line | Check |
|---|---|
| `:285` | `ELOHIM_COMPUTE_LOCAL_API=1` or 503 — the surface is off by default |
| `:288`, `:21-36` | `x-elohim-compute-token` matches, constant-time over SHA-256, secret ≥32 bytes |
| `:310`, `:166-168` | `x-elohim-verified-performer` **equals this cell's own agent key** — a transport id or another agent's key is refused (`:500-505`) |
| `:379` | `task.provider == this cell's agent key`, else 403 `selected-runner-required` — only the selected runner may accept/complete/decline |
| `:391-400` → `:173-261` | the exact `grantActionHash` Record is fetched from mishpat (`:190-201`), its signature/author/entry-hash pins verified (`:207-215`), `action == "delegates-compute"`, `policy.provider == task.provider`, `policy.recipient == task.requester`, `policy.scope == "sweettest-feedback"` (`:218-226`), the shared bounds validator re-run on the *authenticated* payload with `DieselRateHistory` (`:241-254`), and provider-authored lifecycle links required `active` with no `revoked`/`cancelled`/`sunset` (`:256-259`, `:263-271`) |
| `:402`, `:273-278` | launch requires an accepted, attempt-matching, non-terminal request |
| `:412-421` | one-use launch admission recorded as an `economic_events` row `bounded_by` the grant cid |
| `:385` → `:62-131` | on complete: `receiptCid == content_cid(receipt)`; receipt binds taskCid / requestAction / grantAction / requester / provider; `status` ∈ six values; **a `passed` receipt requires non-empty `expectedTests == observedTests` and `exitCode == 0`** (`:91-107`); `binary`, `dna`, `runtimeImage`, `expectedTests`, `project`, `taskKind` byte-equal the immutable envelope (`:108-121`); `retention` equal (`:122-130`) |
| `:133-145` | on **every** read, `content_cid(envelope) == taskCid` is recomputed — a tampered envelope can never be served |

On the DNA side: the request must be an immutable `Create` authored by the requester
(`compute_task.rs:31-51`); any event not authored by the selected provider is ignored
(`:80-84`); `accept`/`complete`/`decline` refuse unless the caller is the selected
provider (`:241-243`); a completion must reference the accepted attempt (`:129-138`).

**What it does not check: that the provider executed anything.** A provider holding its
own local token can author a well-formed `passed` receipt — correct bindings, correct
CID, `observedTests == expectedTests` — without running the script, and both storage and
the zome will accept it. That is not a defect to fix here; it is the honest shape of a
delegation substrate. The substrate makes the claim **attributable, immutable, bounded
and revocable**; it does not make it true. What makes it checkable is the evidence the
run leaves — the framed cucumber report, `stderr.log`, and the ark witness — which is
exactly why D9 puts the verdict on the report and why the receipt goes to a read-only
reviewer (`workspace.mjs:93-136`).

### 6.4 The one adversarial check for the Opus reviewer

Do this on a throwaway `--new-run` task, never the receipt run.

1. On jessica, stop `worker.mjs` after the request is submitted and before acceptance.
2. Hand-author `forged-receipt.json`: copy `binary`, `dna`, `runtimeImage`,
   `expectedTests`, `project`, `taskKind`, `retention` from the envelope; set
   `schemaVersion: 1`, `status: "passed"`, `exitCode: 0`,
   `observedTests = expectedTests`, `logs: []`, and the five bindings
   (`taskCid`, `requestAction`, `grantAction`, `requester`, `provider`).
3. `compute-executor cid forged-receipt.json` → `receiptCid`.
4. Drive `accept` then `authorize-launch` then `complete` against jessica's loopback
   compute API with that receipt.

**Expected: the substrate accepts it.** That is the finding, and it is the point. Then
assert that the *requester side* rejects it: with `logs: []` there is no `stdout.log`
payload, so no framed report can be decoded, so the a2o step
*"the returned cucumber report names the pinned feature and every declared scenario"*
fails. If that step passes on a forged receipt, D9 was not implemented and the stage is
trusting a provider claim — which is the single failure this whole design is shaped to
prevent.

Secondary probes worth ten minutes each, if the reviewer has them: publish chunk bytes
that do not match `binary.sha256` and confirm the worker declines with `invalid-artifact`
(`worker.mjs:262-268`, digest checked at `payloads.mjs:126-139`); flip one character in
an `expectedTests` entry and confirm a `passed` receipt is refused at
`compute_tasks.rs:102`.

---

## 7. Result, attestation, and reading it back

### 7.1 The result path

1. The guest's stdout is captured by the executor into its `payload/stdout.log`.
2. The worker publishes each receipt log as a chunked payload leased to the task CID
   (`worker.mjs:168-195` → `payloads.mjs:58-103`), with lease seconds derived from the
   task's retention (`payloads.mjs:9-13` — with `maxAgeSeconds: 86400` and
   `expireWhen: "either"`, the lease is 86400s).
3. `workspace.mjs poll` on the requester materializes them to
   `$COMPUTE_INBOX_ROOT/logs/<sha256hex(completion.actionHash)>/stdout.log`
   (`workspace.mjs:64-92`), verifying every chunk digest and the whole-artifact digest
   (`payloads.mjs:126-139`).
4. The stage report is the base64 between the two sentinel lines.

Size bound: `resources.maxPayloadBytes` (8 MiB here) per artifact, 1 MiB per chunk
(`payloads.mjs:15`, `compute_payload_store.rs:14`, enforced again at
`http_compute_payloads.rs:47`). One scenario's cucumber JSON is tens of KB; base64 grows
it by a third.

TTL: 86400s from publication. After expiry the payload GET answers **410 Gone**
(`http_compute_payloads.rs:53-60`) while the compact receipt stays readable — the
property `delegated-sweettest.feature:32-36` already measures.

### 7.2 Exact reads

Let `REF` = `requestActionHash` (URL-encoded), `TASKCID` = the task CID,
`TOK` = `$ELOHIM_COMPUTE_LOCAL_TOKEN`.

**Status, receipt, and the attestation hash — on the requester (matthew, :8090):**

```
GET http://127.0.0.1:8090/api/v1/compute/tasks/<REF>
  x-elohim-verified-performer: <matthew agent key>
  x-elohim-compute-token: <TOK>
```

→ `.completion.actionHash` — **this is the attestation**: the DHT ActionHash of jessica's
signed REA `EconomicEvent` (`action: "work"`, id `compute:<REF>:complete`,
`fulfills: ["compute:<matthew>:<TASKCID>"]`, minted at `compute_task.rs:303-311`).
Also `.completion.receiptCid`, `.completion.receipt`, `.acceptance.actionHash`,
`.envelope`, `.grantActionHash`.

**The report payload — on the requester:**

```
GET http://127.0.0.1:8090/api/v1/compute/payloads/<chunkCid>
  x-elohim-compute-token: <TOK>
  x-compute-owner: <TASKCID>
```

for each `cid` in `.completion.receipt.logs[] | select(.name=="stdout.log") | .chunks[]`.
Or simply run `node genesis/agentic/compute/workspace.mjs poll` and read the
materialized file (path in §7.1 step 3).

**The provider-local admission event — on the provider (jessica, :8091):**

```
GET http://127.0.0.1:8091/api/v1/economic-events/compute-admission%3A<REF>
```

(route `/api/v1/economic-events/{id}`, `http.rs:15303`, handler
`api/economic_events.rs:82`). The row carries `action: "sweettest-feedback"`,
`provider` = jessica's agent key, `receiver` = matthew's agent key, and
`bounded_by` = the grant **commitment CID (entry hash)** — the column the rate window
reads (`db/economic_events.rs:782-784`).

So there are two attestations and they answer different questions. The DHT completion
answers *"jessica signed this result for this task under this grant"* and is the one the
receipt records. The local admission event answers *"jessica charged one use against
matthew's grant"* and is the one that makes the idempotency claim measurable.

---

## 8. The idempotency experiment

### The contract

The invocation nonce is part of the envelope, therefore part of the task CID.
`workspace.mjs:189` mints a nonce only when the task file has none **or** `--new-run` is
passed, and writes it back to the caller's file (`:216`). The DNA keys the submission by
`compute:{requester}:{task_cid}` and, when a matching request already exists with the
same envelope and grant, returns the existing status instead of creating a second one
(`compute_task.rs:206-225`).

> **Plain resubmission is idempotent: one intended act survives a lost response.
> A second execution is reachable only through `--new-run`, which mints a new nonce, a
> new task CID, and therefore a distinct request, execution and attestation.**

That is the contract. It is *not* "the same task twice runs twice with distinct nonces" —
the nonce does not change on its own.

### The exact two runs

**Run A — the stage.**

```
cd genesis/a2o && node ../agentic/compute/workspace.mjs submit \
  reports/peer-stage/<date>/task.json \
  reports/peer-stage/<date>/stage-runner.sh \
  ../a2o/features/dataplane/federation-version-convergence.feature
```

Record from stdout: `R1 = .requestActionHash`, `C1 = .taskCid`. Record from `task.json`
after the call: `N1 = .invocation`. Then `node ../agentic/compute/workspace.mjs poll`
until `.completion` appears; record `A1 = .completion.actionHash`,
`P1 = .completion.receipt.logs[stdout.log].cid`.

**Run B — the retry, identical command, no flag.**

Assert, in this order:

1. stdout `.requestActionHash == R1` and `.taskCid == C1`.
2. `task.json`'s `.invocation` is still `N1` (unchanged by the second submit).
3. `GET /api/v1/compute/tasks/R1` → `.completion.actionHash == A1` and
   `.completion.receiptCid` unchanged.
4. On jessica, `<COMPUTE_WORKER_ROOT>/jobs/<sha256hex(R1)>/state.json` has an unchanged
   mtime and still reads `completed: true` — the worker reconciled, it did not run
   (`worker.mjs:66-72`).
5. On jessica, `GET /api/v1/economic-events/compute-admission%3A<R1>` returns exactly one
   row, and no `compute-admission:` row exists for any other reference created by run B.
6. The provider's `runs/` tree contains one receipt for `R1`
   (`compute-executor receipt-path --root <worker root>/runs --request-action R1`).

**Verdict:** one execution, one completion action, one admission event, across two
submissions.

**Run C (optional, the other arm, ~10 minutes).** Same command with `--new-run` appended.
Assert `.taskCid != C1`, `.requestActionHash != R1`, and that a second admission event
and a second completion action appear. This proves the nonce is the *only* thing that
buys a second execution.

---

## 9. Implementation tasks for a Sonnet agent

Ordered. Do not reorder — task 5 must precede task 6, and task 6 must precede task 7.

Environment discipline for every task: run from
`/projects/elohim/.claude/worktrees/sprint-0908`; stage only the files you changed
(`git add <explicit paths>`, never `-A`); never `git stash`/`checkout`/`reset` — the tree
is shared. No cargo builds are needed anywhere in this list.

---

**Task 1 — the stage builder.**

- **Files (new):** `genesis/agentic/compute/stage/build-stage-task.mjs`,
  `genesis/agentic/compute/stage/stage-runner.template.sh`
- **Do:** a Node script `build-stage-task.mjs --feature <path-relative-to-genesis/a2o>
  --requester <key> --provider <key> --out <dir>` that:
  1. parses the feature file for its scenario names (a `Scenario:` / `Scenario Outline:`
     line scan is sufficient — do not add a Gherkin parser dependency);
  2. computes each slug with the expression in §3 D6, **verbatim**;
  3. renders `stage-runner.template.sh` (§6.2) into `<out>/stage-runner.sh`, mode 0755;
  4. copies the feature file byte-identically to `<out>/<basename>.feature`;
  5. runs `compute-executor artifact` on both to fill `binary` and `dna`;
  6. emits `<out>/task.json` exactly as §5.1, with
     `project = "a2o-stage:<feature-slug>"` and `runtimeImage` from
     `--runtime-image` (default: `sha256:` + sha256 of the executor binary);
  7. **validates the result with `compute-executor run`'s parser, not `cid`'s** — invoke
     `compute-executor run --task <out>/task.json --binary /bin/true --dna /dev/null
     --root <tmp> --ark /bin/false --request-action X --grant-action Y` and require that
     the error is `runtime image identity unavailable or mismatched` (the gate *past*
     task parsing) and not `unknown field` or `unsupported task`. This is the §4
     constraint-1 guard and it is the single most valuable check in this task.
- **Test:** `genesis/agentic/compute/stage/build-stage-task.test.mjs` — builds against the
  target feature and asserts: one `expectedTests` entry, it contains `feedback_signal`,
  the slug equals the literal in §3 D6, `dna.sha256` equals the feature file's sha256,
  envelope JSON under 64 KiB, and the `compute-executor run` probe reaches the
  runtime-image error.
- **Verify:** `cd genesis/agentic/compute && node --test stage/build-stage-task.test.mjs; echo "EXIT=$?"`

---

**Task 2 — the a2o feature.**

- **Files (new):** `genesis/a2o/features/compute/peer-executed-stage.feature`,
  `genesis/a2o/features/compute/.epr-meta`
- **Do:** write the feature of §10 verbatim. The `.epr-meta` is **mandatory** — a new
  subdirectory under `genesis/a2o/` must be born with one
  (`genesis/a2o/.epr-meta`, rule `new-feature-subdir-needs-meta`); copy the shape of
  `genesis/a2o/features/dataplane/resiliency-saga/.epr-meta`.
- **Test:** none yet (steps land in task 3).
- **Verify:** `cd genesis/a2o && node_modules/.bin/cucumber-js --dry-run --tags '@concern:operator-runtime-surface and @requires:household-nodes' features/compute/peer-executed-stage.feature; echo "EXIT=$?"`
  — expect undefined steps, not a parse error.
- **Obligation:** writing a `*.feature` under `genesis/a2o/` triggers the blind-reader
  review loop (`genesis/a2o/.epr-meta`, rule `a2o-story-blind-reader-review`). Dispatch a
  fresh `blind-reader` with **only** the feature path and the `a2o-story` profile, revise,
  repeat until READY.

---

**Task 3 — the step definitions.**

- **Files (new):** `genesis/a2o/steps/compute/peer-executed-stage.steps.ts`
- **Do:** implement the eleven steps named in §10. Own your state: the
  delegated-sweettest steps keep theirs in a module-private `WeakMap`
  (`genesis/a2o/steps/dataplane/delegated-sweettest.steps.ts:58`), so its phrasings
  **cannot be reused** without exporting that map. Mirror the phrasing, own the state.
  Read the fixture from `PEER_STAGE_A2O_CONFIG` (a local JSON file, never committed):
  `{ stageDir, requesterApi, providerApi, requesterKey, providerKey, env, timeoutSeconds }`.
  Absent fixture ⇒ `return 'pending'` (never a passing live result), the same discipline
  as `delegated-sweettest.steps.ts:96-97`.
  Report decoding: read the materialized `stdout.log`, take the text between
  `-----BEGIN ELOHIM STAGE REPORT-----` and `-----END ELOHIM STAGE REPORT-----`,
  base64-decode, `JSON.parse`. Assert the report's `uri` names the pinned feature, that
  the set of scenario slugs equals the declared inventory, and that every step of every
  scenario has `result.status === "passed"` — **not** `receipt.status` (D9).
- **Test:** the scenario itself.
- **Verify:** `cd genesis/a2o && node_modules/.bin/cucumber-js --dry-run features/compute/peer-executed-stage.feature; echo "EXIT=$?"` → zero undefined steps.

---

**Task 4 — document the stage operation.**

- **Files (edit):** `genesis/agentic/compute/operations.md` — append a
  `## Peer-executed a2o stage` section. Do **not** create a new `.md` under
  `genesis/agentic/compute/`; appending to the existing file avoids the
  frontmatter-at-birth rule entirely.
- **Do:** record the D2 `project` grammar, the D6 test-name shape and the reason for the
  `feedback_signal::` token, the D7 sentinel framing, and the §5.3 capacity grant.
- **Verify:** `git diff --stat -- genesis/agentic/compute/operations.md`

---

**Task 5 — bring the compute surface up on the mesh.**

- **Files:** none (runtime).
- **Do:**
  1. `TOK=$(head -c 48 /dev/urandom | base64 | tr -d '\n')` — ≥32 bytes required
     (`compute_tasks.rs:32`).
  2. Restart matthew and jessica's storage with
     `ELOHIM_COMPUTE_LOCAL_API=1 ELOHIM_COMPUTE_LOCAL_TOKEN="$TOK"` exported, via
     `just mesh storage-restart matthew jessica`. **Do not restart james** — it is the
     unauthorized-refusal peer and does not need the surface.
  3. Apply the §5.3 capacity grant.
  4. Start the provider worker as root:
     `COMPUTE_PERFORMER=<jessica key> COMPUTE_API_URL=http://127.0.0.1:8091
      COMPUTE_WORKER_ROOT=/tmp/elohim-local-mesh/jessica/compute
      COMPUTE_EXECUTOR=/projects/.cargo-target-pool/family/dev/elohim__rakia/dev/debug/compute-executor
      COMPUTE_ARK=/projects/.cargo-target-pool/family/dev/elohim/dev/debug/ark
      COMPUTE_RUNTIME_IMAGE=sha256:<digest of the executor binary>
      ELOHIM_COMPUTE_LOCAL_TOKEN="$TOK"
      node genesis/agentic/compute/worker.mjs`
     under a supervisor that survives the session.
  5. Issue the grant on **jessica's own** loopback adapter:
     `COMPUTE_API_URL=http://127.0.0.1:8091 COMPUTE_PERFORMER=<jessica key>
      ELOHIM_COMPUTE_LOCAL_TOKEN="$TOK"
      node genesis/agentic/compute/workspace.mjs grant grant.json`
     with `recipient` = matthew's agent key, `scope` implied by the capability, and
     bounds `{"epr_scope":["*"],"reach_ceiling":"commons","rate_per_hour":8,
     "rotation_ttl_days":5}`. Record `grantActionHash` as `COMPUTE_GRANT_ACTION`.
- **Verify:** `curl -s -o /dev/null -w '%{http_code}\n' http://127.0.0.1:8091/api/v1/compute/tasks`
  → **403** (`local-cell-actor-required`), not 503. A 503 means step 2 did not take.

---

**Task 6 — Run A: the stage.**

- **Files (new, gitignored):** `genesis/a2o/reports/peer-stage/<date>/`
- **Do:** build the task (task 1), submit, `start` the listener, `poll` to completion.
- **Verify:** `GET /api/v1/compute/tasks/<REF>` on :8090 shows `.completion.actionHash`
  non-null and `.completion.receipt.status == "passed"` **and** the decoded report shows
  the one scenario passed. If `status` is `passed` but the report disagrees, stop — that
  is the D9 case and it is a finding, not a flake.

---

**Task 7 — Run B: the idempotency proof.**

- **Do:** §8 Run B, all six assertions.
- **Verify:** all six hold; record the outputs into the receipt.

---

**Task 8 — the receipt.**

- **Files (new):** `genesis/a2o/reports/peer-stage/<date>/receipt.json` plus the copied
  artifacts named below.
- **Do:** emit exactly these fields.

```json
{
  "schemaVersion": 1,
  "stage": {
    "name": "federation-version-convergence",
    "project": "a2o-stage:federation-version-convergence",
    "featurePath": "genesis/a2o/features/dataplane/federation-version-convergence.feature",
    "featureSha256": "…",
    "scenarios": ["<slug>"],
    "expectedTests": ["feedback_signal::a2o::dataplane::…"]
  },
  "identity": {
    "requester": "<matthew agent key>",
    "provider":  "<jessica agent key>",
    "grantActionHash": "<uhCkk…>",
    "grantCommitmentCid": "<entry hash the bounds validator read>"
  },
  "runA": {
    "taskCid": "<C1>",
    "invocation": "<N1>",
    "requestActionHash": "<R1>",
    "acceptanceActionHash": "<uhCkk…>",
    "attestationActionHash": "<A1>",
    "receiptCid": "<…>",
    "payloadCid": "<P1, the stdout.log artifact cid>",
    "payloadChunkCids": ["…"],
    "receiptStatus": "passed",
    "exitCode": 0,
    "observedTests": ["…"],
    "reportVerdict": { "scenarios": 1, "passed": 1, "failed": 0, "steps": 0 }
  },
  "runB": {
    "taskCid": "<C1>",
    "requestActionHash": "<R1>",
    "attestationActionHash": "<A1>",
    "newExecution": false,
    "admissionEventCount": 1
  },
  "wallClocks": {
    "submittedAt": "…Z", "acceptedAt": "…Z", "startedAt": "…Z",
    "completedAt": "…Z", "reportReadAt": "…Z",
    "submitToAcceptSeconds": 0, "acceptToCompleteSeconds": 0,
    "completeToReadSeconds": 0, "runBSeconds": 0
  },
  "environment": {
    "mesh": { "matthew": 8090, "jessica": 8091, "james": 8092 },
    "executor": "<path>", "ark": "<path>", "runtimeImage": "sha256:…",
    "gitCommit": "<short sha>", "transport": "<observed>"
  },
  "files": ["task.json", "stage-runner.sh", "<feature>.feature",
            "compute-receipt.json", "cucumber.json", "stdout.log", "run.log", "env.txt"]
}
```

`env.txt` must be token-redacted. Copy the artifacts alongside it so the receipt is
self-contained.

- **Verify:** `node -e 'JSON.parse(require("fs").readFileSync(process.argv[1]))' genesis/a2o/reports/peer-stage/<date>/receipt.json; echo "EXIT=$?"`

---

**Task 9 — the habit DELTA.**

- **Files (edit):** `elohim/elohim-storage/.epr-meta/operator-runtime-surface.habit.md`
- **Do:** append a `DELTA 2026-09-08 (first peer-executed CI stage …)` paragraph naming
  the receipt path, both wall-clocks, the attestation hash, and the honest scope — this
  is a **household** result with jessica standing in as provider, not a shem/Adam result,
  exactly as the 2026-09-07 DELTA says of its own leg. Do not flip `status`.
- **Verify:** `.claude/scripts/habits-project.py --check; echo "EXIT=$?"`

---

## 10. The a2o binding

`genesis/a2o/features/compute/peer-executed-stage.feature`:

```gherkin
@e2e @act:i @compute @concern:operator-runtime-surface @requires:household-nodes
Feature: A peer runs one of the household's CI stages and the household can check its work

  A stage is a piece of the household's own verification: one feature file, the scenarios
  it declares, and a verdict. Until now a stage ran wherever the developer happened to be
  sitting. This one runs on a neighbour's peer, because the neighbour offered capacity and
  signed a grant saying this household may use it.

  What comes back is not a green tick. It is the report the run actually produced, carried
  as expiring bytes leased to the stage's own address, plus a signed statement from the
  peer that it ran this exact stage under this exact grant. The household reads the report.
  It does not take the peer's word for the verdict, because a signature proves who spoke,
  never that what they said is true.

  Background:
    Given a household compute provider with a signed compute grant for this requester

  # WRITES: the stage it runs stages a real disagreement on two peers and flips an
  # operator flag, so this scenario is only ever acceptable on a substrate this run owns.
  @requires:owned-substrate
  Scenario: a peer runs the household's federation-convergence stage and the household checks the report
    Given a pinned stage whose runner script and feature file are content-addressed
    When the requester submits the stage to the provider
    Then the provider accepts the stage and runs it on its own substrate
    And the requester recovers the provider's signed completion for the pinned stage
    And the returned report names the pinned feature file and every declared scenario
    And every declared scenario passed in the report, not merely in the receipt
    And the completion is attested by an economic event naming the grant
    When the requester submits the identical stage a second time
    Then the same request is recovered and the provider runs nothing new
    And exactly one admission event exists for that stage
```

Eleven steps. Every one needs a new definition in
`genesis/a2o/steps/compute/peer-executed-stage.steps.ts`:

| Step | What it does | Mirrors |
|---|---|---|
| `Given a household compute provider with a signed compute grant for this requester` | loads `PEER_STAGE_A2O_CONFIG`; asserts `stageDir`, both API bases are loopback, `COMPUTE_GRANT_ACTION` present; `pending` if absent | `delegated-sweettest.steps.ts:95-104` + `:292-295` |
| `Given a pinned stage whose runner script and feature file are content-addressed` | asserts `task.json`'s `binary`/`dna` descriptors match `compute-executor artifact` on the two files, and `dna.sha256` matches the repo feature file | new |
| `When the requester submits the stage to the provider` | `workspace.mjs submit task.json stage-runner.sh <feature>`; captures `R1`, `C1`, `N1` | `delegated-sweettest.steps.ts:106-124` |
| `Then the provider accepts the stage and runs it on its own substrate` | polls `GET /api/v1/compute/tasks/<R1>` until `.acceptance`; asserts no completion yet | `:126-143` |
| `Then the requester recovers the provider's signed completion for the pinned stage` | polls until `.completion`; runs `workspace.mjs poll`; asserts `.taskCid == C1`, `receipt.binary`/`receipt.dna` deep-equal the envelope's, `receipt.provider == providerKey` | `:157-182` |
| `Then the returned report names the pinned feature file and every declared scenario` | decodes the framed base64 from the materialized `stdout.log`; asserts report `uri` ends with the pinned feature path and the slug set equals `expectedTests`' suffixes | new |
| `Then every declared scenario passed in the report, not merely in the receipt` | asserts every step of every scenario is `passed` in the decoded report — the D9 assertion | new |
| `Then the completion is attested by an economic event naming the grant` | asserts `.completion.actionHash` non-empty; reads `GET /api/v1/economic-events/compute-admission%3A<R1>` on the provider and asserts `bounded_by` equals the grant commitment cid and `action == "sweettest-feedback"` | new |
| `When the requester submits the identical stage a second time` | re-runs submit with no flag; captures the second response | new |
| `Then the same request is recovered and the provider runs nothing new` | asserts second `.requestActionHash == R1`, `.taskCid == C1`, `task.json.invocation == N1`, `.completion.actionHash` unchanged | new |
| `Then exactly one admission event exists for that stage` | asserts the single admission row and that no second `compute-admission:` id resolves | new |

---

## 11. Operator items

Agents cannot do these; they are listed so they are not lost.

1. **`elohim/rakia`: commit `rakia-executor/` and bump the pin** — already filed
   (`rakia-executor-untracked-in-submodule-pin.md`). Until then no fresh clone or CI can
   build the executor this stage depends on, and every constraint in §2 rests on a binary
   whose source is not in the tree.
2. **Widen the executor's task vocabulary** so a stage need not wear the
   `feedback_signal` costume: accept a `taskKind` set rather than a const, and an
   inventory filter that does not require the literal token. This is the one change that
   would let §3 D1 and D6 be retired. It also needs the matching one-line change at
   `compute_task.rs:200` and the schema const. Not required for this landing.
3. **Rename or generalize the `dna` slot** to "second input artifact" — D5 uses it for a
   feature file, which is honest but misnamed. Cosmetic; do it with item 2.
4. **Decide whether a named result payload is allowed.** If the executor's payload
   collector permits arbitrary names, adding `cucumber.json` to the two allowlists we own
   (`worker.mjs:179`, `workspace.mjs:72`) would retire the sentinel framing of D7. Only
   the operator can read the executor to answer this.
5. **Adam as provider** — provision the Secret and pin the image digest so
   `genesis/orchestrator/data/adam-compute-worker.json` can enable. Every result here is
   a *household* result until that lands.
6. **Fix `compute-executor cid` on explicit JSON null** — already filed
   (`compute-executor-cid-refuses-null-retention-field.md`); §5.1 works around it.

---

## 12. Out of scope

- A stage **DAG**. D2 reserves the `@after=` grammar; nothing schedules on it, nothing
  walks it, and no stage in this landing has an upstream.
- Moving the storage cargo gate onto a peer (plan T10 stretch, next sprint).
- Adam as provider (operator item 5).
- Any change to `genesis/a2o/steps/dataplane/federation-deploy.steps.ts` or to the
  scenario under test (D10).
- Any zome, entry type, migration, view, or HTTP route (§4 — none needed, none added).
- Making the provider *honest*. §6.3 states plainly what the substrate does and does not
  guarantee; closing that gap is a witness-and-review problem, not a substrate one, and
  the read-only reviewer path (`workspace.mjs:93-136`) already exists for it.
