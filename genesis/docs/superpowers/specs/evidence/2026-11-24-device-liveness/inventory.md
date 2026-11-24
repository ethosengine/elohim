---
title: "Where a device decides liveness, 2026-11-24"
id: device-liveness-inventory
status: draft
class: process-meta
date: 2026-11-24
context-tier: disclosed
steward: human:matthew
author: "Claude (session d6cfb7e1, Opus 5.5, ethosengine device); cells gathered by a read-only search agent, the load-bearing ones re-checked by hand against dev 9f272d343"
serves: [devices-share-one-berth]
tags: [liveness, presence, berth, ark, session, stasis, inventory]
graduation-trigger: >-
  The steward rules on one presence model for the device (what is asleep, gone, heard and
  acknowledged), and each row below is either conformed to it or retired.
cites: []
---

# Where a device decides liveness

A read-only inventory of every place on one device where code decides that something is alive,
present, stale, silent or delivered. Each row gives what it reads, its threshold, what acts on
the answer, and what it cannot tell apart. Paths are on dev `9f272d343`. Line numbers are given
only where they were checked by hand; the rest name the function.

## Finding first: this device runs an older berth and older hooks

`berth` on PATH and the hooks in effect come from the main checkout, not from dev. On ethosengine
the main checkout is on `memory/ceremony-2026-11-20` (HEAD `6b10ee05f`, 2026-11-20).
- Its `genesis/agentic/bin/berth` has 2392 lines against 3658 on dev. It has no reader cursors,
  no SILENT or "exchanging again" watch in `--follow`, no claim and acknowledgement, and no second
  line on `berth say`.
- Its `.claude/settings.json` does not register `berth-note-inject.py` (dev does, at line 316).
  No session on this device has peer notes injected into its tool calls. Notes arrive only through
  a watcher the session arms itself.
- The old `inbox` parses flags by membership and ignores the rest: `--reader main --follow` runs
  as a plain follow that marks notes seen on the session's single cursor.

So every dev-only row below is absent on this device today, and a session here cannot tell from
berth's output that it runs the older behaviour. Moving the main checkout is the steward's decision.

## Decision points

