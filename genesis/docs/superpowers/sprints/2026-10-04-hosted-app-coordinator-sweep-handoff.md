---
title: "Handoff — hosted apps do not receive coordinator updates"
id: hosted-app-coordinator-sweep-handoff-20261004
date: 2026-10-04
status: handoff
author: "claude-opus-5-5 (integrator, shem), at the operator's direction"
habits: [dataplane-convergence]
cites:
  - "campaign-1-4-leg2-device-authorization-20261003 | Campaign 1.4 leg 2 | sha256:a8b6df7ced5151b6 | path: genesis/docs/superpowers/sprints/2026-10-03-campaign-1.4-leg2-device-authorization.md"
  - "campaign-1-4-closing-definition-20261001 | 2026-10-01-campaign-1.4-closing-definition | sha256:98caccfd5df208af | path: genesis/docs/superpowers/sprints/2026-10-01-campaign-1.4-closing-definition.md"
  - genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md
---

# Handoff — hosted apps do not receive coordinator updates

For whoever picks this up next. It is self-contained: you should not need the conversation it came
from. Marks: **[V]** verified by reading the cited code, **[R]** reported by another session and
not re-checked here, **[I]** inference.

## The problem in one paragraph

When a coordinator zome changes, a deploy heals each node's conductor by hot-swapping the new
coordinator wasm into the cells that are already installed (`update_coordinators`: the agent key,
the cells and the DHT state survive; nothing restarts). That sweep runs for **one installed app per
node**: the node's own. A conductor that also hosts people carries one more installed app per
hosted person, and nothing sweeps those. So a hosted person keeps the coordinator their app was
provisioned with, for as long as the app lives.

## How it was found

Campaign 1.4 leg 2 (the real-device proof on the fleet) was refused on 2026-10-04. The operator's
hosted app on alpha conductor-4 (`elohim-conductor-4-927dff`) issued a head delegation; the Che
workspace's conductor, on the current coordinator, refused to act under it with
`head delegation: legacy grant cannot authorize a new publication`. The current
`grant_head_delegation` records the hash of its own on-chain issuance and the device binding; the
three delegations the hosted app issued (two on 10-03, one on 10-04) carry neither. **[R]** The
coordinator refresh of 2026-10-01 reached conductor-4's own app and not the hosted one. **[R]**

Leg 2 is stood down until the hosted operator app runs the current lamad coordinator. Re-issuing
the delegation does not help.

## What the code does today **[V]**

All in `elohim/elohim-storage/src/happ_manager.rs` unless noted.

- `ensure_happ_installed(admin_ws, happ_path, app_id)` (about line 678) finds the one app whose
  `installed_app_id == app_id` and, when it exists, calls `sync_coordinators` for it (about
  line 747). This is the boot path.
- `sync_coordinators_report(admin_ws, app_id, happ_path, apply)` (about line 1478) is the same
  sweep keyed by an app id passed in. Two callers: the node-local HTTP route below, and the
  release-adoption `coordinator-bundle` vehicle
  (`services/release_adoption/apply.rs`, about line 572), which holds one `app_id`.
- `sync_coordinators_for_app_info` (about line 1498) is the single implementation. Per role of that
  one app it reads the installed DNA definition (`get_dna_definition(cell_id)`), compares
  coordinator wasm hashes with the bundle, and when they differ and `apply` is true calls
  `update_coordinators { cell_id, source: Bundle }` (about line 1611). `update_coordinators` is
  addressed by **cell id**, which is why one app's swap does not reach another app's cells.
- Guards already there, which the fix must keep:
  - a **lineage guard**: a role whose bundle DNA hash differs from the installed cell's is refused
    (`dnaHashMismatch`), because `update_coordinators` matches integrity zomes by name;
  - the **operator gate**: `apply` needs `ALLOW_COORDINATOR_UPDATE=true` (it inherits
    `ALLOW_DNA_REINSTALL` when unset);
  - per-role failures are recorded on the report and never propagated; skipped roles are logged as
    skipped, not as clean.
