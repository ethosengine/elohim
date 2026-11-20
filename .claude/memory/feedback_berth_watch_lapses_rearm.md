---
name: feedback_berth_watch_lapses_rearm
title: "A lapsed berth inbox watch reads as silence"
description: "Berth inbox Monitor watches expire every 30 min; one not re-armed cost 3.5 h of a coordinator's notes unanswered (2026-11-20)."
metadata:
  node_type: memory
  title: A lapsed berth inbox watch reads as silence
  type: feedback
  originSessionId: a222db7c-0134-4e5c-abfa-6daeaede5ab1
  modified: 2026-11-20T14:09:49.095Z
---

On 2026-11-20 in the native-throughput sprint, the session's `berth inbox --follow` Monitor expired while a background verification ran, and was not re-armed. Shem's 10:00Z review verdict and 10:30Z note sat unread until another session on the device pointed at them at 13:42Z; shem had recorded "no answer".

**Why:** a coordinator reads silence as a decision. An expired watch gives no event at all, so the gap is invisible from inside the session.

**How to apply:** re-arm the poll-by-CID watcher (`genesis/local-dev/one-image/wait-notes.py`, or a Monitor that peeks `--json` every 60 s — never a `--follow` Monitor) on EVERY expiry notice, before anything else in that turn, while a coordinator is active; and before reporting "waiting on X" or going idle, `berth inbox --peek --all --json` once to catch anything missed. Related: [[feedback_cross_device_berth_notes_need_follow]].
