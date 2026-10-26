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
never read as `passed`. An interrupted Task gives no new verdict. Its `Unanswered` answer makes the
run close `interrupted`, and the gate records no new brit evidence.

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
- **Arrivals.** The steward's other devices (L1) would add watching a push from elsewhere.
  Excerpts stay local; which places and paths may cross still needs a disclosure ruling.
  A CI runner (L2) would add an independent second witness within its declared observation reach.
  That reach is not supplied by a compute grant. Subscribers to a reach-scoped channel (L3)
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
  values. Excerpts therefore never leave the device. Removing them does not authorize disclosure
  of a place, path or test name. That ruling is still open (workspace-discipline backlog row 52).
  A failure is not a sanction: no event feeds standing or contribution.
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
  derivation?: { method, input, hit },               // Task gate-verdict; method/input are CIDs
  status?: Answered | Unanswered,                   // Task gate-verdict
  unanswered?: { why, comeBack, meanwhile, escalation? }, // gate-verdict and interrupted run-close
  outcome?: passed | gates-passed | failed | interrupted } // run-close
```

`failure` is raised at the line that names it (cargo's `test x ... FAILED`), so it arrives before
the panic block; the block follows as `detail` with the same fingerprint. Fingerprints use the
ci-harvest normalization, so the same failure on two runs has one fingerprint. `excerpt` is
device-only; every crossing rung removes it.

Task fields wrap the existing verdict. `Answered` includes a completed red gate and a reused green
verdict. `Unanswered` carries no new verdict. The full answer and derivation are in the run log;
`follow` prints a shorter line. `gates-passed` closes the gate loop while later pre-push checks remain.

## 6. Graduation ladder

One event shape at every rung; each rung adds readers and never becomes a condition for the one
below it.

| Rung | Reader | Transport |
|---|---|---|
| L0 (this slice) | the session holding the push lease | the run log on the device; `gate-live.mjs follow` |
| L1 | the steward's other devices | proposed `gate-live-events@1` FlowEvents through `epr flow note` and `berth.channel.crosses`; waits on disclosure and retention bounds (row 52) |
| L2 | the same channel, a second witness | a CI runner is a sponsored service agent. Publication waits on observation authority and a reach ceiling (row 51). Native emission also waits on provider acceptance, completion and receipt resolution (row 49). The SDK schema at `elohim/sdk/schemas/v1/gate-live-event.schema.json` owns the event vocabulary |
| L3 | people subscribed to a candidate or branch at a declared reach | the observation layer: signed per-observer log, cursor gossip, pull by equal peers. Delivery waits on an authorized signer and a second-person subscriber (row 50); "one bounded project/branch cursor, do not poll" |

The Scion/GitLab direction, latest code running on a server, lands at L2 and L3 inverted: the
always-on host is a hub guest under a `delegates-compute` commitment that adds capacity, and the
device's own run stays the floor.

## 7. Built: local placement Task and derivation

The orchestrator gate now runs through `ark task run`. `GATE_TASK` defaults on for that project
only. `gate-task.mjs` prepares the Task; `gate-runner.mjs` runs it through the existing tap.
Commits `adc2a2059` and `d453d99e7` built this path.

The method family is `brit-gate:`. It derives `GrantScope::OwnEnvelope`. In
`elohim/ark/cli/src/task.rs`, `decide_private` checks the device's choice to carry the Task.
Placement then admits it against the device's own offer. It runs in that device's own envelope,
offline, with no grant. Completion names the local device and time.

The input root is a CID, a content identifier. `ark task spec` builds it from `gate-evidence.mjs`'s
canonical closure material: sorted JSON with no whitespace. Rust builds an eprfs tree with one
leaf per field. The tree's CID is the input root. The sha256 of the JSON stays brit's dedup key.
A derivation pairs the method CID with that input root and points to the saved verdict.

Before starting a gate child, ark asks the brit-backed `DerivationIndex` in `brit_index.rs`.
An admissible hit runs no gate child. It still requires `GATE_EVIDENCE=reuse`, an enabled project,
clean inputs, and the newest attestation to be green and signed by this workspace. Ark also
requires an unchanged repository tree since that attestation. It does not ask rakia to judge
changes outside the closure. On a miss, ark runs `run-local-gate.sh` under its native driver.

The gate prints `reused: derivation hit` on a hit. Its `gate-verdict` event carries
`derivation: { method, input, hit }` and `status: Answered`. A completed execution also has
`Answered`, including a red result. The brit attestation stays the verdict record. The derivation
wraps it. `DerivationIndex::record` writes no second store; the gate runner remains the writer.

SIGINT or SIGTERM during execution produces `Unanswered`. The terminal names the interrupt.
The run log carries three parts: `why` is `not-carried`; `comeBack` names `rerun-requested`;
`meanwhile` says what is available now. `use-held` points to an existing attestation with a brit
lookup command. It does not certify that attestation as reusable. With none held, `nothing`
explains that this closure has no verdict and an interrupted run cannot supply one.
`gate-live.mjs` carries this answer into `run-close` and sets its outcome to `interrupted`.
The runner records no new verdict evidence for that interrupted gate.

With no closure material, no usable ark, or ark exit 75, the runner prints one visible
`gate-task: … — ran direct` line and takes the old path. The default ark lookup rejects a missing
or stale pool binary; an explicit executable `ARK_BIN` overrides that lookup. Missing material
can follow a dirty native closure. A dirty tree alone does not always remove material: the
orchestrator path can retain it, run through ark, and refuse reuse and evidence recording.

The live channel still uses the same parts:

- `gate-failure-lines.mjs` recognizes each failure line and counts the first red.
- `gate-tap.mjs` forwards output unchanged and appends failures as they print.
- `gate-runner.mjs` emits gate events and records `failure_class` and `first_red_s` on the cycle row.
  `GATE_TAP=off` runs untapped.
- `gate-live.mjs` stores the run log and offers `open`, `event`, `close`, `follow` and `show`.
  `follow` defaults to the caller's session and waits past its previous closed run.
- `.husky/pre-push.bash` opens the log before the gate loop and closes it on completion or interruption.

Ark also runs the device's declared residents through `epr device supervise`
(`06dd6c51c`, `a3dfff3c7`). A gate Task needs no resident supervisor. Pre-push leaves the resident
language server's rust-analyzer running when it carries `ELOHIM_SESSION_ID=lsp-device`
(`5bb980361`). Its flycheck builds yield to a running gate (cargo-disk-guard Rule 4); stray servers are still paused.

Commit `d453d99e7` records live proof in a scratch clone of this branch: a miss took 469 s;
the unchanged rerun hit in 23 s with no child. SIGTERM produced the three-part `Unanswered`
answer, closed the run `interrupted`, and recorded no evidence.

## 8. Frontier

Commit `8d1fc89ed` puts the deferred work in rows 49–53 of
`genesis/data/timeline/backlog/arch-workspace-discipline-backlog.md`.

- Native emission waits on provider acceptance, completion and receipt resolution (row 49).
  A local Task does not prove that provider path.
- Observations reaching another person wait on an authorized signer and subscriber (row 50).
- CI identity is settled: the runner is a sponsored service agent. Its observation authority and
  publication reach ceiling remain open (row 51).
- Device crossing waits on a disclosure ruling for paths and loci, and bounded retention (row 52).
- Tests mode and red-first ordering wait on method identity covering every execution-changing
  parameter. `all` mode is dropped because it would add a second recipe executor (row 53).
