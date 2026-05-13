---
name: Doorway conductor registry persistence asymmetry
description: Two-store registry where one store persists and the other rebuilds is a recurring 502 source — diagnostic shape for any "stale-mapping after deploy" failure
type: feedback
originSessionId: cca06402-6047-484c-bbca-93f417436056
---
Doorway's `ConductorRegistry` keeps two DashMaps with mismatched lifecycles:
- `conductors`: in-memory only, rebuilt from `CONDUCTOR_URLS` env each boot
- `agents`: persisted to MongoDB collection `conductor_registry`, reloaded on boot

When the upstream config changes (reorder, count change, host rename), persisted agent rows reference conductor_ids that no longer exist in the in-memory map. `chaperone.rs` Path 2 returns 502 `{"error":"Conductor info not found"}` for those agents — they get stuck in a permanent reconnect loop with no operator intervention path.

**Why:** Conductor IDs are generated positionally — `format!("conductor-{i}")` — so the encoding is fragile to upstream config changes.

**How to apply:**
- For any Doorway 502 of shape `{"error":"<thing> info not found"}`, check `/admin/conductors` invariant: `totalAgents == sum(per-conductor agentCount)`. If it differs, the delta is the orphan count. That's load-bearing diagnostic evidence — quote it in any RCA.
- The recovery primitive that already exists is the idempotent `AgentProvisioner::provision_agent` — it walks every current conductor for an existing app for the user before installing a new one. Whenever a stale-registry-state path is detected, the right move is `registry.unregister_agent` + call provisioner. It self-heals via MongoDB upsert.
- This pattern (in-memory pool + persisted membership map) likely shows up wherever Doorway holds ephemeral infrastructure state alongside persisted user state. Audit similar pairs (route registry vs cached client mappings, etc.) when refactoring.
- The `discover_existing_agents` boot routine (main.rs:882) tries to reconcile but has an `is_none()` guard that prevents *replacing* stale entries — it only adds new ones. Don't trust the boot sweep to clean up persisted orphans.

Discovered 2026-05-06 while delivering chaperone regression fix. Commit `9d0a45a3`. See also: `project_doorway_peer_registration`, `project_doorway_single_target_no_fanout`.
