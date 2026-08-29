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
tags: [rea, compute-commitment, delegates-compute, mesh, virtual-peer, cgroup, resource-envelope, shefa, rakia, a2o, p2p-gated, agent-agnostic]
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
