---
title: Gate failures arrive live — a local-first failure channel for the push gate, and how it grows into subscribed channels
id: gate-live-failure-channel-design
status: Draft
class: architecture
serves: [gate-failures-arrive-live, push-delivers-within-budget]
date: 2026-10-26
context-tier: disclosed
steward: human:matthew
author: "Claude (orchestrator, Opus 5.5), at the operator's direction"
tags: [gate, pre-push, live, observation, rakia, brit, berth, steward-channel, social-coding]
graduation-trigger: >-
  A real push with a red gate whose first failure reached the pushing session while later gates still
  ran: `first_red_s` on that gate's `gate-cycle-seconds@1` row well under its `value`, every failure the
  run printed present in its run log, and the run closed. A unit test alone cannot graduate it.
cites:
  - "live-session-plane-design | rules passing state never decides; live progress, if ever carried, belongs on this plane | sha256:d1adbf86e46fd487 | path: genesis/docs/superpowers/specs/2026-10-14-live-session-plane-design.md"
  - "workspaces-cooperate-as-devices-design | the steward channel L1 crosses on; berth FlowEvents are Private (B); secrets and transcripts never cross | sha256:28296e002122f501 | path: genesis/docs/superpowers/specs/2026-10-23-workspaces-cooperate-as-devices-design.md"
  - "plane-separation-design | nothing ships without a reader; the pushing session is this log's reader | sha256:657eb48f9c7a4db0 | path: genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md"
  - "working-version-sdk-standard-design | the shown-here / passing / published split this spec's kinds of state follow | sha256:72f6e63fde2fb953 | path: genesis/docs/superpowers/specs/2026-10-04-working-version-sdk-standard-design.md"
  - "what-every-app-inherits-placement-contract-design | Scion/GitLab transfers and inversions; CI runners are sponsored service agents | sha256:fe10f9dc966c2e9a | path: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md"
  - "observation-event-layer-design | the path L3 graduates through: signed per-observer logs, cursor gossip, equal-peer readers | sha256:002fc7ec733dd10a | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md"
  - genesis/data/timeline/backlog/gate-merge-gradient-reading-2026-10-25.md
  - genesis/data/timeline/backlog/ci-as-steward-channel-participant.md
  - genesis/orchestrator/.epr-meta/push-gate-pays-once.habit.md
  - genesis/orchestrator/.epr-meta/gate-failures-arrive-live.habit.md
  - genesis/a2o/features/devflow/gate-failures-arrive-live.feature
---

# Gate failures arrive live

## 1. The problem

The pre-push hook runs each selected project's gate in turn and refuses the push at the end if any
gate failed (`.husky/pre-push.bash`, the gate loop). Until this slice, `gate-runner.mjs` ran each gate
with inherited output, so no process ever read what the gate printed, and the agent that started
`git push` learned nothing until the hook exited. On 2026-10-25 a storage gate on ethosengine went
red after about twelve minutes; the pushing session heard of it at about thirty-two.

Inside one project a recipe still stops at its first failing step. Across projects the loop goes on,
so failures after the first exist and are worth reading: they shape the diagnosis that the first
one starts.

## 2. The rule

**The verdict is the record; live events are observations nobody decides from.** A gate's exit status
decides the push, as before. A recognized failure line never refuses a push, stops a gate or changes
a status, and a line the recognizer missed never permits one. The published record of a gate stays
what it was: the brit `brit.gate-verdict` attestation against the gate's input closure and the
`gate-cycle-seconds@1` row (`push-gate-pays-once`).

A run with no `run-close` reads `interrupted` once it is stale, and `running` before that. Silence is
never read as `passed`.

## 3. Local-first reading

- **Alone, key on the device.** Everything in this slice runs with no network, no daemon and no
  peer. The gate writes to a run log on the device; the session that started the push follows it.
- **Key held by a doorway; no key.** Does not apply. Gates run in a workspace on the steward's own
  device; a push needs a moored session. When berth falls back to a synthetic `pre-push-…` holder,
  the run still writes, and the hook prints the run's path.
- **Working version.** None. A gate run is not editable content.
- **Kinds of state.** The terminal output is *shown here* and is forwarded unchanged. The run log is
  an *observation*: stored on the device, read by the pushing session, pruned when the next run
  opens, never on a timer. The *published* verdict is unchanged. Live progress ("still compiling")
  is not carried; if it is ever wanted it is passing state on the live-session plane.
- **Arrivals.** The steward's other devices (L1) add watching a push from elsewhere and learn
  project, class, fingerprint, place and verdict, never excerpt text. A CI runner (L2) adds an
  independent second witness inside its grant's reach. Subscribers to a reach-scoped channel (L3)
  add collaborators following a candidate; a host carrying it adds capacity and gains no say over
  heads. Losing any of them leaves the device's own run log working.
- **Shown to the person.** Each failure as it lands, each gate's verdict, and the run's outcome
  (`passed`, `failed`, `interrupted`).

## 4. Design gate

**GateRunEvent — Private (B).** It belongs to the steward's device; no peer validates it; it has no
DHT head. This is the class the workspaces spec gives berth FlowEvents. Volume is tens of events per
push; the device keeps the last twenty runs.

