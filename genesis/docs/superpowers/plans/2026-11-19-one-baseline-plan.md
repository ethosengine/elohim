---
title: One baseline — the sprint's stations
id: one-baseline-plan
status: active
class: process-meta
date: 2026-11-19
steward: human:matthew
serves: [workspace-device-arrival, push-gate-pays-once]
cites:
  - "one-baseline-journal | the sprint journal whose definition of done and lanes this plan restates as stations | sha256:8630f271ce165ec4 | path: genesis/docs/superpowers/sprints/2026-11-19-one-baseline/journal.md"
  - "native-throughput-journal | the closed sprint whose failed first line and open rows this sprint inherits | sha256:5aad581969ae3e02 | path: genesis/docs/superpowers/sprints/2026-11-17-native-throughput/journal.md"
  - genesis/data/timeline/backlog/steward-device-arrival-open-items.md
  - "native-dev-tooling-over-peers-plan | the epic's plan, whose section 8 carries the stations for the spec's tracks and joins | sha256:a446939e508db9a5 | path: genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md"
---

# One baseline — the sprint's stations

The one-baseline sprint keeps its definition of done, lanes and rulings in its journal, and the
rows it inherited in the `steward-device-arrival-open-items` backlog. `epr flow project` reads
neither: it mints from `- [ ]` stations in documents under `plans/` and `specs/`. This file
restates each definition-of-done line, each lane and each inherited open row as one station, and
adds nothing to them. The journal stays the record of what happened. A station here is only the
claimable form of a line there.

Written on 2026-11-19 from the journal at dev b9ec59330 by shem session d9c6be9b, at the sprint
orchestrator's request. Nothing in it was re-measured; each State line repeats the journal or the
backlog.

Gap ids are positional. New stations are appended at the end; stations are ticked, not reordered
or removed. A tick is the sprint's claim that a line is met, and still awaits verification.

## Definition of done

- [ ] **DoD-1 — one device's release is the other's: two consecutive `dev` landings from ethosengine are followed on shem with `EPR_FOLLOW_SIBLING=1` and no local build, and the daemon restarts on the drawn bytes and serves** (serves workspace-device-arrival)

State: the experiment passed before anything landed. `epr` built on ethosengine with
`-march=x86-64` was fetched on shem in 4.4 s, ring's 13 C files all carry that flag, and it ran
as shem's peer daemon for 75 s. Two consecutive landings have not been drawn yet.

Verify: the journal's receipt, with the draw time and the compiler flags written in the drawn
binary.

- [ ] **DoD-2 — a release nobody measured is refused before it is fetched: a sibling release without the `env:isa-c` marker is refused as unmeasured with zero bytes fetched, and the device builds its own** (serves workspace-device-arrival)

State: waits on lanes L2a and L2b.

- [ ] **DoD-3 — a landing pays less: the orchestrator gate's wall on a docs-only landing is measured before and after on both devices, with the two slowest test files no longer paid in full on every landing** (serves push-gate-pays-once)

State: waits on lane T1.

- [ ] **DoD-4 — the orchestrator suite keys on what it reads, soundly: the named half runs in a view holding only its declared reads, reuses on a docs-only landing, and `just gate orchestrator` still runs every test file** (serves push-gate-pays-once)

State: waits on the rework of `self/nt-e11`.

- [ ] **DoD-5 — the sessions do not collide: every landing that touches `genesis/orchestrator`, `elohim/eprfs` or `elohim/ark` is announced one line ahead, and no landing dispatches Edge by accident** (serves devices-share-one-berth)

State: one near miss is recorded. The brit session's branch that moved a reader into
`elohim-device-runtime` selected `elohim-edge`; it ran the dispatch walker, saw it, and is
rebuilding the change with nothing under device-runtime.

- [ ] **DoD-6 — the record is current: the journal, backlog rows 28 to 30, and one delta line in each habit atom the sprint moves** (serves push-gate-pays-once)

## Lanes

- [x] **E12-b — the gate's environment-key pattern is widened, and `pool-lib.sh` is keyed for every cargo gate** (serves push-gate-pays-once)

