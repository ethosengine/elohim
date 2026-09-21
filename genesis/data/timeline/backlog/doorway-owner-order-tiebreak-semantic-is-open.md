---
id: "backlog-doorway-owner-order-tiebreak-semantic-is-open"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Owner order is now stable but deliberately arbitrary — what it SHOULD mean (registration time, declared priority, nearest) is an open design question"
slug: "doorway-owner-order-tiebreak-semantic-is-open"
written: "2026-09-21"
author: "serving-edge failover-balance-stream campaign, 2026-09-21 review"
status: "open"
priority: "low"
jobs: [elohim-edge]
tags: [federation, doorway-registry, open-question, name-routing]
---

**The fact.** `install_name_routes` (`doorway-service/src/services/federation.rs:1050-...`) now sorts holder
contracts by `doorway_id` (`:1141-1145`, commit `cd0f90155`, 2026-09-21) as the FINAL tiebreak beneath
`selector_rank`'s ranked terms (Liveness, ReachStanding, Nearest, Weight — `services/name_routing.rs:279`,
`:2210-2211`). The commit's own comment names this choice explicitly: "arbitrary-but-stable, which is exactly
what a final tiebreak must be" (`federation.rs:1136-1140`). Before this fix, `OwnerOrder` was documented as "a
stable final tiebreak" (`name_routing.rs:2211`) but was not: it silently inherited whatever order
`refresh_peer_cache`'s per-seed fan-out returned, which was in turn the DHT link order `get_all_doorways` sorts
newest-first (`infrastructure/src/lib.rs:626-627`) — so the MOST RECENTLY REGISTERED doorway won ties, not any
declared priority.

**Evidence.** `federation.rs:1129-1134` (the fix's own commit-message-quality comment): "Measured on the
household mesh 2026-09-21 (run R1): with 'garden' held by alpha and gamma, beta's fold put the MOST RECENTLY
REGISTERED doorway (gamma) first, so every relay went to gamma and the busy holder alpha was never dialled at
all." Commit `cd0f90155` ("owner order is actually stable — story 3.1 green on the household") closes the
non-determinism but explicitly keeps the tiebreak's semantic content arbitrary (alphabetical by `doorway_id`,
then `url_path`, then `host` — `federation.rs:1141-1145`).

**Why it matters.** The defect that mattered operationally — ties resolving differently on different peers,
because each peer computed its own probe order — is fixed. What remains is a design question the fix
deliberately left open rather than answered wrong: an opaque id-string sort has no relationship to which holder
SHOULD be preferred when the ranked terms above it are tied (registration recency, an operator-declared priority,
or geographic nearest would each be a meaningful tiebreak; alphabetical-by-id is meaningful only in that it is
consistent). Today's answer is fine as a stopgap because it is at least the same answer everywhere; it should not
be read as a considered final semantic.

**Smallest next step.** Decide, when the multi-owner-per-name scenario needs a load-aware answer, whether
`OwnerOrder` should carry a NEW ranked term (declared priority, or registration time) rather than remaining a
bare tiebreak beneath the four existing terms — and if so, where that value is sourced from (a DHT-notarized
declaration, since it affects DHT-visible routing, per the p2p-design-gate). Not urgent: no live scenario
currently depends on the tiebreak's semantic content, only on its stability, which is now proven
(`doorway-apex-transition.feature` story 3.1, green on the household per the commit message).

**Links.** Fix commit: `cd0f90155`. `doorway-service/src/services/federation.rs:1050-1150`
(`install_name_routes`). `doorway-service/src/services/name_routing.rs:279`, `:2210-2211` (`selector_rank`,
`OwnerOrder`'s documented role). Sibling finding, same registry:
`doorway-registry-ttl-unenforced-no-heartbeat.md`.