| # | Where | Reads, and threshold | Decides, and who acts | Cannot tell apart |
|---|---|---|---|---|
| 1 | Mooring liveness, `Berth.is_live` (berth 1300-1316) | `kill(pid, 0)` (a permission error counts as alive); `/proc/<pid>/stat` start time against the stored `pid_start`; `now - last_seen < ttl_s`, with `mooring_ttl_s` 10800 s (pool-policy.json:193). `last_seen` moves only on `moor`, `touch` (UserPromptSubmit via ram-guard, and `berth claim`). | `live` or `STALE` in `berth status`; feeds lease takeover (row 2). | A session alive but without a prompt or claim for 3 h (an autonomous run) from a dead one: the pid check passes and the age check fails. A laptop resumed after 3 h from a gone one (wall-clock ages). SIGSTOP from running. A reused pid when `pid_start` was unreadable. `reap` keeps a mooring with a living pid and a stale heartbeat, while `is_live` calls the same mooring dead. |
| 2 | Lease expiry and takeover, `holder_is_live` and the claim path | `now - since > ttl_s`, and the holder's row 1. Verify leases default to 1800 s (max 3600). The daemon classes `mesh`, `lsp` and `device-ark` have no ttl. | `taken over from a stale holder`; `berth status` prints EXPIRED. | "Taken over only when its holder's mooring is dead" reduces to row 1: three hours without a prompt is enough, so a no-ttl daemon lease is not immune. |
| 3 | `reap` | Heartbeat age, then pid and start time. | Removes moorings (records `unmoor`, which crosses) and expired leases. | A lease reaped while its mooring is kept is recorded as `reap`, which is not in `CROSSING_KINDS` (berth:244): other devices are not told. |
| 4 | Session identity by process ancestry (`device-runtime/src/session.rs`, via `_lib/session.resolve`) | Walks up to 256 parents for a registered runtime `comm`; id `<runtime>-<pid>-<start>`. | Which session a command acts for; exit 4 when none resolves. | A detached child loses its ancestry and resolves to no session or another one. |
| 5 | Envelope owner death, `owner_dead`, `owner_observably_alive`, `holder_released` (`device-runtime/src/envelope.rs` 152, 171, 182) | `boot_id`; pid-namespace inode (a foreign namespace falls back to a flock probe on the holder lock); `/proc/<pid>` presence and start time. An unreadable stat reads as unknown, not dead. | `epr session gc` seals only dead owners; successor admission; `retire` refuses a live owner. | Handles suspend and SIGSTOP as alive, and pid reuse by start time. Cannot tell a hung owner from a working one. In a foreign namespace, a holder that died while its owner lives (a stated residual). |
| 6 | Steward lease verdict (`eprfs-peer/src/lease_view.rs`, `own_verdict`) | Other devices' acknowledgement of the head that carried the claim; `confirm_wait_s` 10, `fence_margin_s` 30, `max_skew_s` 30. | Effective, Pending, Unconfirmed (optionally fenced), RefusedBy, HeldBy; berth refuses with exit 3. | A sibling asleep from gone from partitioned: all read "unconfirmed". Clock skew: a folded lease's ttl counts from the holder's stamp against this device's clock, and skew is budgeted only on the override path. A no-ttl lease on another device does not expire by time in this code, and no check reads its mooring across devices. |
| 7 | SILENT device in `berth status` | `now - folded_at` of a sibling's head on this device's clock, against `freshness_s` 90 (pool-policy.json:196). Heads re-announce every 30 s, so an idle sibling still refreshes. A remote mooring older than 3 h shows STALE because touches do not cross. | Prints SILENT, or STALE for a remote session. | "They went silent" from "my own peer daemon is down": then every sibling reads SILENT. A closed lid from a gone device from a partition. A long-lived remote session shows STALE after 3 h. |
| 8 | SILENT and "exchanging again" in `--follow` (dev only; `_SilenceWatch`, berth 2341-2344) | Row 7's signal, polled every 60 s; skips devices with no mooring younger than the ttl. | Prints a line on each transition only. | Same as row 7. |
| 9 | `berth say` second line (dev only; `_silent_addressee_line` 2303, `_unacked_addressee_line`) | Whether the addressee is in a sibling's folded moorings, and that sibling's freshness or acknowledgement lag. | Prints "SILENT" or "UNACKED; the note is recorded and crosses". | An idle addressee from a dead one. An addressee whose mooring did not cross produces no line. |
| 10 | Unacknowledged outgoing heads (dev only; `unacked`, berth 2245; `UNACKED_LAG` 2236) | `own_seq - acked > 10` sequence numbers, about 5 min of 30 s announces. | Feeds row 9. | The code states the acknowledgement's age is unavailable. |
| 11 | Arrival line "acknowledges this device's head to seq N" (`epr-cli/src/device/arrival.rs` ~544) | Whether any sibling has folded this device's head; picks the newest `folded_at`. | `ok sibling exchange`, with no age threshold and no comparison of N to this device's current seq. | An acknowledgement from days ago from a current one. |
| 12 | Peer daemon liveness, `serve_liveness` (arrival.rs 1130/1153) | A non-blocking flock probe on the serve lock; "newest sibling head folded Ns ago" is shown as evidence only. | `ok peer daemon` or degraded. | A stopped or hung daemon holds the lock and reads as serving. |
| 13 | Daemon rejoin (`eprfs-peer/src/daemon.rs` 92, 1019) | In-memory `last_heard` against `HEAD_STALE_AFTER` 90 s. | Drops and rejoins the node. | A slow peer from a gone one, within one daemon's memory. |
| 14 | Inbox reader claim and acknowledgement (dev only; `READER_LEASE_S` 5 s at berth 977, 30 s for `--follow`; prints at 3481-3514) | Whether a pending claim's lease has expired. `--follow` acknowledges after `print(..., flush=True)`. | Busy, or take over so the notes replay. | A note written to a pipe from a note the agent read. A dead `--follow` from a quiet one: idle, it prints nothing and holds no lease. |
| 15 | `berth-note-inject` receipt (dev only; `.claude/hooks/berth-note-inject.py`, grace 20 s doubling to 900 s at 61-62) | A receipt token in the last 2 MiB of the transcript. With no readable transcript it acknowledges right after writing. | Acknowledges the claim, or re-shows the notes as a repeat. | Context persisted from context the model attended to; the token also matches if the agent quotes it. |
| 16 | ark child death (`elohim/ark/supervisor/src/supervisor.rs`, `reaper.rs`) | `waitid(WEXITED, WNOWAIT, WNOHANG)` every 250 ms; SIGKILL after the grace period, with the intent recorded. | A death witness; the passport goes to `pid=None, ready=false`; a give-up verdict past `max_deaths` within `window_s`. | SIGSTOP or a hang from running: no stopped-state handling and no probe after readiness. Who sent a SIGKILL (ark, the OOM killer, an operator), except through ark's own intent record. |
| 17 | ark readiness (supervisor.rs) | sha256 of `/proc/<pid>/exe` against the artifact, a stdout line, or a TCP connect within 200 ms; at boot only. | Ready; resets the death window. | Ready once from ready now. |
| 18 | `epr device supervise` status (`epr-cli/src/device/supervise.rs`) | ark's pid and start ticks (rejects a zombie); children's `ready` and `pid` from `passport.json` without re-reading `/proc`. | `ok device supervisor`. | A child that died after the passport was written, until ark rewrites it. |
| 19 | Stasis presence and incidents (`.claude/hooks/dialogue-stasis-sense.py`; `genesis/agentic/bin/stasis-coordinator`; `device-runtime/src/session_stasis.rs`) | "Collaborative" when the last human witness (a prompt or an approved dialog) is at most 600 s old. Incident Approach at 180 s, Breach at 300 s. A question is a "?" in the last message. "Coordinator alive" at SessionStart means its pid flock is held; `status` "fresh" means same pid, same boot, age at most 45 s. The native fold reports the coordinator as unknown. | Incident stage and the SessionStart `dialogue-stasis` line. | A human reading quietly from a human who is away. A stopped or hung coordinator from a live one (the SessionStart line reads the flock). |
| 20 | Measure overrun (ram-guard, berth `overrun_check`) | `now - claimed_at > ttl_s`. | Emits `dev-berth-held-by-measure@1`. | As row 1's clock. |

