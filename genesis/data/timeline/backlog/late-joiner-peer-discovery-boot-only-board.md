---
id: "backlog-late-joiner-peer-discovery-boot-only-board"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A peer joining between fleet boots is never discovered by the reconcile stream — the doorway board is read at boot-seed and on empty-book watch only, so late-joiner supply (inventory, hints, carried elections) waits for the next full fleet reboot"
slug: "late-joiner-peer-discovery-boot-only-board"
written: "2026-08-31"
author: "shift 2026-08-31T02-40-fleet-carried-election-convergence"
status: "open"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-sovereign-peer-network-read-no-authorities"
  - "habit:dataplane-convergence"
tags: [dataplane, discovery, bootstrap, late-joiner, membership]
---

Measured 2026-08-31: the workspace supplier peer W2 registered on the doorway
bulletin board at 13:52, fifteen minutes AFTER the 13:37 fleet roll. No fleet
peer ever polled it: `doorway_bootstrap.rs` reads the board at boot-seed and
then only when the peer book is EMPTY (watch phase), so a book seeded with the
fleet's own 7 entries never re-reads. Gossip neighborship formed on every
topic (transport manifest included) but did NOT add W2 to any fleet book. The
2026-08-31 15:10 `[build:edge]` re-roll was fired purely to force boot-time
discovery — a fleet reboot as a membership operation, the documented cost.

Cure direction (vision-level, per the operator's 2026-08-31 course-set): a
LIVING membership plane, not a polling tweak — the board read becomes
periodic and/or the transport-manifest gossip merge adds unseen peers to the
book at runtime. Design belongs with (or inside) the p2p upgrade-propagation
arc, since organic joining is the same seam. Fixture note: the household mesh
cannot currently model a late joiner — see
backlog-mesh-fixture-fidelity-regimes.
