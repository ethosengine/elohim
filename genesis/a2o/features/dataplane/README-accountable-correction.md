# Accountable correction — slice 1 environment notes

An **accountable correction** is a feedback signal one peer files against another peer's
published content record — a factual complaint ("this is wrong"), not an edit. Filing one is an
allegation and costs the filer nothing; only the record's own **root author** (whoever's `Create`
action started the record's lineage) can turn it into something settled, by explicitly **accepting**
it and then publishing a **successor** (the amended content). Nobody else's say-so moves the
record. This is **slice 1** of a staged reimplementation — the first bounded increment; later
slices are expected to add capabilities (e.g. delegated acceptance) this one deliberately excludes.
The feature file organizes its eight scenarios as **stations**, one per numbered contract section.

Companion to `accountable-correction.feature` and the contract it is bound to:
`genesis/docs/superpowers/specs/2026-09-06-accountable-correction-contract.md` (read that first —
every section below cites it, e.g. "contract §3", by number). This file is appended to by
whichever agent lands the matching Rust surface, in either direction (a2o first, Rust first) —
never overwritten. Each section names who wrote it and when.

The three coordinator calls (functions hosted in the conductor’s DNA coordinator zome and invoked by storage or a signed client) this feature
exercises: `create_feedback_signal` files a correction; `create_vouch` (with
`vouch_kind = accept-correction`) is the root author's own act of accepting one — a "vouch" here
means one signed record standing behind another; `amend_content` is the root author publishing the
corrected content, naming the exact prior record ("predecessor") it replaces.

## `ELOHIM_FEEDBACK_NOTIFY` (a2o, 2026-09-06)

Storage learns about a correction through two independent paths: a **durable discovery scan**
(contract §3) — every peer periodically re-scans the records it stewards or has been pointed at,
so a correction is found even if nobody told it — and a **direct p2p notification** (contract §4),
an accelerant that lets a peer learn immediately instead of waiting for its next scan. Station 1
needs a way to prove the scan alone is sufficient, with the notification switched off. No such
flag exists today. (The contract's own text calls this mechanism a "sweep" — this document and the
feature file say "scan" throughout for one consistent term; they name the same thing.)

The a2o story names the flag it needs as **`ELOHIM_FEEDBACK_NOTIFY`** on the storage peer that
_files_ the correction (the author's own peer — in the feature, James's):

- `ELOHIM_FEEDBACK_NOTIFY=0` — suppress sending the direct p2p `feedback-signal` notification
  (contract §4) for a correction this peer authors. The correction is still committed to the
  author's own source chain and still discoverable by every other peer's durable discovery scan
  (contract §3); only the accelerant is withheld.
- unset, or `ELOHIM_FEEDBACK_NOTIFY=1` — today's behavior: notify as well as commit.

This is a **test-only escape hatch** on the notification send path, not a protocol behavior — a
production peer always notifies when it can. The step definitions in
`steps/dataplane/accountable-correction.steps.ts` set this env var on the target peer's process
environment before the correction-filing call and expect the storage process to have read it at
call time, not only at boot. That constraint is load-bearing: `hc-mesh.sh` (the local mesh
harness) never restarts a peer's process between scenarios in one run, so a boot-time-only flag
could not be flipped per scenario.

**Owed by the Rust surface:** reading this env var at the point `p2p/mod.rs` would otherwise send
the `feedback-signal` message (contract §4), and treating it as an author-local, per-call
suppression, not a global peer-startup flag.

## Peer roles this feature assumes (a2o, 2026-09-06)

The Background registers all three household-mesh peers — fixture identities named `matthew`,
`jessica`, `james`, each running its own storage node — but the stations assign them fixed roles
so a reader does not have to re-derive who is who per scenario:

- **Jessica** — root author of the target content record in every station. Only she may accept a
  correction (`create_vouch` with `vouch_kind = accept-correction`) or publish its successor
  (`amend_content`).
- **James** — files every correction (`create_feedback_signal`) against Jessica's record. Station
  3 also has James attempt acceptance, refused because he is not the root author.
