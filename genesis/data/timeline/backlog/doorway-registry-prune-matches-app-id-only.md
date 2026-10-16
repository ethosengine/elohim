---
id: "backlog-doorway-registry-prune-matches-app-id-only"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The doorway registry's boot prune keys on app id alone, so every stale mapping that names a pod's own app \"elohim\" survives; and a doorway-phase account can hold a custodial key with no cell behind it"
slug: "doorway-registry-prune-matches-app-id-only"
written: "2026-10-15"
author: "hosted-cast cleanup, alpha 401 follow-up, 2026-10-15"
status: "open"
priority: "medium"
jobs: [elohim-edge]
tags: [doorway, conductor-registry, hosted-human-lifecycle, alpha]
---

**The fact.** `ConductorRegistry::prune_absent` (`doorway/doorway-service/src/conductor/registry.rs`, landed
a33593660) drops a mapping when its `app_id` is not in the conductor's `list_apps`. A pod's own agent is
installed as app `elohim`, and each conductor reinstall mints a new agent under that same app id. Every
mapping to an earlier `elohim` agent therefore survives the prune: on alpha after the c06e99a9 deploy,
`GET /admin/conductors/conductor-N/agents` lists 17 to 27 `elohim` rows per pod (about 8 agents in three key
forms each), where one is real. Fixed in the commit that adds this row: the prune now compares
(app id, canonical agent key) against `list_apps`, which already returns both
(`an_earlier_agent_under_the_pods_own_app_id_is_pruned`). The alpha registry shows it after the next
doorway boot.

**The second fact.** Matthew's alpha account (`matthew.dowell@alpha.elohim.host`, phase doorway) carries a
custodial key, `hCAkmO-…`, from a hosted app `elohim-conductor-6-c16c7d` that had already gone from conductor-6
(eve's pod) by 2026-10-14T21:10Z. Loki shows `/hc/connect` resolving to that app and failing
`App 'elohim-conductor-6-c16c7d' not found on conductor` after 20 polls. The 14:21Z prune on 10-15 dropped
the dead mapping, so login now mints `conductor_id: conductor-6` with no `installed_app_id`, and `/hc/connect`
answers 401 "Session is missing its installed app binding" at once (genesis #1647, #1648,
observable-distribution After hooks). The prune did not cause this; it made the failure quick and named.
His own pod is conductor-1. Re-binding the operator's admin account to his own key (or closing and
re-seeding it) is an operator decision. The doorway-phase registration branch never takes the
pod's key the way node/device registration does since 9b98ac904, so a re-seed alone may not bind it.

**The third fact (2026-10-16, after the 124524a44 roll).** Hosted placement reads the counts the prune
corrects. `find_least_loaded_excluding` (`registry.rs` ~361, called from `provisioner.rs` ~180) takes the
largest `capacity_max - capacity_used`, and `capacity_used` is seeded from the registry's agent counts at
boot. Before the fix, adam's conductor-0 seeded at 94/50 and was never chosen; afterwards conductor-0 and
conductor-1 both seed at 14/50, and `max_by_key` breaks the tie by DashMap order, which picked adam. adam
is the saturated pod: 96-220 `get_links … response channel dropped` / read-permit timeouts an hour since
before either roll. All three genesis #1649 registrations (04:03-04:04Z) went to conductor-0, their cells
completed genesis, and `create_human` timed out at 10 s. Failed registrations leave the installed app behind
uncounted, so the tie persists. Under that sits a capacity model that cannot hold: each hosted install
keeps about 0.7 GiB of conductor heap until the conductor restarts (adam 1.17 → 3.18 GiB for three installs;
matthew OOMKilled at the 5 GiB limit on 10-15 19:52:53Z, the genesis #1648 `ConnectionClosed`), yet alpha
declares `capacity_max` 50. Every genesis run registers hosted humans, so a hosting conductor reaches its
memory limit about every second run. What to settle: a capacity bound derived from the pod's memory limit
(or the doorway counting installs rather than agents), a deterministic tie-break, and uninstalling the app
when `create_human` fails. Choosing the bound and whether adam keeps hosting is the operator's call. Catalogued as its own performance
entry, with the code read and the fix shape: [doorway-hosted-capacity-bound-from-pod-memory](epr:doorway-hosted-capacity-bound-from-pod-memory).

**Done when.** The alpha registry lists one `elohim` agent per pod after a doorway boot
(`GET /admin/conductors/conductor-N/agents`), and Matthew's alpha session reaches his own cell on
conductor-1 (`/hc/connect` 200 in the observable-distribution scenario).
