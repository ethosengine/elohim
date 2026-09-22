# Runtime-performance report governance

This document binds the local runtime-performance report/compare/trend capability to the existing
verification spine. It is a governance contract, not a claim that a live household or fleet has
passed.

## Canonical answer shape

Each assertion is the existing protocol-owned `CheckWitness`:

`{ checkId, outcome: passed | failed | skipped, summary, observed }`

The stable `checkId` is `runtime-performance`. A measured threshold breach is `failed`; a valid
sample under the declared threshold is `passed`; missing, timed-out, cancelled, malformed, or
otherwise unusable evidence is `skipped` with the reason in `summary`. There is no fourth outcome
and no `warn` outcome. If two otherwise valid observations disagree because their declared
operator cohort or measurement environment differs, the comparison is a canonical
`Decision::Refer(ContestedEvidence)` question, not a fabricated regression or pass.

The report must preserve the declared operator cohort, and a comparison must preserve its
explicitly requested regression threshold beside the observed values. A threshold is comparison
policy, not a property silently invented for an old capture.
The cohort is a local/private diagnostic scope: it is not a DHT entity, does not mint an identity,
and does not create a sync message or durable protocol record. A report/trend is reconstructed
from local evidence; it is not a new decision registry.

## Guarantee matrix

| Guarantee                                                       | Answer                        | Evidence boundary                                                                                                                                         |
| --------------------------------------------------------------- | ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Sampled CPU/resource cost is distinct from actual call counters | **required**                  | `performance-prometheus.test.ts`; raw process-resource witness remains an input, not a second verdict vocabulary                                          |
| Declared operator cohort is carried into comparison             | **required**                  | `runtime-performance.test.ts`; comparison refuses an absent or incompatible cohort                                                                        |
| Declared threshold is carried into comparison                   | **required**                  | `runtime-performance.test.ts`; no default threshold may turn an omitted budget into zero                                                                  |
| Missing sample is not zero                                      | **required**                  | `runtime-performance.test.ts`; missing evidence yields `skipped` / nonzero CLI result                                                                     |
| Timeout and cancellation are not successful samples             | **required**                  | `performance-prometheus.test.ts`; the reason remains visible in the witness summary                                                                       |
| Invalid counters/windows are not trends                         | **required**                  | `performance-prometheus.test.ts`; invalid input yields `skipped` or a refused CLI result, never a plotted zero                                            |
| A valid threshold breach is a regression                        | **required**                  | `runtime-performance.test.ts`; emits `failed` with observed evidence                                                                                      |
| Incomparable valid samples refer rather than collapse           | **required**                  | `runtime-performance.test.ts`; uses existing `ContestedEvidence`, no new enum member                                                                      |
| Live household/fleet pass                                       | **N/A for this source story** | The feature is `@wip @act:host`; no live claim is made by these docs or unit tests                                                                        |
| Rust seam-registry registration                                 | **partial / explicit gap**    | The canonical seam registry schema requires a Rust crate and source locations; this TypeScript CLI has no valid Rust owner. Do not invent a crate or row. |

## Placement and scope

The root `just status` surface may gain a status _view_ for reading an existing report/trend, but
this document does not authorize a new public verb or direct capture from a status command.
Capture remains a bounded, explicit observation against an already-running local household.

The existing `idle-is-free` habit remains the owner of the idle budget and its 300-second household
story. This capability supplies evidence and comparison; it does not flip that habit, raise its
budget, or replace its A2O check.

## Native registration note

### Concern-canon accounting

These answers use `.claude/epr-meta/concerns.yaml` and the C2/C6a enforcement guarantees in
`.claude/epr-meta/policies.yaml`. Implementation-bound answers remain partial until their tests
and independent review complete; none of these grants authority over a runtime.