State: landed at dccd7a963 ("row 28, the slot library every cargo gate sources is in every
gate's key"). Owner: ethosengine a222db7c.

- [ ] **L1 — the per-triple record, the gate's exported flag, the seconds check, a producer that withholds an unmeasured offer, the pin test, and cc at 1.11.1 or later in the eprfs lock** (serves workspace-device-arrival)

State: owner ethosengine a222db7c. Ruled with the experiment: L1 records the C flags as inputs
only and does not declare a target at put.

- [ ] **L2a — the producer side in `epr-cli`: the C variables are recorded, put checks them, and the `env:isa-c` marker is written only after the checks pass** (serves workspace-device-arrival)

State: owner ethosengine a222db7c. `describe` swallowing the argument after
`--target-cflags=...` is a bug for this lane.

- [ ] **L2b — the receiver side in `epr-cli`: the marker rule before fetch, the bundled-C exercise before ingest and before current, the explicit-CID routes, and the honest-draw story** (serves workspace-device-arrival)

State: owner shem (builder). Contract with L2a: the slot is `env:isa-c`, its value is the
declared isa string (`x86-64` for `x86_64-unknown-linux-gnu`), and a triple with no record
writes none.

- [ ] **T1 — the orchestrator suite's slowest test files are no longer paid in full on every landing, with their proof preserved** (serves push-gate-pays-once)

State: owner shem (builder).

- [ ] **E11-rework — the named half of the orchestrator suite runs in a view of its declared reads, and the read fence leaves the trust path** (serves push-gate-pays-once)

State: owner ethosengine a222db7c, after L2a.

- [ ] **Brit-ref-spellings — `rev-parse`, `ls-remote`, `for-each-ref` and a global `-C` are discoverable spellings over verbs brit already has** (serves workspace-device-arrival)

State: owner shem (builder), handed back to brit sprint 3 as patches. The capability exists in
brit 0.2.6 as `revision resolve`, `remote refs` and `-r`; the gap is the spelling.

## Rows left open by the native-throughput sprint

The journal lists these as not in this sprint. They are stations so that they stay claimable.

- [ ] **Row-15 — a device's first `epr` is a forge release asset, checked by a verifier against a pin the steward authenticated before either file runs (ruling R5)** (serves workspace-device-arrival)

State: held. Two review rounds, the second fixed at 22e0c2931; a third was not run and it has
not landed.

- [ ] **Row-18 — a delegated worktree gets a pool family of its own at creation, or the gate refuses a slot another live gate owns** (serves push-gate-pays-once)

State: a family record survives a branch rename (5e47f38f0). Every `claude/<name>` worktree
still resolves to the one family `claude`; two gates removed each other's build tree on
2026-11-18.

- [ ] **Row-20 — every gate writes a verdict with a step fingerprint and a `gate_reuse` entry, or a written reason it cannot** (serves push-gate-pays-once)

State: enrolment is held by ruling on `claude/nt-gate-verdicts` (fbbbe6234): widening the key to
hold the ambient environment does not converge, so the declared envelope is the input.

- [ ] **Row-21 — the hand-over record lists a verdict by closure, with the commit it was minted at** (serves push-gate-pays-once)

State: held with row 20. `handover-record.mjs` reads a build note at the pushed head only.

- [ ] **Row-26.3 — the workspace images are pinned by digest and their build records a package list** (serves workspace-device-arrival)

State: the images live in the steward's `che-devworkspaces` repository; no change is prepared.

- [ ] **Row-26.4 — a build links against a declared target floor, so a workspace-built binary can run on the fleet** (serves workspace-device-arrival)

State: the floor is glibc 2.39 against the fleet's 2.36. `sysroot pack` and `put` exist and
nothing consumes them.

- [ ] **Row-28 — a gate verdict is keyed on the reads and variables its own envelope observed, not on a static scan** (serves push-gate-pays-once)

State: E9 declared the real reads the audit missed (1bb6d1c90, widening only), and E12-b keyed
the slot library. The observed-read design is the steward's open decision.

- [ ] **Row-30 — a `berth say` note that did not cross is queued and retried, not dropped** (serves devices-share-one-berth)

State: the write limit is a fixed 5 s (`.claude/hooks/_observation.py`). On a loaded shem six
sends in a row did not reach the channel store. The sender is told; nothing retries.