- **Matthew** — the third peer: never a root author, correction author, or acceptor in these
  stations. He is the durable-discovery witness — the peer whose scan (contract §3) is what
  proves discovery does not depend on notification.

## The Rust surface as it landed (rust-architect, 2026-09-06)

Written after the coordinator + storage slice landed on `sprint/accountable-correction`. Bind
step definitions to what is written here; where this contradicts an earlier expectation above,
this section is what the code does.

### `ELOHIM_FEEDBACK_NOTIFY` — landed as a RUNTIME-CONFIG setting, not a raw env read

The a2o note above asks for a flag readable "at call time, not only at boot", because
`hc-mesh.sh` never restarts a peer between scenarios. A process cannot have its environment
changed from outside after it starts, so a literal `std::env::var` at the send site could not
have satisfied that — it would only ever see the boot value.

What landed instead is a registered setting in this crate's existing **watched runtime-config
registry** (`elohim/elohim-storage/src/runtime_config.rs`, `Key::FeedbackNotify`), which is the
mechanism already armed on every mesh peer (`ELOHIM_RUNTIME_CONFIG_PATH=<mesh>/<peer>/runtime-config.toml`,
set from boot by `hc-mesh.sh`). It is hot: the send path reads the registry per act.

A scenario flips it on a RUNNING peer, then verifies the change:

- write `ELOHIM_FEEDBACK_NOTIFY=0` into that peer's `runtime-config.toml` and wait for the 10s
  poller, or force it immediately with `POST /admin/runtime-config/reload` on that peer;
- read back the effective value and its provenance (`boot-env` vs `runtime-config`) from
  `GET /admin/runtime-config` — use this to ASSERT the flip took, rather than assuming it did.

Boot-time `ELOHIM_FEEDBACK_NOTIFY=0` in the process environment also works and remains the
default source; the file overrides it while present, and removing the key restores the boot value.

Semantics as implemented (`services::back_prop::direct_notify_enabled`): at `0`, `back_prop_one_hop`
returns "no predecessors targeted" without sending. The act is still committed to the author's own
source chain and still discoverable by every other peer's durable scan. Default (unset) is `1`.

### Coordinator functions (elohim DNA, `content_store` zome — coordinator-only, DNA hash unmoved)

| Function | Input | Returns |
|---|---|---|
| `create_feedback_signal` | `{ target_action_hash, signal_kind, evidence_action_hash?, standing_impact }` | `ActionHash` |
| `create_vouch` | `{ target_action_hash, vouch_kind, standing_impact }` | `ActionHash` |
| `amend_content` | `{ predecessor_action_hash, content: { title?, description?, content?, metadata_json?, blob_cid?, content_size_bytes?, content_hash?, reach? } }` | `ContentOutput` |
| `get_content_lineage` | `{ action_hash, local? }` | `ContentLineageOutput` |
| `get_feedback_signal_record` | `ActionHash` | `Option<Record>` |
| `get_feedback_signal_refs_for_target` | `{ target_action_hash, resolve? }` | `FeedbackSignalRefs` |
| `list_feedback_signal_refs_by_signer` | `{ signer_pubkey, resolve? }` | `FeedbackSignalRefs` |

`ContentLineageOutput` carries `root_action_hash`, `root_author`, `content_id`, `candidates[]`
(each `{ action_hash, predecessor, author, timestamp, fetch_outcome, in_root }`),
`head_action_hash`, `contested`, `contested_predecessors[]`, and the honest counters
`link_count` / `duplicate_links` / `invalid_link_targets` / `other_root_candidates` /
`unfetchable_candidates` / `truncated`.

**Correction admission changed (contract §8).** `create_feedback_signal` with
`signal_kind: "correction"` now REFUSES unless `evidence_action_hash` resolves to a Content
record that is (a) public — `reach` one of `public` / `commons` — and (b) carries this exact
object in its `metadata_json`:

```json
{ "correctionRequest": {
    "operationId": "<the operation id>",
    "targetActionHash": "<same value as target_action_hash>",
    "signalKind": "correction",
    "standingImpact": "<same value as standing_impact>" } }
```

