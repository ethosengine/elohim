---
title: "The workspace is a berth — carrying capacity, moorings, leases and deliberation notes as the local slice of tevah"
id: workspace-berth-carrying-capacity-design
status: Active
class: substrate
context-tier: disclosed
steward: rust-architect
graduation-trigger: the quota verb (io-guard's sensor + ladder) runs inside `ark` with `io.max` on a delegated cgroup controller AND a mooring is a signed `delegates-compute` commitment asserted by the runtime, not a self-typed flag — at that point berth + io-guard retire by their own RETIRE_WHEN
created: 2026-09-03
domain: dev-system (agentic workspace) · tevah compute envelope · REA accounting
topic: [berth, carrying-capacity, io-guard, ram-guard, write-budget, delegates-compute, moorings, leases, deliberation, tevah]
serves:
  - runtime-death-witnessed
  - dev-system-equilibrium
cites:
  - "compute-envelope-tevah | Tevah | sha256:ac9364d4b024290f | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
  - "rea-compute-commitment-primitive | rea-compute-commitment-primitive | sha256:3ea123e3a9796449 | path: genesis/docs/architecture/rea-compute-commitment-primitive.md"
  - "holochain-0-7-upgrade-guide | Holochain 0.7.0 Upgrade Guide | sha256:30ca33eb616ef0b1 | path: genesis/docs/superpowers/plans/2026-09-02-holochain-0-7-upgrade-guide.md"
memory_anchors:
  - project_devspace_recovery
  - project_tevah_compute_envelope_canonized
  - project_rea_compute_commitment_primitive
  - project-eprfs-witnessed-interaction-primitive
---

# The workspace is a berth

## Why this exists (2026-09-03)

Three agent sessions and one human shared this workspace today. They coordinated the household
mesh, the one heavy cargo slot and the disk by sending each other chat messages: "mesh taken",
"mesh free", "hold every cargo command", "seed done". It worked, and it is the wrong organ. The
operator's ruling: *the limits are our carrying capacities at the workspace level; registering
who is doing what, and exactly which model from which lab, has to be native; the communications
and the space used for them are part of the deliberation process — ephemeral because low stakes,
but they have a place in the protocol.*

The same day the workspace pod was **swapped** (not restarted) because a mesh seed plus a cargo
build wrote 130–270 MB/s to the NVMe the cluster datastore shares; dqlite lease writes stretched
to 3–9.7 s, the devworkspace controller lost leader election and re-rendered the Deployment
without its secrets. No process crashed. CPU had 16 cores free. The workspace's real carrying
capacity was a **write budget nobody had declared**, so nobody could refuse work against it.

## What already models this (inherited, never re-minted)

- **tevah / `ark`** (spec §3, §5.2, §5.3): a *berth* is a blade with declared quota; the envelope
  keeps *intents* (write-ahead records of actions it will take) and *incidents*; §5.2 says the quota
  verb is "ram-guard's shape, lifted" — committed-not-gross, protected set, typed tier ladder,
  never-shed-unknown, re-measure between kills, one witnessed event per kill; §5.3 says accounting
  is one record type: a commitment before, interval rows during, breach events, a terminal receipt.
- **`delegates-compute`** (`rea-compute-commitment-primitive.md`): bounded authority from a
  provider agent to a recipient agent is one `Mishpat::Commitment`; every event the recipient emits
  is `bounded_by` it. A human letting a model work in their workspace *is* this delegation.
- **witnessed interaction events** (eprfs): a light runtime witnesses what agents do on
  content-addressed objects; low-stakes local events stay local, adoption by a peer is what carries
  reach.
- **ram-guard** (`genesis/agentic/bin/ram-guard`): the existing shedder, with the process
  classification (critical / tier1 compile / tier2 JS+test / tier3 dev servers / unknown) every
  guard in this workspace reuses.

## The slice that landed (local dev, 2026-09-03)

Two tools, one policy file, one hook line, no cargo build required:

| tool | what it is in tevah terms | store |
|---|---|---|
| `genesis/agentic/bin/berth` | the berth's moorings, leases and notes | `$CLAUDE_CONFIG_DIR/berth/` (survives pod swaps, never `/tmp`, never git) |
| `genesis/agentic/bin/io-guard` | the write-budget quota verb, user-space until `io.max` is delegated | `$CLAUDE_CONFIG_DIR/io-guard/` |
| `pool-policy.json` `io` + `berth` blocks | the declared capacities and thresholds | git |
| `.claude/hooks/ram-guard.py` SessionStart | ensures both daemons, auto-moors the session, one status line each | git |

### Records, in the field shape of their protocol homes

| local record | fields | protocol home (graduation, not redesign) |
|---|---|---|
| **mooring** | `session`, `recipient {model, lab, runtime, session}`, `provider` (the human principal), `bounds {write_set, ttl_s}`, `task`, `pid`, `last_seen` | a `delegates-compute` `Mishpat::Commitment`: provider = principal, recipient = the agent, bounds = the write set + ttl. Identity: agent-scoped composite (principal × session × model). |
| **lease** | `resource`, `holder`, `since`, `ttl_s`, `note`, `action: use` | a REA commitment to `use` one unit of a berth resource for a window (tevah §5.3 "commitment before"). Capacity 1 by default: `mesh`, `cargo` (= `io.max_concurrent_heavy`), `disk-heavy`. |
| **note** | `session`, `to`, `text` | a witnessed interaction event between moored agents; low stakes ⇒ local only, never notarized, kept for the deliberation trail. |
| **shed** | `guard`, `action` (`pause` / `kill` / `would-…`), `target`, `level`, `holder`, `mode` | tevah §5.3 breach event, `bounded_by` the holder's lease; the reciprocity ledger. |
| **refuse** | `resource`, `session`, `holder`, `since` | honest absence: the answer to a claim on held capacity is *who holds it*, never a queue. |

### P2P design gate (summary; the full template is in the tool docstrings)

- **Classification:** every record is **Ephemeral (C)**. Delete the store and the next
  `moor`/`claim` plus the live process table rebuild it; the workspace itself is the source of
  truth. No `dht_anchor_hash`, no route, no table, no sync message. Head-plane cost: zero.
- **What would be Notarized when it graduates:** the mooring as a `delegates-compute` Commitment
  (existing mishpat entry type, DNA-hash-neutral — the discriminator is an action string) and the
  per-period *composite* of sheds (one head per period, never per shed). Interval rows stay amber
  Category-C exactly as tevah §5.3 prescribes.
- **Network stakes:** Simulacra by declaration — this is a developer workspace. Floor-protected
  regardless of stage: the refusal (a claim on held capacity is refused, not silently queued) and
  the never-shed set (critical processes and unknown ones).
- **Address strategy:** agent-scoped composite for moorings and leases (principal × session ×
  resource); notes are ordered by time in an append-only ledger and have no address of their own.
- **Concern canon (Step 4), for the guard's decision predicates:** C0 the guard lives in the
  workspace plane and says so; C3 liveness — moorings expire, stale holders are taken over on the
  record; C4 honest absence — `refuse` names the holder, `would-kill` in observe mode is logged as
  such; C5 evidence-not-authority — a lease is a claim, io-guard measures the writes regardless of
  who claims to hold what; C6a bounded work — at most one action per tick, dwell before action;
  C8 observability-per-decision — one event per pause/kill/resume in two ledgers; C11 externally
  imposed backpressure — host PSI raises the level even when our own rate is low; C13 graduated
  authority — pause before kill, tier-1 before tier-2, never tier-3 or critical. No seam-registry
  row: `genesis/agentic/bin` is not a crate; the census does not read it, so this paragraph is the
  registration until the verb moves into `ark` (below).

### What io-guard decides

Level = the higher of the container's own write rate (cgroup `io.stat`, 30 s window) and the
host's `/proc/pressure/io full avg10`. Thresholds 60/100/160 MB/s and PSI 10/20 %. Nothing happens
without 20 s of dwell. `high` pauses (SIGSTOP) the tree writing the most, tier-1 only, one per
tick; `hard` kills tier-1 and pauses tier-2; `ok` for 30 s resumes paused trees oldest-first. A
paused build finishes later; a killed one is redone; a swapped pod loses everything and its secrets.
`guard_mode: observe` logs `would-…`; `enforce` acts. The first live reading after it started was a
`cargo test` at 215–257 MB/s with host PSI full at 23 % — hard — two `would-kill` rows in observe,
then the operator's standing instruction ("implement the cheap fixes") flipped it to enforce.

## What this deliberately does not do yet

- It does not know the model and lab of a session unless the session says so. Hooks receive a
  session id, not a model id; `berth moor --model … --lab …` completes the record. Making the
  runtime assert it is the graduation step (the recipient identity in `delegates-compute` must be
  attested, not self-typed).
- It does not gate. Nothing refuses a `just mesh start` or a `cargo test` that skipped `berth
  claim`; io-guard measures what actually happens and attributes it to whoever holds the lease,
  which is how a skipped claim becomes visible (`holder: null` on a shed row).
- It does not replace the messages. Sessions still talk; the berth is where the *decisions*
  those messages produce are recorded so the next session, and the guard, can read them.

## Graduation path

1. **Session identity asserted by the runtime.** The moor call moves into each runtime's session
   start with its model id and vendor as facts, not flags; the mooring becomes a signed record.
2. **The quota verb moves into `ark`** (tevah §5.2 step 2): io-guard's sensor and ladder become
   the supervisor's, with `io.max` on a delegated cgroup controller doing the enforcing where it is
   writable and this user-space shedder retired by its own `RETIRE_WHEN`.
3. **Moorings become `delegates-compute` commitments** and sheds become breach events
   `bounded_by` them; the period composite is the only new head. The berth ledger is then a
   projection of the sidecar `flows.jsonl` the same way `epr flow` already reads it.
4. **The workspace joins the fleet as matthew's device peer** (the native-sync recipe already
   proven), at which point "carrying capacity" is reported the way every peer reports it, and the
   cluster datastore stops sharing a disk with a developer's cargo pool — the topology fix the
   operator owns, which no guard substitutes for.
