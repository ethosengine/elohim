---
name: feedback_cross_device_berth_notes_need_follow
title: "A standby session on another device must follow its berth inbox; a note nobody polls costs hours"
description: "Cross-device berth notes raise nothing: watch your inbox by polling `berth inbox --peek --all --json` by CID (wait-notes.py), NEVER a `--follow` Monitor (it dropped live notes on 11-09 and again 11-14). Standby briefs must say the same."
metadata:
  type: feedback
  originSessionId: 184beb01-43bc-4c74-9212-9f7189301998
  modified: 2026-11-14T12:02:26.203Z
---

During re-genesis 1 the orchestrator (ethosengine) sent shem three `berth say` notes over four
hours (a storage ruling to implement, "keep the household up", "dev landed at 78e0ba08c"). Shem
read them together at 07:10Z: it had been "standing by" with no inbox poll. The channel is a file
inbox that nothing raises; `berth inbox` shows notes only when asked.

**Why:** cross-device coordination has no push; a standby session that is not following its inbox
is deaf, and the cost is silent (the orchestrator sees a moored session and assumes it is working).

**How to apply:** every brief that puts a session on standby ends with "arm the poll-by-CID watcher
(`genesis/local-dev/one-image/wait-notes.py <session> <hours>` via run_in_background, or a Monitor
that peeks `--json` every 60 s) and ack each note"; the orchestrator keeps its own inbox
Monitor armed; when a moored session is silent past a task's natural length, send a one-line ping
rather than waiting. Related: [[project_brit_worktree_gaps_2026_11]].

**Poll by running `berth inbox --peek`, never by reading notes.jsonl (2026-11-09):** the inbox file
(`<berth>/inbox/notes.jsonl`) is a projection that advances only when `berth inbox` runs its scan of
`<channel root>/.eprfs/status/flows.jsonl` (cursor in `inbox/offset.json`). A wait loop that read notes.jsonl
directly sat at 17:06Z for six hours while every shem note was already folded in flows.jsonl, and looked like a
one-way channel. `--follow` Monitors also failed to surface notes. Wait with a loop that calls
`berth inbox --session <id> --peek --json` (filter stasis boilerplate), and when a peer says it is waiting on you,
run `berth inbox --peek --all` before answering. Shem's own earlier notes failed silently for a different reason:
its berth used a stale epr on PATH (fixed with `EPR_BIN`).

**Address the session, not the device (2026-11-08):** `berth say --to shem …` accepts a device name and delivers nothing — two re-genesis notes (lane release, survey brief) sat undelivered ~17 min. Address the session id (`berth say --to a8efbf5a …`) and confirm with `berth ledger | grep <id>`.

**Judge "new" by note identity, never by its timestamp (2026-11-10):** a note can reach the inbox late
carrying its original `at` (shem's 08:26 "E4 ready, can't claim the lane" sorted after a 15:58 note),
so a wait loop filtering `at > start` skipped it for ~8 h. Snapshot the inbox's note keys at start
and report any unseen key: `genesis/local-dev/one-image/wait-inbox.sh <session> <hours>` (since superseded by `wait-notes.py`).
After a session resume, `epr` refuses `envelope-owner-mismatch` until `epr session gc` +
`epr session open --model <id>`; `epr peer status` from an older CLI than the running daemon
reports "no daemon running" — ask the daemon's own binary.

**2026-11-11:** `berth inbox --all` also MISSED a household broadcast (shem's new session 3dd97e3d, to ALL, asking for A1 vectors); only a raw scan of `.eprfs/status/flows.jsonl` for `env:kind=note` found it. Watch with `genesis/local-dev/one-image/wait-notes.py <session> <hours>` (by CID, addressed-to-me or ALL), and when the user says "check the peer channel", scan the raw store, not just the inbox.
**Also 2026-11-11:** a watcher that snapshots "seen" at START misses replies that land seconds before it starts (shem's two 04:27Z replies sat unread ~4 h). Persist the seen-set to a file and mark a note seen only when it is printed: `wait-notes.py` does this (`--mark-all` after reading by hand). Run it with run_in_background, never `&` inside a command (no completion notice, and it consumes notes silently).
**Also 2026-11-11 (the costly one):** notes are addressed by session-id PREFIX (`env:to=ab4528a1`), so a watcher matching `to == full id` silently drops every directed note. It hid six of shem's notes for ~3 h, including their lane claim, and my landing script then raced their held lane (the pre-push hook caught it). Match `full_id.startswith(to)`. Separately: a landing script must abort unless `berth claim` succeeds, never just log the claim's output.
**2026-11-13 (the mirror case):** I sent four notes with `berth say --to 3dd97e3d` (a bare prefix) and shem's session received none of them for ~2 h, while their full-id notes reached me. `berth say` accepts any `--to` silently. Always pass the FULL session id from `berth status` to `--to`, and spot-check `berth ledger` that the arrow names the full id. Receivers should keep the prefix match; senders must not rely on it.
**2026-11-14 (repeated despite this note):** at the start of an overnight shift I armed a `berth inbox --follow` Monitor again; in ~60 min it delivered neither of shem's two new notes (a lane-hold heads-up and LANDED 30/31 with a lockfile warning for my branch). Only `--peek --all` showed them. The poll-by-CID Monitor (peek `--json` every 60 s, seen-set seeded from the current inbox, append each printed CID) is the default watcher; arm it first thing on any shift.