Any mismatch is a refusal, not a retry. A station that files a correction must author its
evidence in that shape — or use the outbox below, which does it for you.

**Refusal substrings** worth asserting on: `"is not the author"` (a non-root-author
`amend_content`), `"cross-root canonical head already stands"` (an amendment that could never
become the served head), `"correction evidence"` (every admission refusal).

### Storage HTTP

| Route | Purpose |
|---|---|
| `POST /api/v1/feedback/operations` | The submission outbox (§8). Body: `{ operationId, targetActionHash, signalKind, standingImpact, vouchKind?, body? }`. Runs BOTH phases: authors the Correction EPR with Content id `correction:<operationId>` and the embedded request, then files the feedback citing it. |
| `GET /api/v1/feedback/operations/{operationId}` | `{ operationId, phase, status, evidenceActionHash, feedbackActionHash, requestBytesCid, lastError, … }`. |

Status codes: `201` resolved · `202` accepted-but-not-resolved (read `status`: `pending` or
`unresolved`) · `409` the operation id was reused with DIFFERENT request bytes (the request an
operation pins is immutable) · `503` this node has no content cell.

`status: "unresolved"` is a terminal-for-automation state: an uncertain call whose recovery
enumeration found ZERO matches is NEVER auto-resubmitted. A scenario that wants to retry POSTs
the same `operationId` again, which reuses the same evidence action and therefore the same
group — a second act would be a group MEMBER, never a second contribution.

### What a station can and cannot observe today

- **Can:** the coordinator refusals above; `get_content_lineage` naming the exact root and
  reporting `contested`; the outbox's phase/status; `GET /admin/runtime-config` proving the
  notify flip took.
- **Cannot yet:** there is no HTTP read for the per-generation application rows or the
  `rebuilding` generation state. Standing is still read through the existing
  `GET /api/v1/standing/{agent_cid}`, whose semantics CHANGED at this cutover: a correction alone
  now contributes zero and leaves the subject's aggregate ABSENT (Unknown), where before it
  debited the signal's SIGNER immediately. To prove filing costs James nothing, compare
  James's standing before and after filing. Separately, to prove an unaccepted allegation
  costs Jessica nothing, assert her standing remains Unknown/absent when she has no prior
  accepted corrections; a present row with a zero score is not absence.

## Live-loop and binding follow-up (Codex, 2026-09-06)

The projector is now constructed from the running peer's content-cell client and spawned
beside the release-adoption controller in `main.rs`. The production reader decodes typed
Holochain hashes, verifies the signed action and entry binding, and checks the Correction
EPR's immutable request. Held content anchors populate durable subscriptions. An allegation
can transition from its zero contribution to an accepted contribution; a replay of that
acceptance cannot debit it again. Standing reads normalize the content-cell public keys.

For a mesh run, build this worktree's storage binary and pass its path explicitly as
`STORAGE_BIN`; the harness's default is a **dev-family** pool slot and does not select this
sprint branch's binary. The mesh also needs a matching Holochain conductor/CLI pair, a packed
hApp, and its doorway binary. Use `just mesh status` to inspect the next launch before starting.
Set `ELOHIM_FEEDBACK_SWEEP_SECONDS=5` on the launch command for a short scenario cadence;
unset defaults to 60 seconds. `ELOHIM_FEEDBACK_NOTIFY` still uses the watched runtime-config
registry, independently of the sweep. The bindings read `ELOHIM_RUNTIME_CONFIG_PATH` from
that peer's running process, edit its file, reload, assert the effective value, and restore
the original file after the scenario.

### Station 4's deliberate crash

Opt in at launch with `ELOHIM_TEST_FEEDBACK_CRASH_ONCE=1`. The scenario arms Jessica alone by
creating `<STORAGE_DIR>/feedback-crash-once` before acceptance. The projector consumes that
file and aborts **inside** the transaction, after writing the application row and before
writing the generation aggregate. The consumed file prevents a second abort after
`hc-mesh.sh storage-restart jessica`. Without both the environment opt-in and the per-peer
file, this hook does nothing. This is a storage crash test; it never restarts a conductor.

