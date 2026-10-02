---
id: performance-deep-dive-shem-handoff-20261002
status: handoff
date: 2026-10-02
author: "claude-opus-5-5 (ethosengine workspace), at the operator's direction"
habits: [zome-call-cost-bounded, runtime-performance, idle-is-free, dataplane-convergence]
---

# Performance deep dive on shem: reconcile the labelled concerns, then profile

## Operator direction

The operator, 2026-10-02: campaign 1.4 has not closed because convergence takes longer than the
test tolerates. Understanding performance and bottlenecks is a good focused first task for the
shem workspace, because it needs little realtime communication with the ethosengine workspace.
This workspace was asked only to label and index the concerns so that the deep dive can start
well. It investigated nothing.

You are the shem workspace. Every message between the two workspaces is carried by the operator
by hand, so work from this page and the repository, and send back files, not questions.

## What exists for you

| What | Where |
|---|---|
| The map: 74 backlog entries grouped by layer and mechanism, and the eight performance habits | `genesis/data/timeline/backlog/performance-concern-index.md` |
| The tag vocabulary | `genesis/data/timeline/CONVENTIONS.md` §Performance |
| The tags themselves (`performance` + `perf-*`) | frontmatter of 71 backlog entries |
| The profiling tooling and its acceptance rules | the `runtime-performance` skill; `genesis/a2o/scripts/runtime-performance.md` (start with the opening section and §Agent reconciliation loop); `.epr-meta/runtime-performance.habit.md` |
| The fork's ranked resource-bound priorities | `genesis/data/timeline/backlog/arch-dataplane-borrows-backlog.md` rows 17–18 |

The labels are a first reading. 235 of 675 backlog files were swept by keyword and classified from
a partial read; the other 440 were not opened. A tag names the mechanism an entry reports. It does
not mean the cost is still present.

## Why now: what stopped campaign 1.4

Campaign 1.4's household leg needs two successive updates to one root to be authored, approved,
witnessed, elected by each receiving peer and served by the local doorways inside one shared
75-second deadline. It has zero qualifying passes. The figures below are copied from the stop
assessments on the ethosengine workspace; all actors were test fixtures.

| Observation | Figure |
|---|---|
| Native acceptance verification budget | 4,000 ms; refused twice with `acceptance verification time budget exceeded — PENDING` |
| Local read of the grant action, lock waiting excluded | 9,313 ms (action sequence 45,207) |
| Local read of the approval action, lock waiting excluded | 10,873 ms (action sequence 45,235; 28 actions after the grant) |
| Root approval call, client wall time, two attempts | 19,697 ms and 21,480 ms (both ended `invocation mandate expired`) |
| Controller witness verification, read-only, two attempts | 11,615 ms and 8,979 ms |
| Witness exercise in the 30-second-budget run | terminated at the 75-second deadline; no native cancellation is inferred |
| Doorway B after an earlier recovered head was elected by all three receivers | still at baseline after 150 seconds |

The stop assessment's own words for the smallest unblock: reduce the cost of native record and
acceptance verification enough to fit the existing four-second budget, preserving every signature,
issuance, revocation-history and Human-witness check, and then prove canonical projection delivery
on doorway B. It says a focused trace is needed to find the implementation change, and that the
readings do not locate the internal hotspot. The deadlines are not to be raised.

The closing definition defers conductor per-call cost to its own campaign. This deep dive is that
work's diagnosis half.

## The ranking lens: trust should price compute

The operator asked for a second set of labels so that the obvious wins surface. The protocol's
thesis is that verification is paid once, close to the human judgment, and that cost falls as trust
rises. So every performance entry now also says who stands at each end of the costed path
(`trustful-self`, `trustful-declared`, `trustful-earned`, `trustless`) and what kind of cost is paid
(`friction-verify`, `friction-wait`, `friction-mechanical`, `friction-blind`). Definitions:
`genesis/data/timeline/CONVENTIONS.md` §Trust context and friction. The cross-tab and the two
reading-order tables are in the index under "The trust gradient".

| Of 74 performance entries | verify | wait | mechanical | blind |
|---|---|---|---|---|
| trustful-self | 3 | 7 | 17 | — |
| trustful-declared | — | 7 | 9 | 3 |
| trustless | — | 1 | — | — |
| mixed, or no relationship on the path | 1 | 3 | 15 | 8 |

How to use it:

- **Trustful × verify or wait (17 entries) is the candidate set for compression**: a carried proof,
  or one trust act per relationship in place of one per item. Ceremony is compressed, never skipped.
