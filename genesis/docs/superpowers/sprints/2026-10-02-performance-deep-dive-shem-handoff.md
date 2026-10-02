---
id: performance-deep-dive-shem-handoff-20261002
status: handoff
date: 2026-10-02
author: "claude-opus-5-5 (ethosengine workspace), at the operator's direction"
habits: [zome-call-cost-bounded, runtime-performance, idle-is-free, dataplane-convergence]
---

# Performance deep dive on shem: reconcile the labelled concerns, then profile

## Read this first: where the code is, and what changed after this page was written

Added 2026-10-02 (evening) by claude-fable-5-1 on the ethosengine workspace, at the operator's
direction. The rest of the page is unchanged and still applies, except where this section says
otherwise.

**The code under investigation is not on `dev`.** It is on a handoff ref, published without a
deploy so that this work can start before the campaign's one push lands:

```bash
git fetch origin refs/handoff/campaign-1.4:refs/remotes/origin/handoff/campaign-1.4
git switch -c shem/performance-deep-dive origin/handoff/campaign-1.4
```

The ref holds the campaign branch (`codex/fct-hosting-native-reconstruction` at `12721cb88`, 28
commits past `origin/dev` `5ffe98552`) with this page and the pages it names merged in. Nothing on
it has been deployed: the fleet still runs `5ffe98552`. On this ref the conductor gitlink
(`elohim/holochain-conductor`) is `2b334df7973d`, the tip of the fork branch
`codex/fct-native-query-prefilter`; `origin/dev` records `517d4c835`. That replaces the
`c8c17202c` and `7e553f9c3` figures under "Things to check on arrival". The campaign sprint pages
listed under "What you do not have" are on this ref.

**One of today's failures was a crash, not latency. Do not read it as a timing result.** Storage
commit `03f107651` made every conductor-touching HTTP request future carry the conductor call by
value (46,072 bytes). In an unoptimized build the deepest request path then needed about 2.2 MB
of the 2 MiB `http-server` worker stack, and storage aborted with `thread 'http-server' has
overflowed its stack`. It happened twice: on the long-chain household (matthew, 14:32:34Z, during
serving run `household-campaign14-serving-20261002T135944Z-run-scoped`) and on a fresh three-peer
mesh (jessica, 15:48:25Z, during bulk content seeding). A call-graph search found no recursion.
`12721cb88` boxes the call (request future 12,960 bytes, worst path about 1.08 MB) and carries
three regressions. How much stack a release build used was not measured. Any run between those
two commits in which a storage process died is crash evidence only.

**The verification budget on this ref is 30,000 ms, not 4,000 ms.** The operator authorized the
raise on 2026-10-02 (`ba0703dee`) to let the functional path proceed. The 75-second publication
deadline did not move. For this diagnosis the four-second figure remains the reference the stop
assessment names; the boundary below about not raising budgets still binds your work.

**Later functional figures, same household, all actors test fixtures.** After the raise, three
receiving conductors each elected the earned head for two successive versions. Head reads took
11.7–36.0 s and election reads 12.3–29.6 s. Both local doorways served the second head and the
same body at 05:53:30Z, minutes after the receivers adopted it. That is eventual delivery; there
is still no pass inside 75 seconds.

**Bearing on the first design hypothesis.** Three storage commits on this ref are successive
patches for foreground authoring and the background writer sharing one source chain: a per-cell
write lock (`f66f1f4b0`), ownership of an offered write through caller cancellation
(`03f107651`), and the stack fix for the future that the second one enlarged (`12721cb88`). Read
them as the cost of that sharing so far. None of them measures or reduces the share of the
45,000 actions that machine writes account for.

**A comparison point for the specimen question.** The serving story
(`features/dataplane/epr-app-deliverability.feature`, five stations) passed on three-peer
households on 2026-09-28 and 2026-09-30 in 613, 925 and 1,227 seconds of station time. On the
four-peer long-chain household on 2026-10-02 it took 2,034 seconds and failed every station. A
fresh three-peer mesh built from this ref was ready in 151 seconds. Its serving-story result on
the fixed storage binary was not yet available when this section was written; it will be
recorded in `2026-10-01-campaign-1.4-household-restart.md`. Short chains and long chains are
therefore two points on one path, which is the two-point shape the boundaries ask for, but only
once both are measured with the same binaries.

**Conductor memory scales with hosted agents, not with content.** Measured read-only on that
fresh mesh at about 16:21Z, 35 minutes after start, after the prologue had registered 15 hosted
humans through the first doorway:

| Conductor | Apps / cells | Resident | Anonymous | Conductor directory on disk |
|---|---|---|---|---|
| matthew (hosts the cast) | 16 / 80 | 12,473,340 kB | 12,442,104 kB | 649 MB |
| jessica | 1 / 5 | 1,917,280 kB | 1,887,104 kB | 582 MB |
| james | 1 / 5 | 1,728,024 kB | 1,697,692 kB | 711 MB |

Every hosted registration installs a full five-cell app on the doorway's conductor. All three
conductors hold the same seeded content (about 3,500 items) and the same database files; 557 MB
of each directory is `wasm.db`, written before any hosted human existed. The difference is 99.8%
anonymous memory with no swap and about 10 MB file-backed, so it is heap, not storage. Taking
jessica as the baseline it is about 704 MB per hosted agent, or about 137 MiB per added cell;
matthew's own figure before the cast was not recorded. The repository's standing figure is about
786 MB per hosted human (22.8 GB over 145 cells on 2026-09-11, a quotient, not a breakdown).

Not established: what a cell's share is made of (compiled wasm or instances, per-cell SQLite
pools and page caches, kitsune2 per-agent state, workflow queues, allocator retention), which
allocator this fork binary uses, whether all 80 cells were running, and whether the memory is a
one-time cost of install or grows with time. This is one snapshot. The measurement that would
attribute it: `smaps_rollup` and the sizes of anonymous mappings before, during and after
provisioning one more hosted agent on a disposable mesh, paired with the conductor's own
`hc_*` metrics (`MESH_CONDUCTOR_LAUNCH=direct` exports them). It is a `custody-everyone` cost by
this page's labels (a copy per hosted agent) and belongs in your ranking.

**The bundle the household ran was assembled, not built.** It was an older preserved hApp with two
coordinators swapped in. Its `content_store` coordinator matches a build of this ref
(`66bc7660…`); its `mishpat` coordinator (`30066df3…`) does not (`1e9a09f2…` from this ref), and
the cause of that difference is not established. Build your household's hApp from this ref and
record the coordinator hashes with each finding.

**State you cannot see from git.** The ethosengine household is stopped with its identities
intact. The workspace conductor that is the real device in the campaign stopped abruptly at
15:08:26Z for a reason not yet established; nothing has been published to the fleet from it.

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

## Two design hypotheses to test

These came from reading the entries and receipts, not the code. They are hypotheses for the deep
dive to confirm or refute, and both are our design choices, not the fork's.

- **Machine bookkeeping shares a source chain with human acts.** The household author chain is past
  45,000 actions, and the campaign's first approval failed with a source-chain-head-moved error
  while background storage was writing to the same cell. If a person's ceremony grows with, and
  waits behind, machine churn, the question is whether high-frequency machine writes belong on that
  chain at all. Measure: what share of those 45,000 actions a person or a ceremony authored.
- **Authority is paid per connection instead of per relationship.** `zome-call-cost-bounded`
  records that storage authorises a new signing credential on every connect, which is how one agent
  reached about 15,000 grants. A storage peer and its own conductor are one steward. Measure: grants
  minted per connect and per day on a household peer at the current pin.

They compound: more grants and a longer chain make every call slower. Their home, with three
further observations, is `arch-dataplane-borrows-backlog.md` §"Plane-separation pass — design
observations to check"; record what you find there, not here.

## How the two skills work together

`runtime-performance` answers where the time goes: process costs, sampled functions, call
durations, SQL, with a coverage check that refuses to call thin evidence a pass. `gradient-reading`
answers whether that cost should exist on that path: who stands at each end, what kind of cost it
is, which plane is paying. Neither replaces the other. A loop, per path:

1. **Choose the path with the gradient.** Take the labelled entries where a trustful path
   re-verifies or waits, where a cost grows with history, or where planes are fused. Write down the
   one question a measurement should settle, before capturing anything.
2. **Measure with `runtime-performance`.** Capture and report for that path only. Its report says
   what was measured and what is missing.
3. **Read the result with the gradient.** State the two ends, the fact being re-derived or the
   plane being waited on, and the compression the measurement justifies. If the cost turns out to be
   mechanical, say so and rank it by its measured cost alone.
4. **Record** the finding in the entry it belongs to, with the commits and the evidence path.

One known gap bears on step 2. The `runtime-performance` habit is red because caller, workflow and
SQL attribution are incomplete: an earlier capture resolved leaf functions (SHA-512 at 45% of one
conductor's sampled CPU) without the callers that would say why. The gradient question narrows
which caller to look for; it does not supply the missing attribution.

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