## What no row can tell apart

- **Asleep from gone.** Only the session envelope (row 5) treats a suspended or stopped owner as
  alive in every case. Berth (rows 1-3) ages heartbeats by wall clock and refreshes them only on a
  prompt or a claim, so a long autonomous stretch or a resume after a long suspend reads as STALE
  and makes the session's leases takeover-able. Every cross-device row (6-11) reads asleep, gone and
  partitioned the same way.
- **Heard from acknowledged.** Every delivery acknowledgement (rows 14, 15) proves bytes reached a
  pipe or a transcript, not that a model read them.
- **A dead watcher from a quiet one.** `--follow` prints nothing when idle and has no heartbeat;
  the silence watch speaks only on transitions.
- **Deaf from silent.** SILENT is read from this device's own fold, so a down local daemon makes
  every sibling look silent. Only claim refusals say the daemon is down.
- **A held lock from a working process.** The serve lock (row 12), the coordinator flock (row 19)
  and ark's passport (row 18) all read a stopped or hung process as healthy.
- **Clock skew.** Budgeted (30 s) only for offline overrides; folded ttls and status ages compare a
  holder's stamp with the local clock.
- **Which code runs.** A session cannot tell from berth's output that the device runs an older
  berth than dev, and an unknown flag is ignored rather than refused (the finding above).