| Concern | Status at design | Binding                                                                                                             |
| ------- | ---------------- | ------------------------------------------------------------------------------------------------------------------- |
| C0      | partial          | Local operational measurement/projection, not DHT truth or resource allocation.                                     |
| C1      | n-a              | No candidate election or locally authored head adoption.                                                            |
| C2      | n-a              | No protocol head selection; baseline/candidate are explicit CLI arguments, not latest-head inference.               |
| C3      | partial          | Each CLI invocation terminates; finite input, scrape, and sampler bounds.                                           |
| C4      | partial          | Missing/reset/truncated/invalid measurements retain reasons and cannot become zero.                                 |
| C5      | partial          | Read-time validation and process witness checks; operator cohort is a declaration, not proof of causal equivalence. |
| C6a     | partial          | Explicit duration/byte/file-count limits and bounded child processes.                                               |
| C6b     | partial          | Read-only reports are reproducible; capture refuses overwriting prior evidence.                                     |
| C7      | partial          | Coverage lists unavailable method outcomes, heap, I/O stacks and causal attribution.                                |
| C8      | partial          | Canonical checks preserve outcomes and observed units; samples and invocation counts remain separate.               |
| C9      | n-a              | No agent keys, lineage, or cross-identity joins; process PID/start-tick identity is an apparatus guard.             |
| C10     | partial          | Invalid input shapes/units fail explicitly; no coercion of unknown measurements into known fields.                  |
| C11     | partial          | Oversize inputs/response bodies are refused, never queued without bounds.                                           |
| C12     | n-a              | No remote actuation or network grants; paths/endpoints are explicitly selected by the local operator.               |
| C13     | n-a              | No social standing or lower-tier authority bypass is introduced.                                                    |
| C14     | partial          | Unexpected parser/collector errors are visible in diagnostics and exit status.                                      |

The SDO/RWA boundary is local evidence custody: a holder may see process and operation activity,
so captures are not published or merged across holons automatically. No prompt, payload, secret,
SQL bind value, or cross-namespace identity join is added by this reporting layer. Raw perf stack
bytes and existing endpoint labels may still be sensitive; reviewed summaries are the sharing boundary.

`elohim/sdk/schemas/v1/manifest/seam-registry.schema.json` and the in-tree registries are for
decision points owned by Rust crates. The runtime-performance implementation and its focused tests
are TypeScript under `genesis/a2o`; no valid Rust `crateRoot`, `sourceLocation`, or native contract
test exists for a row at this stage. The honest state is therefore an explicit gap, not a fake
registration. If a future native verdict predicate is introduced, register that predicate in its
actual owning crate at birth and cite its real contract tests.

## Remaining native measurement work

### Approved private capture controls (2026-09-21 continuation)

Design gate, recorded before implementing the new controls: a re-arm generation and a
disposable heap-capture run are **Ephemeral (C)** operational measurements. They are rebuilt
by taking a new measurement, never reconstructed as historical truth. Their source is the
local process and its private evidence files; neither requires a database. Process-instance
identity plus an opaque capture nonce names an operation, not content or a human identity.
Byte hashes verify evidence only. No integrity/coordinator function, DHT entry, head, signal,
Automerge projection, HTTP route, or peer transport is introduced. DNA hashes are unchanged;
head-plane cost is zero now and at one year. These controls reuse the conductor admin channel.
They are local diagnostic apparatus at every network stage; privacy and finite-work limits
never cheapen with stage. This is a runtime/footprint concern, not protocol head selection.

The user approved private re-arm and a disposable canary, with the active household and
preserved specimen untouched. Re-arm is default-off, enabled at process startup, and accepts
no caller-selected filesystem path. Each capture family admits one active generation;
closing remains busy until terminal output finishes. Old timers/runs cannot close or join a
new generation. Existing duration/event/identity/byte caps remain; a finite lifetime generation
budget bounds cumulative files. Outputs are exclusive, private, and directory-confined.
Workflow TRACE availability must be checked, not fabricated by an admin success response.

Heap capture uses jemalloc profiling, not a new allocator profiler. Jemalloc traversal is not
cancellable. A supervisor owns a freshly spawned isolated child and enforces deadline/byte
overflow by terminating only that child, with no restart. It never accepts an arbitrary PID.
A capped FIFO writer, EOF, native completion and parser validation are separate witnesses;
none alone certifies completeness. Partial evidence remains explicitly incomplete. Canary
launch must declare isolated network/state paths; canonical mesh paths are forbidden.

