---
id: "backlog-consent-speaks-refresh-is-a-timer-not-a-demand"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Every node reads its own consent standing every 30 s whether or not anyone is asking: the device carrier's speaks refresher is a timer keeping a 120 s freshness bound warm"
slug: "consent-speaks-refresh-is-a-timer-not-a-demand"
written: "2026-10-12"
author: "claude-opus-5-5 (sprint 2026-10-12-idle-call-floor-sprint)"
status: "open"
priority: "low"
severity: low
nodes: [alpha, household]
relatedNodeIds:
  - "habit:idle-is-free"
tags: [device-consent, carrier, mishpat, performance, perf-cpu, idle-is-free, trustful-self, friction-verify, plane-notary, unit-peer, phase-steady, lane-interactive]
cites:
  - genesis/docs/superpowers/sprints/2026-10-12-idle-call-floor-sprint.md
  - elohim/elohim-storage/.epr-meta/idle-is-free.habit.md
---

**What.** `services/device_carrier.rs` keeps "whom this node speaks for" in memory with a 120 s maximum age
(`SPEAKS_MAX_AGE_MICROS`); a refresher spawned in `HttpServer::serve` re-reads it with
`mishpat::my_consent_standing` every 30 s (`SPEAKS_REFRESH_SECS`), plus `my_human` when the node is a controller.
Measured on the household at rest 2026-10-12: 2.0 to 2.2 calls a minute per peer, on the interactive lane
(`call_zome_mishpat` has no classed variant), all day, on every node. That is a third of the whole `idle-is-free`
call budget spent holding one value fresh for an ask that may never come.

**What was tried and withdrawn.** The 2026-10-12 sprint built a slower idle cadence (first 80 s, then 60 s, then
45 s with a 15 s retry, short cadence kept while the carrier is in use) and withdrew it after three independent
review rounds. Each version was shown a schedule in which a node's evidence expires where the 30 s cadence's does
not: a later first read leaves less-fresh evidence when an outage begins, whatever the retry does afterwards. The
saving was under one call a minute and was paid for in the freshness the read exists to hold. The withdrawn diff
is kept at `genesis/a2o/reports/recovery/idle-call-floor-20261012/consent-pacing-withdrawn.diff` with the review
receipts beside it.

**The cure that does not trade freshness.** Read on demand. `Carrier::on_ask` is synchronous (it runs in the
transport's event loop) and reads the cached value; that is why the value must be kept warm. If the ask path can
await one single-flighted standing read, rate-limited to one per `SPEAKS_REFRESH_SECS` however many asks arrive
(the existing "never per incoming ask" bound, kept as a rate), the timer can stop and a resting node makes no
call. The 120 s bound then limits how stale an answer may be, as now, without a poll. The refresher and the
affirmation pass should read on the background lane either way, which needs a classed mishpat call.

**Constraints.** The freshness bound is a consent decision and is not this entry's to lengthen. A node whose
joining record no longer stands must still stop listing within it. `ensure_listed_proof` rides the refresher loop
and needs a home.

**Done when.** `household-at-rest.feature` shows no `my_consent_standing` call on a resting peer, and a device
ceremony scenario shows the first ask after an idle hour listed without a retry.

## 2026-10-12 — planned; needs a decision before it is built

There is no signal to drive a refresh from. Mishpat's `post_commit` emits only `CommitmentCommitted` and
`ChallengeOutcomeCreated`, on the author's own cell, so nothing fires when a remote revocation integrates;
`src/main.rs` near line 598 records the same gap. A demand-driven read is buildable in storage alone: one reader,
triggered by an incoming ask, a pending-list or decide request, or an identity write, and served only if the last
answered read is at least 30 s old, with the 120 s evidence bound unchanged. It would bring a resting peer's
`my_consent_standing` from 10 calls per 300 s to 0.

In the safety direction it is no weaker (corrected 2026-10-13: it is weaker, see "what demand gives up" below): a node
whose record stopped standing stops listing at the next ask. In the
availability direction it is weaker than the 30 s timer. An idle node holds no evidence when a conductor outage
begins, so it lists nothing until the conductor answers, where the timer would keep listing for 90 to 120 s into the
outage. The asking device re-offers every 5 s, so an ask is listed within about 5 s of recovery, and an agreement
needs the cell to sign anyway.

The decision owed by the operator who owns consent policy: restate the invariant as "never list on evidence older than
120 s; list whenever the cell can be read", or keep the timer until mishpat emits a standing-change signal (a
coordinator-zome change). Files a build would touch: `src/services/device_carrier.rs`, `src/http.rs` (about lines
2114 to 2136 and the three readers) and `src/p2p/mod.rs` (about line 6880). Status unchanged.

