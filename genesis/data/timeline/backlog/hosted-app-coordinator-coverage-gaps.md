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

## 2. Release adoption exits before it reaches hosted apps

`verify::already_runs_target` (`elohim/elohim-storage/src/services/release_adoption/verify.rs`)
compares the release target with one app's installed roles, and `watch.rs` exits on it. When the
node's own app is current the vehicle never runs, so its sweep of the other apps never runs either.
The vehicle's sweep only helps on the pass where the node's own app is itself behind.

Fix shape: `InstalledReality` carries every lineage-matching app on the conductor, and "already
runs target" means all of them do. Not urgent while the fleet does not deliver coordinators through
a followed channel.

## Not gaps

- The node's own app: unchanged, still swept on the readiness path.
- An app on another DNA lineage: reported as skipped (`no_role_on_bundle_lineage`) or, for a
  mixed app, per role as `dnaHashMismatch`. It needs the migration path, which mints a new key.