- Hosted apps are installed by the doorway, not by storage:
  `doorway/doorway-service/src/conductor/provisioner.rs` calls `install_app(installed_app_id,
  agent_key, bundle_path)` (about line 251) with ids of the form `elohim-conductor-<n>-<suffix>`.
  Storage's sweep never sees those ids.

## The route that exists, and what it can and cannot do **[V]**

`elohim/elohim-storage/src/http.rs`, `handle_coordinators_sync` (about line 5655):

```
POST /admin/coordinators/sync?apply=true|false&appId=<installed app id>
Body: the raw .happ bundle (application/octet-stream, 64 MiB cap)
```

- `appId` may name **any** installed app, including a hosted one.
- `apply=false` is a dry run and is always allowed. It returns the per-role report: installed and
  bundled coordinator hashes, `drifted`, and any error. This is the way to confirm, before changing
  anything, that a hosted app is behind. The campaign's parity probe could not do this (it failed
  with `Failed to deserialize request` against the fork conductor).
- `apply=true` needs the operator gate above.
- It is **node-local on purpose**: it is not in `build_manifest()`, so no doorway proxies it.
  From outside the cluster it cannot be reached. Someone with access to the pod calls it, or a
  pipeline step does.

## Two pieces of work

### A. Unblock the operator's hosted app (small, operator-run)

Needs cluster access; no code change.

1. On the node that runs alpha conductor-4, dry-run:
   `POST /admin/coordinators/sync?apply=false&appId=elohim-conductor-4-927dff` with the `.happ`
   the fleet currently deploys. Expect the lamad role (and possibly others) reported `drifted`.
   If a role reports `dnaHashMismatch`, stop: the hosted app is on a different DNA lineage and
   this vehicle must not be used (see "Open questions").
2. With `ALLOW_COORDINATOR_UPDATE` true on that node, repeat with `apply=true`. Expect
   `applied: true` per drifted role. Nothing restarts and no key changes.
3. The Che workspace then re-issues the delegation and writer credential and runs leg 2
   (`genesis/local-dev/campaign-1.4/fleet/LEG2-RUNBOOK.md`, in that workspace only).

Which bundle: the one whose DNA hashes match the installed cells. The leg-2 note of 10-03 records
three DNA lineages in play that day (live alpha, the CI-packed DNAs, a worktree pack). **[R]** The
dry run's lineage guard is the check; do not choose by filename.

### B. Make the sweep cover hosted apps (the real fix)

**Goal.** After a deploy that carries a coordinator change, every installed app on a conductor
whose cells are on the bundle's DNA lineage runs the bundle's coordinators, and the deploy log says
so per app.

**Smallest shape that fits the code [I].** In the boot path, after the node's own app is swept,
enumerate `list_apps` and run the same `sync_coordinators_for_app_info` for each other app whose
roles appear in the bundle. Everything else is reused: the lineage guard decides eligibility per
role, the operator gate decides `apply`, the per-role report is already the unit of truth. The
release-adoption vehicle and the HTTP route should take the same "all eligible apps" form, or the
three paths will disagree about what a coordinator release reached.

**Things to decide before writing it.**

1. **Cost at boot.** One `get_dna_definition` per role per app, and one `update_coordinators` per
   drifted role per app. The lane cast is 14 hosted people plus prologue registrants on one
   conductor; alpha conductors carry more (adam held about 58,000 capability grants across its
   apps on 10-03 **[R]**). Bound the work before the call: a conductor call cannot be cancelled
   (rule `conductor-call-is-uncancellable` in `elohim/elohim-storage/src/.epr-meta`). Sweep in
   bounded batches off the readiness path, and make the node ready on its own app first.
2. **Who owns a hosted app's coordinators.** The doorway installs hosted apps and storage would be
   updating them. Either storage sweeps everything on its conductor (simple; one owner of
   "coordinators on this conductor"), or the doorway's provisioner owns its apps' coordinator
   state. Pick one and say so; two sweepers on one cell is the failure to avoid.
