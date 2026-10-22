---
id: "backlog-compute-envelope-virtual-peer-contract"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Respecting limits as a compute contract: virtual peers in the test mesh run inside a declared envelope a consenting host lends them — the RAM guard's six fields are the missing fields of `delegates-compute`"
slug: "compute-envelope-virtual-peer-contract"
written: "2026-08-29"
author: "fable-5 session 2026-08-29 (operator steering: bonus points if 'respecting limits' becomes part of a virtual peer's compute contract)"
status: "backlog"
priority: "medium"
area: "a2o/mesh · shefa/compute-commitment"
domain: "protocol"
jobs: [elohim-genesis, elohim]
relatedNodeIds:
  - "habit:dataplane-convergence"
  - "memory:project_ram_guard_oom_group_kill"
  - "memory:project_rea_compute_commitment_primitive"
cites:
  - genesis/docs/architecture/rea-compute-commitment-primitive.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-04-compute-commitment-substrate-floor-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
  - genesis/data/timeline/backlog/mesh-prologue-cast-and-env-gaps.md
  - genesis/data/timeline/backlog/agentic-context-tooling-consolidation-queue.md
  - genesis/agentic/pool-policy.json
  - genesis/docs/content/elohim-protocol/architecture/2026-07-16-alpha-test-bench-compute-envelope.md
  - genesis/data/rakia/compute-capacity.schema.json
  - elohim/rakia/docs/plans/stage-2-canopy.md
tags: [rea, compute-commitment, delegates-compute, mesh, virtual-peer, cgroup, resource-envelope, shefa, rakia, a2o, p2p-gated, agent-agnostic, performance, perf-memory, trustful-declared, friction-mechanical, unit-once, phase-transition, lane-operator]
shift_objective: |
  Rung (a) only. Make `app/elohim-app/scripts/hc-mesh.sh` place each mesh peer's processes
  (conductor + storage + doorway) into a named cgroup-v2 sub-tree `<pod>/mesh/<peer>` with
  `memory.max` taken from a per-peer `compute_envelope` in the household fixture manifest
  (default: unbounded = today), `memory.oom.group=1` INSIDE the guest tree, and teach
  `genesis/agentic/bin/ram-guard` to read guest cgroups as shed units keyed by peer id (a guest
  over its envelope is shed whole and logged with the peer id, never by process-name heuristics).
  Done = `just mesh start` shows the per-peer cgroups in `ram-guard plan`; one peer given a
  deliberately tiny envelope dies alone under `just mesh recovery warm <peer>` while the other
  peers and the host's own sessions keep running; the shed event carries the peer id. No Rust,
  no DHT change, no new entry type.
---

# Compute envelope as a virtual peer's contract — genesis

