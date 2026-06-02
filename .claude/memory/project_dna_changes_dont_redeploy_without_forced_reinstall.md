---
name: project-dna-changes-dont-redeploy-without-forced-reinstall
description: "A changed DNA (new hash, same roles) does NOT reach running conductors on a normal deploy — the conductor data dir is a persistent PVC and the install stale-check is role-structure-only. Needs DNA-hash drift detection (gated reinstall) or a conductor-data wipe. Cost us ~a day on the Gap-F alpha delivery."
metadata: 
  node_type: memory
  type: project
  originSessionId: d8ac4ba5-8c70-42b9-a7eb-f98689dea358
cites:
  - genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml
---

On the alpha cluster (and any embedded-conductor deployment), a DNA-content change — e.g. an integrity-zome fix that changes the DNA hash but keeps the same role structure — does **NOT** reach the running conductors via a normal edge redeploy. Three compounding facts:

1. **Conductor data dir is a persistent PVC.** `/var/local/lib/holochain` (role `holochain-data`) is a `volumeClaimTemplates` PVC in `genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml`. The installed hApp (with its DNA) survives pod restarts AND the genesis `Reset Storage` stage (which only `rm -f /data/content.db` — the storage SQL, NOT the conductor PVC).
2. **The install stale-check is role-structure-only.** `elohim-storage/src/happ_manager.rs::is_stale` (and its `.cjs` mirror `elohim/holochain/edgenode/scripts/install-happ.cjs`) only checks: are all expected roles present with provisioned cells? A new DNA hash with the same roles reads as "not stale" → no reinstall → conductor keeps the OLD DNA indefinitely.
3. **Edge redeploy doesn't reinstall.** The edge pipeline restarts edgenode pods with a new container image (new HAPP_PATH bundle baked from Harbor `elohim-happ:dev-latest`) but the embedded conductor, seeing an already-installed app that isn't structurally stale, keeps the old DNA.

**Symptom shape:** the new DNA builds fine (holochain pipeline SUCCESS, hApp pushed to Harbor), but on-cluster behavior is unchanged. For Gap-F (2026-05-28/29): genesis Seed Projections kept failing with the identical `"projection did not land in local SQL within 1s"` error across builds #1053/#1054, and a manual `RESET_STORAGE=true` run (#1055) ALSO couldn't fix it (it only wipes content.db, not the conductor PVC) — and separately timed out because the `Reset Storage` `kubectl wait` was 120s, too short for an embedded-Holochain cold start.

**The fix (landed in c60b6e036):** DNA-content **drift detection** in `happ_manager.rs::ensure_happ_installed` — probe the bundle's per-role DNA hashes (install under a throwaway app id, NOT enabled → no network join, read cell hashes, uninstall), compare to installed; on drift → uninstall + reinstall. GATED behind `ALLOW_DNA_REINSTALL` env (default false = prod-safe; reinstall mints a new agent key, which on prod needs migration/lineage not a blind wipe). Wired per-env in `elohim/holochain/Jenkinsfile::deployHumanManifest` (`env=='prod' ? 'false' : 'true'`) via an `ALLOW_DNA_REINSTALL_PLACEHOLDER` in the consolidated template; adam's hand-rendered manifest sets it directly. Also bumped the `Reset Storage` wait 120s→600s.

**Watch out:** if you force-reinstall on SOME peers but not others in the same namespace, they land on **different DNA hashes → different DHTs → P2P partition**. The alpha genesis pair (matthew templated + adam hand-coded) must BOTH get the flag. See [[project_lineage_rna_upgrade_path]] (prod DNA upgrades are migration, not blind reinstall) and [[feedback_no_kubectl_from_dev_env]] (cluster wipes are operator territory; the drift-detection code is the in-repo lever that avoids needing them).

Related: [[project_alpha_topology_bootstrap_pair]], [[project_storage_actor_vs_forwarder_patterns]], [[feedback_cascade_halt_masks_failures]] (the sweettest compile errors that masked each other on the way to even building this DNA).