Per-dump deadlines must survive a blocked collector event loop, independently of the longer
whole-canary lifetime. The disposable-only native path therefore pins a startup-authorized
1..30 second dump bound and arms a fresh Linux POSIX monotonic timer delivering SIGKILL to
its own process before scheduling each admitted traversal. Kernel timer setup failure refuses
the operation; cleanup failure cannot emit a completed receipt. No timer targets another PID,
no permanent watchdog service is introduced, and the regular household path remains disabled.
The outer GNU timeout still limits total lifetime. A completed receipt records the phase deadline;
it does not establish collected bytes, parser completeness, or full heap coverage. Reaching a
deadline is terminal even if completion races signal delivery; disarming is not cancellation of
an already-pending signal. This follows the Linux [timer API](https://man7.org/linux/man-pages/man2/timer_create.2.html)
and its [deletion semantics](https://man7.org/linux/man-pages/man2/timer_delete.2.html).

The launcher also uses a fresh PID namespace with private procfs, alongside its network namespace.
Detached inner watchdog/parser groups must not outlive a killed outer worker. Making that worker
namespace init lets the kernel terminate namespace members when it exits; a network namespace
alone does not provide this containment. Namespace creation is an explicit local prerequisite and
refuses when unavailable, never raising host privileges or falling back onto the household. See
the Linux [PID-namespace lifecycle](https://man7.org/linux/man-pages/man7/pid_namespaces.7.html).

SDO/RWA test: a holder of raw evidence can observe process activity and sampled allocation
stacks. Files stay local/private, with no automatic publication, cross-holon join, SQL text,
bind values, or identity-namespace correlation. The admin mechanism is a host-operator scaffold,
not a notarized grant and not suitable for exposing through a public doorway. Graduation to
remote actuation requires a separately authorized, acting-node grant design; this change does
not imply that authority.

Concern answers at birth (all implementation claims remain **partial** until contract tests
and independent review): C0 partial — operational projection; C1 n-a — no election;
C2 n-a — no competing protocol versions; C3 partial — timed close and owned-child reap;
C4 partial — disabled/busy/refused/incomplete remain distinct; C5 partial — reader revalidates
identity, bytes and window; C6a partial — finite generation/time/event/byte budgets;
C6b partial — duplicate nonce refuses, evidence is never overwritten;
C7 partial — acceptance is not capture completion; C8 partial — typed outcomes counted for
admission and refusal, sample semantics explicit; C9 n-a — no human agent lineage;
C10 partial — strict versioned request parsing; C11 partial — immediate busy/budget refusal,
no queued captures; C12 partial — startup authorization plus existing private admin channel,
not a remote notarized grant; C13 partial — host scaffold and remote-grant successor above;
C14 partial — unexpected failures retained as bounded diagnostics, never a success fallback.
Native decision surfaces must register these answers in their owning crate's seam registry
at birth with real test names, or explicit null tests and a gap note before tests exist.

Source audit on 2026-09-20 identifies these boundaries; the CLI does not claim to fill them:

- Storage's custom `REGISTRY` in `elohim/elohim-storage/src/metrics.rs` now declares
  `process_start_time_seconds`, initialized once at metrics startup. This is a producer-instance
  marker, not a recovered OS process birth timestamp. The new source also measures completed
  `call_zome_timed` attempts by zome/function/class/outcome, after admission and separately for
  each retry. A dropped waiting future increments a separate counter without recording a
  completed duration or claiming that the conductor cancelled. These instruments still need
  a rebuilt runtime and fresh capture before they can fill any live evidence gap.
- Conductor `create_workflow_duration_metric` in
  `elohim/holochain-conductor/crates/holochain/src/core/metrics.rs` has workflow timing,
  but does not use its agent argument. A bounded workflow/cell/outcome run counter at
  `queue_consumer_main_task_impl` would supply the missing workload denominator.
- `hc.db.connections.use_time` and `hc.db.write_txn.duration` measure connection/transaction
  scopes, not individual SQLite statements. Statement timing needs a separately bounded,
  opt-in query-class instrument without SQL bind values.
- Kitsune2/iroh facts exist through `dump_network_metrics` and `dump_network_stats` in
  `holochain_p2p/src/spawn/actor.rs`; storage's `lineage_partition.rs` already consumes
  diagnostics. Queue/traffic watermarks still need an explicit time-series projection.

These are follow-on implementation points, not an assertion that the required instruments
have been added or that all runtimes are now attributable. The first report exercise found that
saved James/Jessica binaries already match and resolve leaf symbols: the initial unknown-heavy
ranking conflated missing unwound callers with unknown leaf symbols. Leaf ranking and caller
coverage must therefore be measured separately; a symbolized leaf alone cannot establish its
calling workflow.
