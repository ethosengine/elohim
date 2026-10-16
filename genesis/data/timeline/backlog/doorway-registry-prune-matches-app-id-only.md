---
id: "backlog-doorway-registry-prune-matches-app-id-only"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The doorway registry's boot prune keys on app id alone, so every stale mapping that names a pod's own app \"elohim\" survives; and a doorway-phase account can hold a custodial key with no cell behind it"
slug: "doorway-registry-prune-matches-app-id-only"
written: "2026-10-15"
author: "hosted-cast cleanup, alpha 401 follow-up, 2026-10-15"
status: "backlog"
priority: "medium"
jobs: [elohim-edge]
shift_objective: |
  Prune stale app/key mappings and rows for confirmed retired conductors, including conductors
  absent from the doorway list. Show accounts with no installed app as stuck, distinguish unknown
  lookups, and offer the remedy to the owner. Reuse the operator-standing branch work; cleanup
  needs a seat decision and must never change an account key.
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
His own pod is conductor-1. The doorway-phase registration branch never takes the
pod's key the way node/device registration does since 9b98ac904, so a re-seed alone may not bind it.

**Owner remedy (2026-10-16).** A hand Mongo edit moved the account from `hCAkmO-…` to the owner's
node key, `hCAkJH75…`, after the hosted app vanished when its pod was retired. Operator and owner
were the same person, but the edit used the wrong actor's authority. The operator never re-points
a key. The owner chooses graduation or re-hosting on their own account page, following the
[own-node-key design](../../../docs/superpowers/specs/2026-10-16-graduate-onto-own-node-key-design.md).
No dashboard control, agent loop or Mongo edit substitutes for that act.

Use the registry and provisioner lookup to list accounts whose `UserDoc.agent_pub_key` has no
installed app on any registered pool conductor. This is read-only: failed lookups mean unknown,
not absent. Show confirmed cases as stuck with their current key in the `doorway-app` hosting
view, and address the remedy to the owner. The graduation view currently exposes only read-only
`/admin/graduation/pending` and `/admin/graduation/completed` surfaces.

**Placement after the prune (2026-10-16).** After the 124524a44 roll, conductor-0 and conductor-1
both seeded at 14/50 instead of conductor-0's stale 94/50. DashMap tie order picked saturated
adam for all three genesis #1649 registrations (04:03–04:04Z). Cells completed genesis but
`create_human` timed out at 10 s. Installed leftovers, heap evidence and the capacity remedy
have one home: [hosted capacity](doorway-hosted-capacity-bound-from-pod-memory.md).

**Remaining prune scope (alpha, 2026-10-16).** Registry rows for earlier runs remain:
conductor-4 about 20, conductor-6 about 17, conductor-1 three (newest 11:49Z). The apps are
likely still installed and holding heap; registry rows alone do not prove that. Check actual
installs and send app cleanup to the hosted-capacity work. Rows for conductor-10 through
conductor-13 never clear because cleanup walks only the doorway's current conductor list.
The prune must also inspect rows whose conductor is absent from that list. Delete only after
retirement is confirmed; a timeout alone is not retirement. This concerns pool mappings, not
[DHT doorway-registration TTL](doorway-registry-ttl-unenforced-no-heartbeat.md).

**Seat boundary (2026-10-16).** Follow the
[operator-loop design](../../../docs/superpowers/specs/2026-10-09-elohim-operator-loop-wedged-peer-design.md):
pain → agent loop → seat → actuator. The loop gathers evidence and proposes an act; the seat
checks standing, capability and bounds before system code executes it. At birth Permit alone
covers only adding holders without removing anything, announcements and informing parties
within bounds. Everything else Refers. Registry deletion therefore Refers; an unanswered Refer
leaves it pending. Record the decision and outcome. Widen the seat only with evidence.
Informing an owner gives no authority to perform their ceremony. Human-owned app release and
the other hosting actuators follow the [capacity entry](doorway-hosted-capacity-bound-from-pod-memory.md).
Hand edits in k8s or Mongo are not these actuators.

**Operator-standing dependency.** Reuse branch work in `.worktrees/doorway-operator-surface`
(commits `5f66681fd`, `fb3c12867`, `991e10241`), not yet on dev. `5f66681fd` carries the story
`genesis/a2o/features/doorway/operator-standing.feature`, the habit
`doorway/doorway-service/.epr-meta/operator-dashboard-under-standing.habit.md`, the plan
`genesis/docs/superpowers/plans/2026-10-16-doorway-operator-surface-under-standing-plan.md`
and operator bindings. The story includes the stuck-account owner boundary. `fb3c12867` adds
interim Admin refusal on eight actuators; it does not complete standing checks. `991e10241`
clarifies the binding issuer: the network's bootstrap steward (`progenitor_pubkey`), read from
the network. Reuse that dashboard and per-action capability work; the habit remains red.

**Done when.** Household scenarios prove authorized cleanup and refusal, including rows outside
the current list; each mutation names its seat decision. After boot alpha lists one `elohim`
agent per pod and no confirmed-retired conductor mappings. Missing apps appear as stuck;
unreachable conductors remain unknown; healthy hosted accounts are not flagged. The operator
has no key-changing control. The owner receives the remedy and can complete it from their own
account; Matthew's session reaches his own cell on conductor-1 (`/hc/connect` 200 in
observable-distribution).