3. **Which apps are eligible.** By lineage, not by name: an app is swept only for roles whose DNA
   hash equals the bundle's. Do not parse the `elohim-conductor-<n>-…` id shape. Note the id shape
   itself is changing: the unlanded branch `fix/hosted-provisioning-stop-damage` moves hosted app
   ids from a 6-hex suffix to a full-width hash.
4. **Reporting.** The boot log and the HTTP report must carry one row per (app, role). Today's
   report has no app dimension except the single `app_id`. A sweep that reports "3 roles applied"
   without saying for which of 20 apps repeats the 2026-09-06 mixture the code comments describe.
5. **Does `get_dna_definition` work against the pinned fork from storage?** The JS client's call
   failed with `Failed to deserialize request` on 10-02. **[R]** Storage uses the Rust client and
   the boot sweep already depends on it, so it presumably works **[I]**; confirm with the dry-run
   route on the household before relying on it for N apps.
6. **New hosted apps.** A person provisioned after a deploy gets whatever bundle the doorway holds
   (`bundle_path`). Check that the doorway's bundle is the deployed one, or new people start behind.

**Where it lands.** Coordinator-only: no integrity change, no DNA hash move. Storage source changes
under `elohim/elohim-storage/src/`; if the doorway takes ownership, also
`doorway/doorway-service/src/conductor/`. `just gate elohim-storage` (the local gate now pins Rust
1.96.1). A change under `doorway/doorway-service/src/` also needs the household serving receipt at
push (`just test mesh features/dataplane/epr-app-deliverability.feature`).

**Scenario first.** Find or write the story before the code. Candidates to read:
`genesis/a2o/features/delivery/workspace-to-fleet-release.feature` (a coordinator release reaching
the fleet) and the hosted-human stories under `genesis/a2o/features/auth/`. The missing station, in
the form the story graph wants:
`chain: coordinator release reaches the fleet / between "the node's own app runs the new
coordinator" → "a hosted person's call is served by the new coordinator" / missing node: every
lineage-matching installed app on the conductor is swept / state: open.`

**Proof on the household (Act I), before the fleet.**
1. Start the household and run the prologue so hosted apps exist (`just mesh start`,
   `just mesh prologue`).
2. Build a `.happ` with a coordinator-only change (any visible behaviour change in a lamad
   coordinator function).
3. Dry-run the route for a hosted app id: it reports drift. Restart storage (or call the route
   with apply): the report shows one row per (app, role), the hosted apps included.
4. Call the changed function as a hosted person and as the node's own agent: both see the new
   behaviour. Agent keys and cell ids are unchanged before and after.
5. Negative: an app on a different DNA lineage is reported `dnaHashMismatch` and left alone.

**Proof on the fleet.** After landing, the per-app report in each node's boot log; then leg 2
itself is the acceptance test (the operator's hosted app issues a delegation the current zome
accepts).

## Open questions

- Is the operator's hosted app on the same DNA lineage as the deployed bundle? If not, piece A is
  not available and the app needs the reinstall or migration path, which mints a new agent key.
- How many hosted apps are on each alpha conductor today, and how many are behind? The dry-run
  route answers both, per node.
- Whether any path other than the three named above updates coordinators. A search of
  `elohim/elohim-storage/src` and `doorway/doorway-service/src` for `update_coordinators` and
  `sync_coordinators` found only those. The Che session, reading the same code, also found no sweep
  over hosted apps and did not rule out another path.

## State of the fleet at handoff (2026-10-04 12:30Z)

- `dev` at `1f3ae6575`. All seven alpha conductors on fork `5f4c16abe` (grant lookup keyed), CPU
  limits split 3/4 conductor, 1/4 storage on six of seven, the operator registration gate live on
  both doorways.
- Not landed: `fix/hosted-provisioning-stop-damage` (tip `9ac56f6ee`), which rewrites hosted
  provisioning and changes hosted app ids. Piece B should be built with that branch in view, or on
  top of it once it lands.
- Leg 2: stood down. The delegation and credentials issued on 10-04 lapse at 13:04Z and are in the
  old format in any case.
