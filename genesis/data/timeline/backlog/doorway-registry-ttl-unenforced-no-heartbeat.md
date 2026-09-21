---
id: "backlog-doorway-registry-ttl-unenforced-no-heartbeat"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Doorway registry TTL is advertised but never enforced — get_all_doorways returns every registration with no staleness filter, and DoorwayRegistration has no heartbeat field to filter on"
slug: "doorway-registry-ttl-unenforced-no-heartbeat"
written: "2026-09-21"
author: "serving-edge failover-balance-stream campaign, 2026-09-21 review"
status: "open"
priority: "medium"
jobs: [elohim-holochain, elohim-edge]
tags: [federation, doorway-registry, discovery, ttl, infrastructure-zome]
---

**The fact.** `get_all_doorways` (`elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs:619-649`)
reads the fixed `"__all__"` anchor's links, sorts them "newest-first so the first record seen per id is the live
one" (`:626-627`), dedups per doorway id, and returns every surviving registration — with no age check anywhere
in the function. `doorway_endpoint_ttl_secs` (`doorway-service/src/config.rs:268-270`, default 300s, doc'd "Cache
TTL carried by signed doorway endpoint records") is read by whatever consumes the signed record, but nothing
compares it against a last-seen time before deciding whether a registration is still `online`. It cannot: the
`DoorwayRegistration` entry (`infrastructure_integrity/src/lib.rs:90-108`) has no last-seen or heartbeat field at
all — only `registered_at`/`updated_at` strings, neither of which the extern consults. The staleness primitive
that WOULD do this exists in the same crate for a different entry: `ContentServer::is_stale`
(`infrastructure_integrity/src/lib.rs:195-199`, `now.saturating_sub(self.last_heartbeat) > max_age_secs`) — but
`ContentServer` is a separate entry type from `DoorwayRegistration`, and no equivalent field or method exists on
the doorway registration itself.

**Evidence.** Direct code read, no live-mesh probe attached (a `get_all_doorways` call against a running household
was not run for this finding — the code-level absence stands on its own and does not depend on it). `doorway_id`
re-registers on every boot (`infrastructure/src/lib.rs:619-627` doc comment: "a doorway re-registers at each boot
... appending a link + entry per registration, so collapse to the newest record per id"), so the mechanism that
DOES exist (newest-wins dedup) hides a doorway that stopped re-registering only until the next time some other
doorway with the same id boots — an unbounded window, not the advertised `doorway_endpoint_ttl_secs`.

**Why it matters.** Any consumer of `get_all_doorways` (peer discovery, federation routing tables) can act on a
doorway registration long after that doorway process has died or been decommissioned, with no signal
distinguishing "still serving" from "registered once, never withdrawn." The 300s TTL a client is told to expect
from the signed record is not actually the bound anything enforces at the read side.

**Smallest next step.** Two candidate shapes, DNA-hash-moving vs not: (a) add a heartbeat/last-seen field to
`DoorwayRegistration` (integrity-zome change, moves the DNA hash — needs the upgrade-governance ladder) so
`get_all_doorways` can filter with the same `is_stale`-shaped predicate `ContentServer` already has; or (b) keep
the entry as-is and have the READER (federation peer-probe / route-registry refresh) apply a heartbeat-backed
filter using an operational signal outside the DHT entry (coordinator-only, hash-neutral). Name which is chosen
before implementing — this is a p2p-design-gate question (does staleness belong to the notarized entry or to an
operational projection of it).

**Links.** `elohim/holochain/dna/infrastructure/zomes/infrastructure/src/lib.rs:606-654` (`get_all_doorways`).
`elohim/holochain/dna/infrastructure/zomes/infrastructure_integrity/src/lib.rs:90-108`
(`DoorwayRegistration`), `:171-199` (`ContentServer::is_stale`, the sibling primitive). `doorway-service/src/config.rs:268-270`
(`doorway_endpoint_ttl_secs`). Sibling finding, same registry: `doorway-owner-order-tiebreak-semantic-is-open.md`.