## 2026-10-13 — decided and built; not measured

The operator restated the invariant: never list on evidence older than 120 s; list whenever the cell can be read. The
demand-driven read is built in storage (`services/device_carrier.rs`, `services/device_consent_cell.rs`, `http.rs`, a
counter in `happ_manager.rs`) and is the default. `ELOHIM_CONSENT_SPEAKS_REFRESH=timer`, read once at start, keeps the
30 s refresher; the mode in force is logged once at start.

On demand a resting node makes no standing call. A device's signed ask, a pending-list, decide or signed-out standing
request, and an identity write want a read. One read runs at a time, and a caller that queued behind one is judged when
its turn comes. Reads wanted by requests (one of which needs no sign-in) are at most one attempt per 30 s, answered or
not; only a device's ask may retry 5 s after a read the cell did not answer. An ask cannot wait in the transport's
event loop, so the first ask after rest wakes the reader and is dropped unanswered, as by a node whose standing is
unknown, and is listed on the device's re-offer about 5 s later. The controller's Human read rides the same read; the
listing proof has its own worker, woken by a read, and no read waits on it.

**What demand gives up against a healthy timer, in the safety direction.** The earlier planning note above said demand
was no weaker for safety. That was wrong, and an independent review showed the schedules. All of them are inside the
restated 120 s invariant:

- A new ask is listed on evidence up to 30 s old. The timer's evidence is also up to 30 s old, but its reads fall every
  30 s regardless; a demand read falls only when something asked. A record that stops standing just after a demand read
  is listed on for up to 30 s more, where the timer's next read could have caught it sooner.
- The same ceremony (same device, same peer, same request with its state and challenge) keeps its place on evidence up
  to 120 s old while the next read is made, where a healthy timer would have re-read within 30 s. If that next read is
  not answered, it stays listed until the evidence reaches 120 s. A new ceremony from a listed device is a new ask and
  takes the 30 s rule.
- Listing is not authority: a decision still runs the agree path, and the cell signs.

Availability is weaker than the timer during a conductor outage, as planned: an idle node lists nothing until the cell
answers.

**One deliberate tightening of timer mode.** Evidence is now stamped when the read's turn came, before the cell was
asked, not when the read finished. Timer evidence therefore expires up to one read-duration earlier than before, and a
read that takes longer than 120 s is unknown when it lands. Two other timer differences: the timer rests on "zome
function doesn't exist" (matched on the zome and function named) until this process applies a coordinator update or 30
minutes pass, and a failing read warns once and then logs at debug. The listing proof moved off the timer's loop to
its own worker in both modes.

Not built: a classed (background-lane) mishpat call. On demand the read has a person waiting, and the timer is kept on
its lane; the affirmation pass is the one remaining caller, two reads per 30 minutes, and the variant would touch three
source-scanning rails in `hc_client.rs`, `chain_write_gate.rs` and `cell_probe.rs`.

Not wired: a wake when the conductor's cell comes back. A reader woken with no cell spends the turn as an unanswered
read; the asking device's next offer (5 s) is the retry. Nobody else is waiting, so nothing polls for the cell.

Not verified: nothing here was run on a household. Both "done when" lines are still owed, and the second reads
"listed without the person retrying": the first offer is unanswered by design. Status unchanged until measured.
