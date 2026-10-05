---
id: "backlog-hosted-app-coordinator-coverage-gaps"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Two paths still leave a hosted person's app on old coordinators: apps provisioned after a coordinator-only release, and release adoption's early exit"
slug: "hosted-app-coordinator-coverage-gaps"
written: "2026-10-04"
author: "session e065030b (hosted-app coordinator sweep), with read-only findings from the integrator session"
status: "open"
priority: "medium"
jobs: [elohim-edge, elohim-holochain]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "habit:dataplane-convergence"
tags: [upgrade, coordinator-hot-swap, hosted-apps, doorway, release-adoption]
---

The conductor-wide coordinator sweep (2026-10-04, `happ_manager::sync_coordinators_all_apps`)
covers every installed app on the bundle's DNA lineage from three places: the boot path, the
node-local route with `allApps=true` (driven by `scripts/ci/fleet-coordswap.sh --all-apps`), and
the release-adoption coordinator vehicle. Two gaps remain. Both are read from code, not observed.

## 1. A person provisioned after a coordinator-only release starts behind

The doorway holds no bundle. `HAPP_BUNDLE_PATH=/opt/holochain/elohim.happ`
(`genesis/orchestrator/manifests/doorway/alpha.yaml`) is handed to the conductor as a path
(`AppBundleSource::Path`, `doorway/doorway-service/src/conductor/typed_admin.rs`), and the
conductor reads that file from its own pod, where the `happ-fetcher` init container wrote it at
pod start. A coordinator-only release does not restart the pod (the conductor roll is keyed on the
DNA hash set and `happ.yaml`), so the file stays at the previous bundle and every app installed
afterwards takes the previous coordinators.

Today's catch-up is the next driver run: a newly provisioned app is one more app on the lineage
and is swept then. Between the two it runs old coordinators.

Options: the sync route persists the bundle it applied to the conductor's bundle path; or the
doorway triggers a single-app sweep (`appId=<new app>`) after install. Decide which side owns it
before writing either; the sweep's ownership rule is that storage owns coordinators for everything
on its conductor.

Probe: provision a hosted person on the household after an all-apps apply of a variant bundle,
then dry-run the route for that app id. `drifted: true` is this gap.

**Closed in code 2026-10-04 (storage side, unit-proven; not yet measured on a mesh).** Storage keeps
the last bundle it applied at `<storage_dir>/coordinators/last-applied.happ` and a standing pass
(`coordinator_standing::spawn`, default 300 s) makes one `list_apps` call and sweeps app ids it has
not read against that bundle, eight per pass, the node's own app first. Storage owns it; the doorway
is unchanged. The probe above still stands as the mesh receipt. Design:
`coordinator-acceptance-tightening-contract.md`.

## 2. Release adoption exits before it reaches hosted apps

`verify::already_runs_target` (`elohim/elohim-storage/src/services/release_adoption/verify.rs`)
compares the release target with one app's installed roles, and `watch.rs` exits on it. When the
node's own app is current the vehicle never runs, so its sweep of the other apps never runs either.
The vehicle's sweep only helps on the pass where the node's own app is itself behind.

Fix shape: `InstalledReality` carries every lineage-matching app on the conductor, and "already
runs target" means all of them do. Not urgent while the fleet does not deliver coordinators through
a followed channel.

## 3. Hosted apps whose cells the conductor does not hold

Fleet, 2026-10-04, DNA #1482 and #1483: `update_coordinators` answered `CellMissing` for 107 roles
across 32 hosted apps (james 1 app, gertrude 11, susan 7, eve 13), most with exactly three roles
missing. `list_apps` reports those cells as provisioned and the dry run reports them drifted, so
the sweep attempts them on every run and they stay drifted. Not investigated: whether the apps are
disabled or paused, or the roles were never instantiated.

Since `616768fb0` the rolling driver names them on the peer's row and goes on
(`updated-with-unhealed`, `COORDSWAP: UNHEALED`), so they no longer stop a roll. Two things remain:

- Storage should report an app or role the conductor is not running as a skip row, not as drift
  that an apply will fail on. Needs an edge roll.
- The driver's log has no per-app `applied` row, so whether a named app was swapped cannot be read
  from it; only the failures are named.

Probe: `POST /admin/coordinators/sync?apply=false&allApps=true` on gertrude; any app in the
unhealed list above reporting `drifted: true` is this gap.

## Not gaps

- The node's own app: unchanged, still swept on the readiness path.
- An app on another DNA lineage: reported as skipped (`no_role_on_bundle_lineage`) or, for a
  mixed app, per role as `dnaHashMismatch`. It needs the migration path, which mints a new key.