- **Address.** A run is an agent-scoped composite — device, session, pushed sha, start time — bound
  to an immutable candidate: the pushed sha, the target refs and each project's closure key. An
  event is `(run, seq)`. At L1 a crossing event gets the CID `epr flow note` mints.
- **Source of truth by rung.** L0: the run log under `<main checkout>/.eprfs/status/gate-live/`
  (gitignored; never imported or projected, like recall receipts). L1: the device's flow store,
  folded copies marked `via:` their device. L2 and L3: the emitter's signed per-observer log. At
  every rung the verdict record stays the brit attestation.
- **Stakes.** Same behaviour at every stage. Nothing floor-protected; nothing decision-bearing.
- **No zome, no route.** L3 graduates through the observation layer (signed per-observer log, cursor
  gossip, equal-peer pull), never a DHT entry per event.
- **SDO/RWA.** The worst holder of the device sees failure excerpts, which can contain environment
  values. Excerpts therefore never leave the device; a crossing event carries fingerprint, class,
  place and verdict only (workspaces spec §3.3). A failure is not a sanction: no event feeds
  standing or contribution.
- **Concerns.** C4: an unclosed run reads interrupted. C5: an event never gates. C6a: excerpts are
  capped at 20 lines and 2 KB, a run at 200 failures, with the overflow counted. C8: `first_red_s`.

## 5. Event shape v1

```
{ v: 1, run, seq, at,
  kind: run-open | gate-start | failure | detail | step-failed | gate-verdict | run-close,
  candidate?: { sha, target },  session?,           // run-open
  project?, step?,
  failure?: { class: test|compile|lint|format|env, fingerprint, locus: { file?, line?, test? },
              nth, excerpt },                       // failure, detail
  exit?,                                            // step-failed
  verdict?: { exit, seconds, failure_class, failures, overflow?, evidence?, reused?, deferred? },
  outcome?: passed | failed | interrupted }         // run-close
```

`failure` is raised at the line that names it (cargo's `test x ... FAILED`), so it arrives before
the panic block; the block follows as `detail` with the same fingerprint. Fingerprints use the
ci-harvest normalization, so the same failure on two runs has one fingerprint. `excerpt` is
device-only; every crossing rung removes it.

## 6. Graduation ladder

One event shape at every rung; each rung adds readers and never becomes a condition for the one
below it.

| Rung | Reader | Transport |
|---|---|---|
| L0 (this slice) | the session holding the push lease | the run log on the device; `gate-live.mjs follow` |
| L1 | the steward's other devices | a `gate-live-events@1` FlowEvent through `epr flow note`, added to `berth.channel.crosses`; `epr peer serve` gossips and folds it |
| L2 | the same channel, a second witness | a CI runner, as a sponsored service agent, emits the same events from its stages (`ci-as-steward-channel-participant`). The native emitter becomes `rakia-executor`'s task stream, supervised by ark beside its death witnesses; the schema moves to rakia `schemas/v1/`; `gate-tap.mjs` and `gate-runner.mjs` become adapters, as Husky already is |
| L3 | people subscribed to a candidate or branch at a declared reach | the observation layer: signed per-observer log, cursor gossip, pull by equal peers; channels named like release channels and widened by ceremony; "one bounded project/branch cursor, do not poll" |

The Scion/GitLab direction, latest code running on a server, lands at L2 and L3 inverted: the
always-on host is a hub guest under a `delegates-compute` commitment that adds capacity, and the
device's own run stays the floor.

## 7. What this slice builds

- `genesis/orchestrator/gate-failure-lines.mjs` — a line-at-a-time recognizer for cargo, rustc,
  clippy, rustfmt, vitest, tsc, eslint, node test and just output. It raises failures, fills the
  `classifyFailure` signals and counts the first red.
- `genesis/orchestrator/gate-tap.mjs` — wraps one gate command: bytes forwarded unchanged, exit status
  or signal passed through, failures appended to the run as they print, signals returned on fd 3.
- `genesis/orchestrator/gate-live.mjs` — the run log: `open`, `event`, `close`, `follow`, `show`.
  `follow` with no selector follows the caller's own session and waits past the previous push's
  closed run.
- `gate-runner.mjs` runs each gate (and its rust-style floor) through the tap, emits `gate-start` and
  `gate-verdict`, and writes `failure_class` and `first_red_s` on the cycle row, which delivers the
  output capture `push-gate-pays-once` named D2b. `GATE_TAP=off` runs untapped.
- `.husky/pre-push.bash` opens a run before the gate loop, closes it `passed` or `failed` after, and
  closes it `interrupted` from its exit trap.

## 8. Frontier

- L1–L3 as above. L2 waits on the runner-identity question in `ci-as-steward-channel-participant`;
  moving rakia's pin rolls the fleet, so the schema lands upstream first.
- Fail-at-end inside a project (cargo `--no-fail-fast`, recipes that continue past a red step) would
  surface more trailing failures. It changes gate cost and meaning, so it is an operator decision.
- Running likely-red gates first, ordered by the red rate in the gate-cycle rows, would bring the
  first event forward.