### Station 7's rebuild trigger

`POST /api/v1/feedback/generations/rebuild` returns `202` with `{ generationId, status:
"rebuilding" }`. It retains the old generation and copies its retained action references into
a new generation under the same pinned policy. A process-local writer lock serializes the
request with projector ticks. Retained pending references receive service independently of
fresh link discovery. Standing reads return `503` with a rebuilding explanation while an
in-flight generation exists. Publication waits for pending members to settle; the fixture
compares operation groups, acceptance dependencies, subjects, contributions and policy
fingerprints, excluding operational timestamps.

The bindings use read-only SQLite queries as local test witnesses for application rows and
generations; these are not new product read endpoints. Station 8 discards the initial HTTP
response body, independently checks that the server resolved the operation, then retries
with the same client-minted operation id. This exercises lost-result recovery, not a proxy
that cuts a TCP connection mid-response.

### Remaining binding boundaries

`genesis/a2o/steps/dataplane/accountable-correction.steps.ts` contains 53 step definitions. Eight remain explicitly pending, each with
its reason attached to the Cucumber receipt:

- Station 1 needs a fixture that withholds an **older** correction's discovery and releases it
  later. Backdating a newly authored source-chain action is not a substitute.
- Station 2 needs a notification-driven wake and an observable scheduled-scan deadline to
  prove acceleration, plus a P2P fixture that injects a foreign-DNA envelope into the real
  receiver. Filing through HTTP cannot prove transport rejection.
- Station 3's dependent-view re-render needs a specified browser view and route. A successful
  content HTTP read alone cannot establish that a dependent view re-rendered.

Station 5's literal Unknown assertion needs an evaluator with no earlier accepted corrections.
All scenarios use the same three cell agents, so prior stations can leave Jessica with standing;
this fixture-isolation constraint must not be hidden by treating a pre-existing tally as Unknown.
Station 6 checks that the sequential **edge** introduces no additional contested predecessor;
the earlier fork remains contested, as the contract requires.

Run `just test mesh genesis/a2o/features/dataplane/accountable-correction.feature` with
`A2O_RUN_WIP=1` after `just mesh start` and `just mesh prologue`. Keep a scenario's `@wip` until
its whole scenario passes, and always finish with `just mesh stop`. The mesh-attempt section below records the startup refusal. Its logs are separate from the
registration receipts; no live scenario receipt exists.


### First supported check: binding registration

From this worktree's root, with workspace dependencies installed (`pnpm install`), run:

```bash
CUCUMBER_JSON_REPORT=reports/accountable-correction/bindings-dry-run.json \
CUCUMBER_HTML_REPORT=reports/accountable-correction/bindings-dry-run.html \
pnpm --dir genesis/a2o exec cucumber-js --dry-run --tags '@concern:accountable-correction'
```

Expected result: **8 scenarios / 91 steps skipped, zero undefined steps**. This confirms that
the bindings load; it does not execute a station or establish product correctness. The JSON
and HTML receipts are under `genesis/a2o/reports/accountable-correction/`.

For live execution, `just gate elohim-storage` prints the target pool slot used by this
branch. Build its executable with `CARGO_TARGET_DIR=<that-slot> cargo build --manifest-path
elohim/elohim-storage/Cargo.toml`, retaining storage's existing `RUSTFLAGS`. Pass
`STORAGE_BIN=<that-slot>/debug/elohim-storage`, `ELOHIM_FEEDBACK_SWEEP_SECONDS=5`, and
`ELOHIM_TEST_FEEDBACK_CRASH_ONCE=1` to `just mesh start`. Conductor/CLI selection, bundle
packing, and doorway prerequisites are documented in the usage block and start preflight of
`app/elohim-app/scripts/hc-mesh.sh`; `HOLOCHAIN_BIN` accepts a directory containing both
matching binaries. This handoff does not supply a prebuilt 0.7 pair or all required DNAs.
Until the mesh prerequisites are available and a station has passed, there is no claimed
successful live station. A start failure is a stopping boundary, not permission to substitute
an unrelated binary or another worktree's bundle.


