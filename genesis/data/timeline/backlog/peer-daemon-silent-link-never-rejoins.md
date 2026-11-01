---
id: "backlog-peer-daemon-silent-link-never-rejoins"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "epr peer serve keeps running after its link to a sibling dies, and never re-joins"
slug: "peer-daemon-silent-link-never-rejoins"
written: "2026-11-01"
author: "claude-opus-5-5"
status: "open"
priority: "high"
area: "epr peer serve / steward topic / device liveness"
domain: "D5"
tags: [epr-peer, relay, steward-topic, power-loss, leases, liveness]
---

# epr peer serve: a silent link never re-joins

After the 2026-10-30 power loss, ethosengine's `epr peer serve` process stayed alive, but its
link to shem died at about 03:09Z on 2026-10-31. For about 34 minutes neither device folded
the other: our last fold of shem was seq 18683 and shem's last fold of us was seq 15427. The
daemon never re-joined on its own, and nothing it reports said the link was gone. Every
steward-scoped lease reads the head table, so the two devices waited on each other: our push
was refused on a stale view of shem's claim, and shem's claim was refused as `unconfirmed`
because we were silent.

| Row | What | State |
|---|---|---|
| 1 | Restarting the daemon with the devfile recipe (`EPR_PEER_RELAY_URL`, `--listen 0.0.0.0:7777`) folded shem within 15 seconds, and shem acknowledged our heads | worked around: manual restart at 03:43Z |
| 2 | A session-local liveness loop restarts the daemon when it dies or when shem's head in `epr peer status` is older than 600 s. It fired once more (shem head 645 s old at 05:14:16Z, 17 s old after the restart), so the failure recurs | worked around: the loop lives in one Claude session and dies with it. Its first version matched `pgrep -f 'epr peer serve'`, which also matched the loop's own command line, so a dead daemon never read as dead (found 2026-11-01 when a deliberate stop went unrestarted); the match is now anchored (`^epr peer serve`) |
| 3 | The daemon itself should notice that an admitted sibling has gone quiet past a bound while its own process is healthy, and re-join the topic or re-dial through the relay, instead of depending on a process restart | open: the cure belongs in `epr peer serve` (`elohim/eprfs/epr-cli/src/peer.rs` and the topic transport under it) |
| 4 | `epr peer status` shows each sibling's head age, but nothing turns a stale age into a signal. A declared bound on sibling head age would name this state in the headline instead of leaving it to a lease refusal | open |
