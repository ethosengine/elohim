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

The same day the workspace pod was **swapped** (not restarted) and came back without its secrets.
The first diagnosis blamed a 130–270 MB/s write storm from the mesh seed and a cargo build. The
operator's cluster read the same evening corrected it: the secretless ReplicaSet was rendered at
07:21Z while k8s-dqlite was segfaulting in a crash loop (seven core dumps in eight minutes, all
three voter links timing out at once), and the 10:20Z swap was the Deployment flipping onto that
stale template; sysstat had the disk at 8–12 MB/s and under 1 ms await during the lease timeouts.
ethosengine is a dqlite *standby*; the quorum lives on three ThinkPads. And this guard's first cut
over-read its own byte rate by 3× (it summed io.stat over an md mirror and both members). What the
record does support is latency contention on two QLC NVMes past rated TBW (10:13Z: 81 ms await,
queue depth 52 at 38 MB/s; 2026-08-29: two hard resets under I/O pressure). So the carrying
capacity that matters is **I/O latency headroom, read as host PSI**, with the byte rate secondary —
and the coordination gap (who is doing what, on which capacity) was real regardless of which
organ failed.

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

Level = the higher of the host's `/proc/pressure/io full avg10` (primary; 10/20 %) and the
container's own write rate (cgroup `io.stat`, max over devices so an md mirror counts once, 30 s
window; 60/100/160 MB/s device-level, secondary). Nothing happens
without 20 s of dwell. `high` pauses (SIGSTOP) the tree writing the most, tier-1 only, one per
tick; `hard` kills tier-1 and pauses tier-2; `ok` for 30 s resumes paused trees oldest-first. A
paused build finishes later; a killed one is redone; a swapped pod loses everything and its secrets.
`guard_mode: observe` logs `would-…`; `enforce` acts. The first live reading after it started was a
`cargo test` the sensor reported at 215–257 MB/s (72–86 MB/s real, the 3× overcount) with host PSI
full at 23 % — hard on the PSI axis regardless — two `would-kill` rows in observe, then enforce.
Dwell now resets on daemon start: the first enforce run shed two rustc trees of a build launched a
second earlier because the level clock carried over a restart.

## VSM placement — a discipline, not a restraint (operator note, 2026-09-03 evening)

The operator's second ruling: *manage our own carrying capacities as a discipline of the VSM
primitives in our compute, but make sure we don't restrain ourselves by accident.* Placed in Beer's
terms (the ontology `ark-core` already inherits from `elohim-epr-rea`):

| VSM organ | here | the accidental-restraint failure it must avoid |
|---|---|---|
| **System 1** — operations | the sessions and their builds, seeds, mesh runs | none; S1 is what we protect |
| **System 2** — coordination / anti-oscillation | the berth's leases: one mesh, one cargo slot, one disk-heavy job, refusal by name | queueing silently (a refusal must say who, so S1 can negotiate) |
| **System 3** — resource bargain | the mooring: a human delegates bounded compute (write set, ttl) to a named agent; the claims are the bargain | bargaining on a number the substrate does not feel |
| **System 3\*** — sporadic audit | `berth ledger`, `io-guard status`, the shed rows with `holder` | none; audit reads, never acts |
| **Algedonic channel** — pain that bypasses the hierarchy | host `/proc/pressure/io` (`full avg10`) and ram-guard's committed memory | acting on a *proxy* (our byte rate) instead of the pain (host stall) |
| **System 4/5** — outside / identity | the operator: topology, quorum placement, lease durations, what counts as capacity | delegated to a daemon |

Concretely, after the corrected incident read: io-guard **acts on host PSI only** (the byte rate
is advisory in status and ledger), **acts only when this container is a real writer** (below
20 MB/s of our own writes a host stall is not ours to relieve), **pauses before it kills** (kill
only after 90 s of hard dwell that pausing did not relieve), and **resets its dwell clock on
start**. A budget that would have paused a build at 115 MB/s while the disk sat at 22 % utilisation
and under 1 ms await is restraint by accident; a budget that pauses the one compile writing into a
disk whose full-stall share is above 10 % for twenty seconds is the discipline.

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
   proven), at which point "carrying capacity" is reported the way every peer reports it. The
   topology fix the operator owns is not "move the mesh off the dqlite node" (ethosengine is a
   standby) but the datastore quorum sitting on the three least reliable machines, the dqlite
   SEGV itself (install systemd-coredump before the next one), and the controller's lease and
   cold-cache render — none of which a guard substitutes for.