### Mesh attempt — 2026-09-07

Receipt directory: genesis/a2o/reports/accountable-correction/mesh-20260907000336Z/.

- "just mesh stop": EXIT=0.
- "STORAGE_BIN=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev/debug/elohim-storage ELOHIM_FEEDBACK_SWEEP_SECONDS=5 ELOHIM_TEST_FEEDBACK_CRASH_ONCE=1 just mesh start": EXIT=1.
- Exact refusal: "missing binary: /projects/.cargo-target-pool/family/dev/doorway__doorway-service/dev/debug/doorway (build it first — see CLAUDE.md pool-slot paths)".
- Final "just mesh stop": EXIT=0.

**Stations 1–8: NOT RUN. Zero stations passed; zero station assertions failed.** Startup
failed before any peer launched or storage executable ran. The preflight selected stock
Holochain/hc 0.6.0; no 0.7 pair was found in the inspected executable locations. This
worktree has lamad.dna but no complete packed hApp. The missing doorway binary was the
actual stopping error; the other prerequisites were not reached. Following the stopping
rule, no prologue or live test was attempted and no @wip was removed. There is no live
Cucumber JSON/HTML receipt to copy; bindings-dry-run.{json,html} is explicitly a separate
registration check. The next operator must supply the mesh prerequisites before retrying
this same worktree and feature.

### Mesh run — 2026-09-07 (rust-architect)

Supersedes the "Mesh attempt — 2026-09-07" section above: the prerequisites that
blocked it were supplied and the stations ran live. Receipt directory:
`genesis/a2o/reports/accountable-correction/mesh-20260907T025002Z/` (`cucumber.json`,
`cucumber.html`, `run.log`). **8 scenarios / 91 steps: 3 passed, 3 pending, 2 failed,
69 steps passed.** Cucumber exit code 1 (a pending or failing scenario is a red run).

| Station | § | Verdict | One line |
|---|---|---|---|
| 1 — durable discovery | §3 | PENDING (13 steps ok) | Matthew's unnotified peer DOES discover and apply the correction on its own scan; stops at the late-arrival fixture, which cannot backdate a Holochain action. |
| 2 — notification accelerates | §4 | PENDING (12 steps ok) | Stops at the acceleration proof: notifications enqueue subscriptions but there is no notification-driven wake or observable next-scan deadline to measure against. |
| 3 — only the root author settles | §5.2/§5.3/§6 | PENDING (16 steps ok) | Acceptance, successor, three-peer head adoption and James's refusal all pass; stops at "a dependent view re-renders", which names no browser view or route. |
| 4 — crash window | §7 | **PASSED** (18 steps) | Storage killed mid-transaction, restarted, reapplied exactly once; replay of a settled correction leaves the tally unchanged. |
| 5 — an allegation costs nobody | §7 | FAILED (12 steps ok) | Honest refusal, not a product defect: the station asserts Jessica reads *Unknown*, and by the time it runs she carries standing from earlier stations. Its own guard says so ("requires a fresh evaluator with no prior accepted corrections"). Needs per-scenario evaluator isolation. |
| 6 — contested fork | §6 | **PASSED** (18 steps) | Two updates naming one predecessor mark the record contested; the deterministic pick serves a head without clearing the marker; a sequential amendment is not contested. |
| 7 — rebuilt generation | §7 | FAILED (12 steps ok) | The rebuild publishes, but this scenario's own acceptance had not reached the rebuilt generation's tally inside the poll (expected 8, observed 6). Rebuild-convergence budget, or a real lag between republication and the retained-member replay — not yet separated. |
| 8 — lost response | §8 | **PASSED** (16 steps) | A discarded response plus a retry under the same client-minted operation id resolves to one correction and at most one contribution. |

`@wip` removed from stations 4, 6 and 8 only. Stations 1, 2, 3, 5 and 7 keep it.

