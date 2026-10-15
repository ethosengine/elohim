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
forms each), where one is real. The prune should compare (app id, agent key) against `list_apps`, which
already returns both.

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

**Done when.** The prune drops a mapping whose agent is not the agent of that app on that conductor, with
a unit test, and the alpha registry lists one `elohim` agent per pod after a boot; and Matthew's alpha
session reaches his own cell on conductor-1 (`/hc/connect` 200 in the observable-distribution scenario).