**Where this came from.** On 2026-08-29 the devworkspace pod hit `memory.max` and the whole
workspace died — not "a process was OOM-killed" but *every* process and PID 1, because the
container cgroup carries `memory.oom.group=1` (kubelet's cgroup-v2 default). The trigger was
one `rustc` (4.3 GB) plus seven `rust-lld` linkers on top of three conductors. The fix that
landed the same day is `genesis/agentic/bin/ram-guard` + `.claude/hooks/ram-guard.py` +
`pool-policy.json` `ram` (see [[project_ram_guard_oom_group_kill]]). Stripped to its shape, the
guard is **six fields** — and they are exactly the fields a `delegates-compute` commitment
([the primitive](../../../docs/architecture/rea-compute-commitment-primitive.md)) does not
carry yet when what is delegated is *the right to run on someone's machine*.

## The six fields, and what each is in the contract

| guard field (host, today) | contract field (provider → recipient) |
|---|---|
| `memory.max` — a bound the host never chose but lives in | **provider's envelope** — what a host *consents* to lend: memory, CPU share, disk, egress, wall-clock window |
| committed = `anon+kernel+shmem` (page cache never counts) | **the measure** — bound what cannot be reclaimed; a guest is not charged for cache and the host is not surprised by it |
| the critical set the guard never touches | **provider's protected set** — its own conductor, storage, family sessions. Declared, never inferred |
| the tier ladder (what is disposable first) | **recipient's declared shed order** — "my compile is disposable, my chain state is not." Subsidiarity: host declares what it protects, guest declares what it can lose |
| soft / high / hard | **graded obligations** — soft: guest self-throttles · high: guest self-sheds (cooperates before it is killed) · hard: provider sheds it |
| `events.jsonl` + per-session banner | **the reciprocity ledger** — each shed is an `EconomicEvent` `bounded_by: <Commitment CID>` ("recipient exceeded envelope at T; provider shed it"); a guest's own "I had to defer" is a `FeedbackSignal`. Default accrues on-chain, not in a log nobody reads |

The guard is already the **substrate floor** of the compute-commitment design
([§1–2](../../../docs/content/elohim-protocol/architecture/2026-05-04-compute-commitment-substrate-floor-design.md)):
mechanical, deterministic, requires no AI, returns a verdict with a reason. The elohim ceiling
is what a guest does with the soft line — revise its own shed order, defer work, negotiate a
bigger envelope. The floor stands alone; the ceiling enriches.

## Two inversions the incident taught

1. **Group-kill is correct at the guest boundary and wrong at the host boundary.** What hurt
   us — `oom.group=1` — is exactly right one level *down*: each virtual peer in its own
   sub-cgroup (`<pod>/mesh/<peer>`) with `memory.max` = its envelope and `oom.group=1` inside
   it. A misbehaving simulation dies whole and clean; the host's conductor, storage and
   sessions never see it. The host's guard then treats "guest cgroup over its envelope" as a
   shed unit keyed by peer id — no process-name heuristics.
2. **In-kind vs REA is only whether the envelope is metered.** In-kind: a household runs the
   simulation as a gift — same envelope, no event. REA: envelope × wall-clock is a `use` event
   on a `compute` resource; shed events are the deductions. The shem fixture peer saying "run
   this test peer" is the *recipient* of a `delegates-compute` commitment from each consenting
   host, so every test run already carries the proof that it stayed inside what it was lent.

## P2P design gate (answered here so pickup does not re-litigate)

- **Entity class.** The envelope is an attribute of a manifest (SDK seam: what you ADD is a
  manifest field, `compute_envelope: {memory, cpu, protected: [...], shed_order: [...]}`) —
  not a new entity. The commitment is the existing `Mishpat::Commitment` with the existing
  `delegates-compute` discriminator; the shed is an `EconomicEvent` with `bounded_by`. **Zero
  new DHT entry types.** Rung (a) touches no DHT at all.
- **Identity.** Envelope: content-derived from the manifest CID. Commitment: entry_hash
  ([[project_mishpat_commitment_cid_is_entry_hash]]). Shed event: agent-composite (provider,
  commitment, timestamp).
- **Head-plane cost.** Sheds are rare by construction (a healthy mesh emits none); a metered
  `use` event per run is one item per test run — bounded by the run cadence, not the peer count.
- **Track.** This is participation-track work (how a node participates), never a seam
  addition; the atlas's disambiguator says a manifest → SDK seam.

## Rungs (cheapest first; each independently shippable)

- **(a) hc-mesh.sh sub-cgroups** — the `shift_objective` above. Composes with the mesh-prologue
  cluster (added there as an env leg) and with `ram-guard` as it stands.
- **(b) manifest field** — `compute_envelope` in the household fixture manifest and the
  virtual-peer app manifest; `ram-guard plan` renders guests by envelope. Composes with
  agentic-queue item 17 (capacity is a vector — the envelope is that vector's physical face).
- **(c) mint the commitment** — `seed-delegates-compute` already exists as a seed leg
  (`ALLOW_SEED_DELEGATES_COMPUTE=1`); extend its bounds with the envelope, emit sheds as
  bounded events, and bind a habit `respecting-limits` / `@concern:compute-envelope`: **green
  when no consenting host's soft line was breached by a guest in the run**, red on the first
  ledger event with a peer id, unwired until (a) lands. Retire-when: the platform (kubelet /
  devfile) carries per-guest envelopes itself.

## Rakia — where this concern is implemented next

Rakia's Stage 2 (Canopy) is *exactly* the provider/recipient relationship: `/rakia/claim` is a
peer announcing what it can build, `/rakia/dispatch` is a request to run a step on that peer's
machine, `/rakia/result` is what comes back — and today none of the three carries an envelope.
The rakia plan now names the split (`elohim/rakia/docs/plans/stage-2-canopy.md`, "compute
envelope"; submodule — the operator commits it in the rakia repo):

| protocol | carries |
|---|---|
| `/rakia/claim` | provider's envelope (`memory_Mi`, `cpu_m`, disk, egress, wall-clock — the units `compute-capacity.json` `stewardCommitments[].limits` already uses) + protected set |
| `/rakia/dispatch` | the step's requirement (`buildExecutor.envelope` — a schema-version bump, `buildExecutor` is `additionalProperties: false`) + the recipient's shed order |
| `/rakia/result` | sheds/breaches as `EconomicEvent` `bounded_by` the commitment; defers as `FeedbackSignal` |

The governance side already exists: `genesis/data/rakia/.epr-meta` couples the capacity ledger
to `epr:alpha-test-bench-compute-envelope` (governance) and `epr:rea-compute-commitment-primitive`
(value), and its `test-bench-aggregate-capacity` validator refuses a portfolio that exceeds the
promoted envelope. This entry is that same rule one level down — per guest inside a host instead
of per portfolio inside a cluster — so a reader of either surface finds the other.

- **(d) rakia claim/dispatch/result carry the envelope** — after (a) proves the mechanism and
  Stage 1 lands; it is the first Canopy sprint's first bullet, not a new sprint.

## Scenario sketch (goes to `genesis/a2o/features/` through the blind-reader loop at pickup)

```gherkin
@concern:compute-envelope @requires:household-nodes
Feature: A test peer respects the envelope its host lent it
  Scenario: a guest over its envelope dies alone
    Given host matthew consents to run guest peer "james" under a 512Mi envelope
    And james's declared shed order is [compile, projection, chain-state]
    When james's projection exceeds the envelope
    Then james is shed whole, the host's own conductor and sessions are untouched
    And the ledger carries one event bounded by matthew→james's delegates-compute commitment
```

**Readiness.** (a) is a one-session scripts change, verifiable on the local mesh. (b)–(c) wait
on (a)'s evidence and on the seed leg for `delegates-compute` being exercised in the Prologue.

## 2026-10-22 harvest — a consenting peer also needs a compatible execution flavor

This adds stations before `measure-runs-on-a-peer`'s existing finish line: a real
peer executes the authorized feature and returns verifiable results. It does not
replace that finish line with a successful image build, change rung (a) above, or
claim Rakia already selects deployment flavors. Adam's fixture-provider native identity issues the provider grant. Matthew's
accountability as steward is operator-declared; this is not a separately
notarized Matthew-to-Adam grant.

The concern-routing atlas (§3.1–3.3) separates hardware capability, OS/packaging,
and runtime composition from network participation. A task needs independently
satisfied constraints: CPU instructions; loader/library ABI; enforced memory,
CPU and deadline bounds; current authority; and the protocol contract required
by its peers. Spare RAM cannot satisfy missing instructions. A compatible ELF
cannot authorize a task. A successful task cannot establish interoperability
with a different flavor unless that interaction was measured.

### Evidence and its boundary

- Edge **1683** exposed `Fatal glibc error: CPU does not support x86-64-v3`
  before worker bootstrap in the UBI10-derived image. The exact old published
  `holochain` binary also raised SIGILL on an Intel Xeon E5-2667 Sandy Bridge
  host (x86-64-v2, AVX present, AVX2 absent). Merely rebasing the image while
  retaining those native binaries was therefore insufficient.
- The missing diagnostic observation became available after an operator applied
  narrow Jenkins RBAC: `pods/log get` and `pods/exec get/create` for only
  `elohim-adam-alpha-0` in `elohim-alpha`. This enabled startup evidence; it did
  not create native `measure-stage` authority. Exec permission still covers any
  container/command in that pod, as the owning RBAC manifest documents.
- Edge **1685 SUCCESS** rebuilt the forked `holochain`/`hc` pair, Ark and executor
  from source `04ef84c0798e63d5e0a3e95b55f4a75c71077ac5` on the existing owned
  x86-64-v2 base digest
  `67c1a73215731295a1c5f8c91b0d993981eae6c302241da946a075f13aca4a30`.
  Final image digest:
  `742f3ded663a429fcec704e63954fd0bd68476cdd72f83a0673c0724a99c5a32`.
  The hApp and source identity remained pinned while runtime bytes changed.
- **52 exact-published-binary CLI/ELF checks passed on the observed host using
  its userspace**, recorded in the private campaign receipt
  `genesis/local-dev/jenkins-native-bdd/live-evidence/cpu-0724a99c5a32/proof.json`.
  This proves those sampled binaries/checks on that CPU and host userspace;
  it does not prove startup of the full target image, all runtime code paths,
  native G2, or an executed BDD job on Adam.
- At this harvest, actual target-image bootstrap/native G2, automatic flavor
  selection, and mixed-flavor peer interaction remain **unmeasured**. Different
  runtime digests with unchanged source/hApp illustrate a packaging distinction,
  not a successful cross-peer protocol test.

### Missing stations ready for story refinement

All four rows belong to the `measure-runs-on-a-peer` chain. They preserve the
existing completion assertion and make its prerequisites observable.

| Between existing truths | Missing node and its probe | Current evidence |
|---|---|---|
| Provider offers capacity → task is admitted for launch | Select a digest-pinned flavor whose ISA and ABI requirements fit observed provider capabilities; refuse unsupported or unknown compatibility before guest launch. Probe incompatible-v3/compatible-v2 candidates on the same v2 peer and record the selection/refusal. | The incompatibility and compatible build/host CLI checks are observed; Rakia selection/refusal behavior is unmeasured. |
| Compatible flavor → authorized bounded execution | Independently verify current grant, enforced resource envelope and task deadline; reject a resource or authority mismatch even with compatible binaries. Probe each failure independently. | Existing authority/resource mechanisms are separate; this joint heterogeneous-peer scenario is unmeasured. |
| Image published → peer completion accepted | Observe the exact target image bootstrapping on Adam, then execute one authorized feature and verify its native request/completion/receipt/report binding. Probe the real worker and exact returned Cucumber artifact. | Publication and host-userspace checks pass; target bootstrap/G2 and job execution remain pending. |
| Each flavor can run → diverse peers participate together | Exercise the declared common protocol across v2 and v3 deployment flavors without changing identities or weakening content verification. Probe both directions and compare returned content identity/bytes. | Mixed-flavor interoperability is unmeasured; common source/hApp alone is insufficient. |

These are prose skeletons, not executable coverage or a new schema/API proposal.
At pickup, refine into the existing delegated-compute stories
`ci-runner-stewarded` / `peer-executed-stage` and the dataplane peer stories,
through the required a2o blind-reader loop. Reuse existing native task/grant and
receipt contracts before proposing new capability fields or selection machinery.

```gherkin
@wip @regression @concern:measure-runs-on-a-peer
Scenario: A capable older host is not given an unsupported execution flavor
  Given Matthew delegates a bounded job to the Adam fixture provider
  And the observed provider CPU supports x86-64-v2 with AVX but without AVX2
  And one pinned flavor requires x86-64-v3 while another fits its ISA and ABI
  When Rakia evaluates the job's execution candidates
  Then the v3 candidate is refused before guest launch with the unmet constraint
  And any selected candidate is bound to the compatible immutable runtime digest
  And missing compatibility evidence produces refusal rather than a guessed match
  # Constraint: CPU family, ISA and runtime ABI are distinct from spare capacity.
  # Parameters: E5-2667/Sandy Bridge; x86-64-v2; AVX present; AVX2 absent.

@wip @concern:measure-runs-on-a-peer
Scenario Outline: Compatible packaging does not bypass independent admission bounds
  Given Adam has an execution flavor compatible with its observed ISA and ABI
  And Matthew's reviewed job requests 8000 millicores and 25769803776 bytes
  But <constraint> is not satisfied at launch
  When the job is considered for execution
  Then no guest launches and the refusal names <constraint>
  And no successful completion is invented
  Examples:
    | constraint                     |
    | current native grant authority |
    | enforced resource envelope     |
    | task deadline                  |
  # These resource values are the commissioning slice, not every peer's minimum.

@wip @concern:measure-runs-on-a-peer
Scenario: The published compatible flavor executes the actual job on Adam
  Given Adam has bootstrapped the exact reviewed runtime image on its own host
  And Matthew is the accountable steward by operator declaration
  And Adam's finite native grant authorizes the Jenkins runner and this task
  When Jenkins submits the pinned peer-mesh feature to Adam
  Then Adam executes the task within its enforced resource and time bounds
  And Jenkins verifies the native completion and executor receipt against the request
  And Jenkins publishes exactly the returned Cucumber report with its real verdict
  # CLI/build checks cannot substitute for this on-target execution station.

@wip @concern:measure-runs-on-a-peer
Scenario: Different deployment flavors preserve the shared participation contract
  Given one participating peer runs a pinned x86-64-v2 flavor
  And another runs a pinned x86-64-v3 flavor of the same declared protocol
  And their identities and authorized participation are retained
  When they exchange the same declared content in both directions
  Then each verifies the other's content identity and bytes under that protocol
  And the evidence records both runtime digests and each observed direction
  # CPU/ABI flavor differences do not justify re-keying or weakening verification.
```

Operational pickup: capture observed CPU features and the target runtime's
loader/library requirements alongside existing immutable source/image evidence;
retain refusal reasons and startup diagnostics without exporting credentials.
Flavor compatibility, protocol compatibility and resource/authority admission
must remain separately measurable. This harvest authorizes no new placement
runtime and marks no habit green; the active frozen-source commissioning proof
continues unchanged.

**Next bounded task after native BDD commissioning:** establish a repeatable
x86-64-v2 / x86-64-v3 flavor pair using existing capability and build contracts.
Acceptance requires (1) explicit architecture, required ISA/optional hardware
features and runtime ABI constraints for each candidate; (2) evidenced compatible
variant selection, including refusal of unsupported or unknown combinations;
(3) actual bounded job execution on representative peers for both variants; and
(4) measured mixed-variant protocol exchange in both directions. Preserve source
identity separately from runtime digest/ABI evidence and from protocol/version
and interaction evidence. The four stations above are the initial slice, not a
claim that all architectures work. ARM and other optional-feature combinations
are extension points requiring their own observed build, execution and
interoperability proofs. Current work still finishes only when the native BDD
request executes on Adam and its authentic results return to Jenkins.

**Human acceptance for that next task (requirements, not implemented claims).**
Arks and berths should make the initial v2/v3 slice usable across consumer
hardware without making people manage routine ISA selectors. Establish compatible
local execution before launch. Selecting a compatible variant must retain the
human's identity, local data, drafts and progress; it must not require a new
identity or a doorway for functions the person's own node can already perform.
Verify locally held published bytes before use, including offline use. Retain
prepared local job intent while execution on a particular peer is waiting.
Where a hosted key must sign, show that signing is waiting honestly; a visitor
session must not imply durable local persistence it does not provide.

An authorized other device or peer adds capacity and learns only the declared
task payload and metadata. It must not become a new dependency for previously
local functions. Losing that peer preserves local work and exposes a meaningful
waiting, unsupported or unknown remote outcome state, never a fabricated completion.
Retain the original task identity for receipt reconciliation through existing
recovery contracts; losing contact does not authorize an automatic rerun. Acceptance checks
must exercise those transitions alongside variant choice, without weakening the
initial v2/v3 execution and mixed-protocol proof requirements. Doorway-independent
local use does not mean a remote provider can execute while unreachable, nor does
local intent preparation replace required native signing or consent.

Build selection itself has no human draft editor or visitor persistence policy;
these requirements apply where selection, Ark/berth launch and execution results
meet existing human workflows. A build-time flavor check therefore cannot prove
local-first UX. Reuse existing identity, intent, task and receipt contracts at
pickup; this harvest introduces no new state/schema entity. Any proposed data
entity or capability-contract redesign must pass the P2P design gate then.

**Future commons placement and performance acceptance for the same v2/v3 slice.**
Peers may fetch and cache the compatible immutable artifacts they need instead
of holding every variant. Artifact bytes are nonrival; the storage, bandwidth
and maintenance facility is rival and remains subject to its existing member
charter and provider consent. This is a requirement for existing commons work,
not a new pool, schema or policy, and establishes no charter-gate pass without
its declarations and probes.

Fetch the compatible digest-pinned bytes, verify them, then use them locally.
Pool loss must not remove already active or offline local capabilities. Keep a
bounded subset whose placement has observed failure-domain independence; choose
retention and redundancy from measured need rather than inventing a replica
count or cache limit here. Authority to announce the latest release or withdraw
an artifact remains separate from possession/reuse of immutable bytes. Public
artifact reach does not make fetch history or reference metadata public.

Measure cold and warm paths separately: bytes held and transferred; fetch,
unpack, verification and startup latency; eviction/refetch cost; and energy
where instrumented. Distinguish human-blocking local work from background waits.
Reuse valid verification evidence where its binding still holds, without skipping
required authorization, signatures or current revocation checks. These are costs
to measure, not benchmark results or a claim of support for all architectures.

### Interrupted fixture initialization station (2026-10-22)

Chain `measure-runs-on-a-peer`, between compatible worker startup and ready private
household: an interrupted first initialization must continue without replacing existing
conductor identities or treating lost storage as never-created storage. Edge #1690
retrieved the original log: `ss` was hidden by sanitized PATH, and disabled membership
passed an empty optional path to doorway; conductor sandboxes existed, storage had not
reached its first directory creation. The byte-bound continuation checks the retained
log, rejects prior-storage witnesses, and compares installed agent/cell identities before
and after readiness. Its unresolved attempt refuses replay. Probe:
`hc-mesh-initialization-continuation.test.sh` plus actual provider before/after identity
and readiness evidence. Current live state **RED**: focused fixtures verify the repair,
but live continuation, G2 and the native three-scenario completion remain unproven.

Two further stations were exposed before that live attempt:

- Chain `measure-runs-on-a-peer`, between retained conductor state and the
  before/after identity comparison: interpret the pinned executable's actual
  identity output. `hc client call --port <admin-port> list-apps` converts native tuple cell IDs to
  `{dna_hash, agent_pub_key}` objects; a fixture modeled on the raw websocket
  response incorrectly passed the earlier parser tests. Probe the actual helper
  with the pinned `hc`, plus malformed-cell, agent-mismatch and DNA-change
  refusal fixtures. Source `b8d1f8295` passes those checks and a read-only call
  against three local conductors, five provisioned cells each. The private
  `continuation-local-app-identities.jsonl` receipt records that local evidence.
  Current state: **GREEN locally; live Adam continuation unmeasured**. Source
  `6b06b6c6` is barred from activation despite its successful mocked checks.
- Chain `measure-runs-on-a-peer`, between a source correction and publication
  of its execution flavor: make the cost of rebuilding unchanged native inputs
  observable. Edge **1692**, observed at 20:54 UTC, cached the base image but
  reran the fork COPY, toolchain installation and native compilation with the
  same conductor pin `96bcd2bee215` and base digest as Edge 1685. This delayed
  the corrected source behind a superseded assembly. Probe two assemblies
  whose complete native build inputs match, recording cache hits/misses,
  compilation time and resulting artifact provenance; changed inputs must
  invalidate reuse. Current state: **RED for observed reuse**, with final
  Edge 1692 duration still unmeasured. Edge 1685's saved log measures the fork
  build RUN alone at **2773.3 seconds** (the conductor release build reports
  35m31s, the CLI build 10m39s); this is one cold build, not a general benchmark
  or total pipeline duration. The saved pod declaration gives the OCI BuildKit
  worker no persistent state mount, and the packer supplies no cache import or
  export; the base check earlier in the same pod explains the cached base.
  Separately, a tiny reproduction of the packer's Git snapshot steps produces
  different `.git/index` and `.git/logs/HEAD` bytes for identical pinned source
  and tracked-file timestamps (private receipt:
  `genesis/local-dev/jenkins-native-bdd/cache-snapshot-variance.json`). Both files enter the fork COPY. Cross-pod cache
  retention and stable input bytes therefore need separate probes; making only
  one reliable cannot establish a warm path. Reusing an artifact still requires
  compatible target runtime evidence and current task authority.

These are refinement inputs for the existing stories, not additional executable
coverage. Keep compile/reuse, artifact fetch/cache, on-target compatibility and
authorized execution as separately measured costs in the same performance work.