- **Mechanical friction is owed whatever the relationship** and is ranked by measured cost alone.
  Do not defer a livelock, a leak or a contention storm to trust.
- **The campaign 1.4 stop is the sharpest trustful-self case on record**: one human's second device
  approving that human's own root, paying 9–21 s per call. It has no backlog entry of its own; its
  home is `conductor-residual-cpu-full-chain-read-and-perpetual-republish.md`.
- **Every path is trust-blind today.** The dataplane trust handshake is a stub that classes every
  sync edge `public`, so the labels name the relationship that holds, not one the code reads.
- The labels are one reader's first pass. Re-check them on anything you rank near the top.

A third label marks where breadth of holding drives the cost. Nine entries are `custody-everyone`:
the cost is paid because every participant holds or validates all of it (full arc, flood gossip, a
copy per hosted agent). For those, report what the cost would be at a smaller holder set, and say
plainly where the entry shows breadth was not the driver. The conductor line in use has only a zero
or full arc, so a sharding finding is a design input, not something to actuate. The operator's
principle — compose the holder set for independent failure domains, not for headcount — is recorded
at `commons-holonic-stewardship-backlog.md` row 30. Index section: "Custody shape: who carries it".

A fourth label names the plane whose work consumes the resource (`plane-bytes`, `plane-head`,
`plane-authority`, `plane-notary`, `plane-custody`, `plane-projection`, `plane-reference`,
`plane-attention`, `plane-value`), and `fused-planes` marks 28 entries where that price is charged
on another plane's path. The commonest pairings are projection or notary work charged on the head
path (5 each), custody work charged on bytes (3) and notary work charged on projection (3). The question for
each: can the cheap plane stop waiting on the expensive one? This pass read only titles and opening
lines, so the pairing is a prompt, not a finding. The residual-CPU entry, the home of the campaign
1.4 stop, was read as notary work on its own path and is not marked fused; whether the 9–21 s
calls are authority or notary work charged on the head path is exactly what the first profile
should settle. Index section: "Planes: which part of the object is doing the work".

Three shorter labels complete the set: what the cost multiplies by (`unit-once|call|item|peer|agent|history`),
when it is paid (`phase-steady|transition|growth`) and who is waiting
(`lane-interactive|background|borrowed|operator`). Seven entries are costs that grow with history
and seven are background work standing in a foreground lane; the index lists both under "Cost unit,
phase and lane". Of the 65 entries with a phase, 23 are transition costs (restart, cold start,
roll, catch-up), which want staggering or warm-up more than optimisation.

The whole family is the **gradient view**, and the `gradient-reading` skill carries the method. You
are the performance developer: you measure. Load the skill yourself once you have a ranked list of
measured costs, and read each path by relationship, friction, plane and unit in the session that
holds the measurements. Its design-time section applies when a finding implies an object should
declare its reach, custody or freshness differently; the `p2p-design-gate` skill now points to it.
For a bulk labelling pass, the skill says how to brief a general-purpose reader; there is no
dedicated agent. The skill has not been exercised on a real task yet.

## The entries nearest that stop

Read these first. They are the labelled entries whose subject is per-call cost at a long source
chain, or head delivery to a doorway. Whether any of them explains the figures above is unknown.

- `conductor-residual-cpu-full-chain-read-and-perpetual-republish.md` — the home the closing
  definition names for per-call cost; the campaign added an evidence line to it.
- `conductor-cap-grant-scan-per-zome-call.md` — the capability-grant read per zome call.
- `elohim/holochain/.epr-meta/zome-call-cost-bounded.habit.md` — the invariant and its three
  checks. Two are passing fork tests; the third is declared `NOT YET WIRED`: an `entry_type`-only
  chain read still reads every committed header, and `content_store` carries that shape.
- `conductor-admission-saturated-for-hours-after-restart.md`,
  `resolve-canonical-election-get-links-deadline.md` — what a slow call does to admission and to
  the election read.
- `head-authority-carried-with-content-sync-unit.md`, `alpha-a-projector-chronic-catchup-flap.md`,
  `2026-08-10-adam-pull-loop-wedged-at-boot.md` — the doorway-B side: a head elected natively but
  not yet served.
- `arch-dataplane-borrows-backlog.md` row 18, priorities 1 and 4 — bounded background work, and
  diagnostics that tell progressing from waiting from stalled.

## The work, in order