**The projection itself was read directly and is correct.** Mid-run, jessica's peer held
17 application rows, all `applied`: three with `accepted=1, contribution=2` and fourteen
unaccepted allegations at `contribution=0`, with `standing_generation_aggregate.debit_weight_sum`
exactly 6. That is §7's rule — an allegation debits nobody, only an accepted correction
contributes, and it contributes once — observed on the mesh rather than argued.

**Environment the run needed** (none of it is default; `just mesh status` shows the
selection before you start):

```bash
STORAGE_BIN=/projects/.cargo-target-pool/family/sprint/elohim__elohim-storage/dev/debug/elohim-storage \
HOLOCHAIN_BIN=/projects/.claude-config/tools/hc-fork-25dd2d0be144/bin \
MESH_RELAY_BIN=/projects/.claude-config/tools/iroh-relay-1.0.3/bin/iroh-relay \
ELOHIM_FEEDBACK_SWEEP_SECONDS=5 ELOHIM_TEST_FEEDBACK_CRASH_ONCE=1 just mesh start
A2O_RUN_WIP=1 just test mesh features/dataplane/accountable-correction.feature
```

Notes for whoever runs it next:

- The storage binary must be built `--features "p2p p2p-iroh"`. The mesh defaults to
  `MESH_TRANSPORT_BACKEND=dual` and refuses to start a binary without the iroh marker.
- `HOLOCHAIN_BIN` must be set explicitly. Stock on PATH is 0.6.0 and will not load an
  hdk-0.7 DNA; auto-detect only looks in `fork-bin` directories, so the 0.7 pair has to be
  named. `just mesh status` then still labels the running conductor "STOCK" — that label
  compares against the fork-bin *path*, not the version, and is wrong here.
- The scope positional is relative to `genesis/a2o`, not the repo root.
- `just mesh prologue` needs built Angular dists and is NOT required by these stations:
  the Background authors its own content and the helpers read peer HTTP, conductor calls
  and SQLite directly. It was not run.
- The stations share three cell agents and do not reset state between scenarios or between
  runs. Station 5's assertion is the one that notices; treat a fresh mesh as part of its
  fixture.

## Mesh rounds 2026-09-07 04:52 - 08:24 (rust-architect)

Five rounds, three completed. Receipts: `genesis/a2o/reports/accountable-correction/mesh-20260907T045232Z/`,
`.../mesh-20260907T051633Z/`, `.../mesh-20260907T055414Z/` (each `run.log` + `sprint-report.json`),
plus an aborted partial at `.../mesh-20260907T082426Z-aborted/`.

| Station | 045232Z (600 s ceiling) | 051633Z | 055414Z |
|---|---|---|---|
| 1 discovery | PENDING | PENDING | PENDING |
| 2 notification | PENDING | PENDING | PENDING |
| 3 root author settles | PENDING | PENDING | PENDING |
| 4 crash window | FAILED (crash env absent) | FAILED (210 s crash-arm budget) | **PASSED** |
| 5 allegation costs nobody | **PASSED** | FAILED (stale baseline) | **PASSED** |
| 6 contested fork | **PASSED** | **PASSED** | **PASSED** |
| 7 rebuilt generation | **PASSED** | **PASSED** | FAILED (Background quiescence timeout) |
| 8 lost response | **PASSED** | **PASSED** | FAILED (Background quiescence timeout) |

Every one of stations 4-8 has now passed on the household mesh; none of the rounds carries all
five at once, so **no `@wip` was removed this pass**. A tag flips on a round that carries the
station, never on a union across rounds.

**Station 7's numbers** (round 045232Z, ceiling raised so the run reported lag instead of a
deadline): rebuild requested → generation published **59,974 ms** (one sweep), then the fresh
generation's canonical rows matched the live generation's exactly with zero pending members.
The old 210 s budget was not waiting on the rebuild at all — it was waiting on this scenario's
own acceptance: **212,471 ms** to the application row, **272,528 ms** to the republished tally.
It expired two seconds before the row appeared. Station 7 was a harness budget, not a substrate
defect.