1. **Reconcile.** Walk the index one layer at a time, conductor first. For each entry, read it
   whole and settle three things against the code and the conductor pin in your checkout: is the
   cost still present, is it the same mechanism as a sibling entry, and what is the one measurement
   that would confirm or refute it. Correct a wrong tag at the source. Tag any performance entry the
   sweep missed. Record the result in the entry's own body as a dated line, not in a new file.
2. **Rank.** Order what survives by measured cost on a path someone waits on, not by how alarming
   the entry reads. Say which entries have no usable measurement at all.
3. **Profile.** Follow the `runtime-performance` skill. Name the target, workload, duration and
   evidence directory before any capture. The first question is the one campaign 1.4 could not
   answer: where the time goes in a local record read and in acceptance verification at a source
   chain of about 45,000 actions.
4. **Report.** See "What to send back".

## What you do not have

- **The evidence directories.** `genesis/a2o/reports/` and `genesis/local-dev/` are gitignored, so
  none of the receipts, captures or private run state on ethosengine reach you through git. This
  page carries the headline figures for that reason. If you need a receipt itself, name the exact
  path and ask the operator to carry it. The campaign's are under
  `genesis/a2o/reports/recovery/campaign-1.4-restart-20261001/` (`acceptance-cost-stop.json`,
  `acceptance-budget-30s-stop.json`, `approval-witness-latency-20261002.json`); the fleet CPU work
  is `genesis/a2o/reports/recovery/fleet-cpu-publish-livelock-2026-09-28.md`.
- **The campaign 1.4 sprint pages**, unless they have been committed by the time you pull. On
  2026-10-02 `2026-10-01-campaign-1.4-closing-definition.md` and
  `2026-10-01-campaign-1.4-household-restart.md` were untracked in the ethosengine tree.
- **The specimen.** The slow reads were measured on the ethosengine household, whose author chain
  is past 45,000 actions. A fresh household on shem starts near zero. How you obtain a comparable
  chain — grow one with a fixture, or have the operator carry a stopped store — is an open decision
  for the operator. Do not present a fresh household's timings as a reproduction.
- **A conductor on the fleet or the household.** The shem workspace is deliberately not a device in
  campaign 1.4 and holds no witness-authorized grant. Work on a household you own on shem.

## Things to check on arrival, not to assume

- The conductor fork commit. The superproject records `c8c17202c` for `elohim/holochain-conductor`;
  the ethosengine checkout sat at `7e553f9c3` on 2026-10-02. Read `git submodule status` and name
  the exact commit in every finding.
- Tool availability on the shem image (x86-64-v2 flavor): `perf`, the fork toolchain under
  `MESH_TOOLS_DIR`, `pnpm` or `node --import tsx`. `cargo nextest` is not installed on ethosengine.
- The node itself. Memory records shem as a Dell T7610, dual Xeon E5 v2, 135 GB, with new SAS SSDs
  arriving in 2026-09; re-probe before relying on that.
- Whether the observability and Jenkins MCP servers are reachable from shem. If they are, alpha
  fleet history is readable without touching the fleet.

## Boundaries

- Diagnosis only. No fix, fork patch, pin move, deploy or push is authorized by this page.
- Do not raise the 4-second verification budget or the 75-second publication deadline, and do not
  weaken a signature, issuance, revocation or witness check to make a number fit.
- A capture must not restart, recast or reset anything it did not launch. Heap traversal with a
  hard deadline belongs only in a disposable canary.
- Scale claims need at least two controlled points per hot path. One snapshot is not a growth law.
- Keep uncertainty the entries already preserve. Several say plainly that a mechanism is a working
  theory or that a local reproduction did not establish the fleet cause.
- Persistent work goes under `genesis/local-dev/<campaign>/` and evidence under
  `genesis/a2o/reports/recovery/`, never `/tmp` alone.

## What to send back

One branch from the shem workspace, carrying:

- A report at `genesis/docs/superpowers/sprints/` with the ranked findings: for each, the evidence
  path, the fork and storage commits, the confidence, what remains unproved and the smallest next
  measurement. Copy the headline figures into the report, because your evidence directories are
  gitignored too.
- The dated reconciliation lines in the backlog entries you settled, and any tag corrections.
- A regenerated `performance-concern-index.md` if the tagged set changed.
- One DELTA line in the habit atom the work actually moved (`zome-call-cost-bounded` or
  `runtime-performance`), then `.claude/scripts/habits-project.py`. A status flips only on its own
  check.

Stop and write down the question for the operator if the work needs the ethosengine specimen, a
fleet action, or a design decision.