**Why the budgets are now derived.** Discovery is a rotation: `MAX_MEMBERS_PER_SWEEP = 8`
members per `SWEEP_INTERVAL_SECS` tick, and `publish_generation` republishes only on a tick
ending with nothing unvisited and nothing pending. So an act lands within `ceil(N/8)` sweeps
plus a clean sweep — and N is every content record and every discovered correction this peer
has ever seen. It grew 14 → 44 → 127 members over the day's rounds, and the measured lag grew
with it (106.6 s, 212.5 s, 252.6 s, then 332.8 s at N=44, exactly `ceil(44/8)` sweeps). At the
60 s product default a single act passed 25 minutes by the fourth round. The lane pins
`ELOHIM_FEEDBACK_SWEEP_SECONDS=5`; the budget is a 300 s DHT-propagation floor (measured 2-3
min, which no sweep setting accelerates — dropping the floor made four previously-passing
stations fail) plus `2 x (ceil(N/8) + 2)` sweeps read from the peer's own live environment.

**Environment this round** (added to the previous section's list):

```bash
MESH_RESTART_ENV_OVERLAY="ELOHIM_TEST_FEEDBACK_CRASH_ONCE=1 ELOHIM_FEEDBACK_SWEEP_SECONDS=5" \
  just mesh storage-restart matthew jessica james
```

- Without `ELOHIM_TEST_FEEDBACK_CRASH_ONCE=1` on jessica's peer, station 4 cannot pass: the arm
  file is written but never consumed (`feedback_projector/rebuild.rs:50`).
- `MESH_RELAY_BIN=/projects/.claude-config/tools/iroh-relay-1.0.3/bin/iroh-relay` is required on
  this branch — `hc-mesh.sh` refuses to start without a relay and does not auto-detect one.
- `STORAGE_BIN` must be named: this branch's `hc-mesh.sh` defaults to the **dev** pool family and
  the worktree binary lives in the **sprint** family.
- `just mesh prologue` was again NOT run. It needs built Angular dists, and this worktree has no
  `elohim-library` build (`elohim-core/register` unresolved by the app bundler). These stations
  author their own content and read peer HTTP, conductor calls and SQLite directly.

## Stations 1 and 2 — the product surface that unblocked them (rust-architect, 2026-09-07)

Both stations passed on the household mesh in one round. Receipt:
`genesis/a2o/reports/accountable-correction/mesh-20260907T202913Z/` (`cucumber.json`,
`cucumber.html`, `run.log`, `sprint-report.json`) — cucumber EXIT=0, 5 scenarios / 57 steps, all
passed in 4m23s (stations 1, 2, 4, 6, 8; stations 3, 5 and 7 keep `@wip` and were not in the run).

**Station 1 needed a coordinator surface, not a fixture.** `create_feedback_signal` committed the
entry and both index links in one call, so a link could never be published later than the act.
Two coordinator-only additions (DNA hash unmoved, commit `a2055c21c`):

| Function | Input | Returns |
|---|---|---|
| `publish_feedback_signal_target_link` | `{ feedback_action_hash }` | `{ feedback_action_hash, target_action_hash, link_action_hash?, already_published }` |

plus an additive, defaulted `defer_target_link: bool` on `create_feedback_signal`. The extern takes
**no caller-supplied target** — it reads the act's own `target_cid`, so it can restore an index and
can never forge one — and it is idempotent (`already_published: true` rather than a second edge).
Its product case is index repair: delete-link validation is permissive, so an act whose edge is gone
is still valid, still fetchable by reference, and invisible to every peer that does not already hold
the reference.

**Station 2's blocker was not timing — the notification plane had admitted nothing, ever.**
`EprAtomService::with_origin_dna_hash` existed and was called from nowhere, so every peer refused
every `feedback-signal` notification with `no local content DNA hash bound`. Both transports now
bind it from the `HcClientRegistry` they already hold, and the receive path calls T6's
`admit_notified_signal` (`epr_atom_service.rs:624`) instead of its own inline `add_member` — so a
notified act joins the durable subscription set **and** goes Hot in the live `SweepScheduler`. What
the station measures is that: a member whose `source` is `notified` is swept within two sweeps of
admission, against the `ceil(N/8)` rotation it would otherwise have waited for (reported rather than
asserted when the peer's whole set fits in one sweep).

**`ELOHIM_TEST_FEEDBACK_INGRESS=1` — a second ingress to the real receiver.** `GET`/`POST`
`/admin/test/feedback-notify` on a storage peer; 404 when unarmed. `GET` reports
`{ armed, kind, originDnaHash }`. `POST` takes `{ signal: <semantic FeedbackSignal, camelCase, with
actRef>, from? }`, encodes it with `rmp_serde::to_vec_named` (the encoder a real sender uses), hands
it to the **same** `EprAtomService::handle(IntegrityNotify { kind: "feedback-signal" })` both
transports call, and returns that receiver's own verdict verbatim plus `originDnaHash`. It exists
because a household mesh has no peer in another content space: the foreign-DNA refusal has to be
*delivered* and *observed*, not argued from an absent row.

**Running this lane from a worktree needs a full mesh restart, not a coordswap.** The DNA hash is
path-dependent, so `happ_manager::lineage_mismatch_error` refuses `update_coordinators` between a
worktree-built bundle and a mesh installed from the main checkout — measured against the running
peer: role `lamad` drifted with `dnaHashMismatch`
(`installed=uhC0keuLMYBe0sj4ZqLvIyuVqCXcxPL3PbHyiqx40UGgBh-bTbBOl`,
`bundle=uhC0k8pow3EM3IVN9lZhCBlgnwyKrReAAXLKTEKnEFcPJQxjPZLNm`), while the other four roles showed
zero drift. Rung-1 coordswap is a main-tree vehicle. From a worktree:

```bash
STORAGE_BIN=<sprint pool slot>/debug/elohim-storage \
MESH_HAPP_PATH=<worktree>/elohim/holochain/dna/elohim/workdir/elohim.happ \
HOLOCHAIN_BIN=/projects/.claude-config/tools/hc-fork-25dd2d0be144/bin \
MESH_RELAY_BIN=/projects/.claude-config/tools/iroh-relay-1.0.3/bin/iroh-relay \
ELOHIM_FEEDBACK_SWEEP_SECONDS=5 ELOHIM_TEST_FEEDBACK_CRASH_ONCE=1 \
ELOHIM_TEST_FEEDBACK_INGRESS=1 just mesh start && just mesh wait --timeout 600
```

Pack that bundle by copying the four unchanged role DNAs beside your own `lamad.dna` in the
worktree's `workdir/` and running `hc app pack .`. One trap that cost a mesh cycle: rebuilding the
shared pool slot **without** `--features "p2p p2p-iroh"` makes `hc-mesh.sh` refuse every peer (it
greps the binary for the `p2p_iroh` marker) — on a shared slot the feature set is not sticky.

### Blind-reader findings on stations 1-2, deferred not fixed (2026-09-07)

A context-blind reader (a2o-story profile) returned **REVISE (light)**, no blockers, on the
station 1-2 Gherkin. Station 1 was called structurally sound. The findings, for whoever revises next:

- **MAJOR.** Station 2's central claim — that a notification is a *reference* and the peer must
  independently fetch and verify before applying — lives in a comment (feature lines 122-124), while
  the step that should carry it reads as a data match (`matches the fetched, verified record`). The
  binding does fetch through `get_feedback_signal_record` and does gate on `applied`, so the proof
  runs; the Gherkin just does not say so. Repair: promote it to its own step. That changes a step
  name and needs a re-run, so it was not taken in the round that made the station pass.
- **MAJOR.** `TargetToFeedbackSignal` is named only in a comment (line 106); the vocabulary describes
  discovery as scanning "corrections that have been linked" without naming the link type.
- **MINOR.** "envelope" appears in station 2's title and one comment but is never defined (the
  vocabulary defines NOTIFICATION); the "DNA hash is unchanged" assertion does not state the threat it
  guards (a foreign envelope must never move which space this peer is in); "content record" is
  undefined in an otherwise complete vocabulary; `E2E_STORAGE_*` is unexplained in the Background;
  and each of these two scenarios carries two sub-claims with no structural marker between them.

