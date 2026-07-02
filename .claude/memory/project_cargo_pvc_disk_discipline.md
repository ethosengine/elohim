---
name: project_cargo_pvc_disk_discipline
title: Cargo/PVC disk-pressure discipline (umbrella)
description: "PVC/cargo disk discipline: act at 85%+, cargo-pool legacy-targets reclaim, disk-guard hook denies heavy cargo, DWARF footprint, multi-agent cargo pacing."
metadata:
  type: project
---

# Cargo / PVC disk-pressure discipline (umbrella)

Folds the disk-pressure and cargo-target-pool discipline cluster. Members:

- [[feedback_multi_agent_pvc_pacing]] — never run cargo test/build concurrently across agents; shared target-pool locks + PVC disk/RAM contention crash builds.
- [[feedback_pvc_threshold_and_recovery]] — 118G PVC: above 85% used, act; cargo-pool legacy-targets --clean --yes reclaims ~25-35G; check df <80% before dispatching any cargo agent.
- [[project_devspace_disk_cleanup_procedure]] — pool families dominate disk pressure; act at 85%+; reclaim ladder ends in operator-gated family prune — never prune the active family mid-push.
- [[project_cargo_disk_guard_override]] — at the 85% hard-ceiling the PreToolUse hook DENIES heavy cargo; FORCE_HEAVY_GATES does not bypass it — free non-pool space or bump volume_hard_pct.
- [[project_rust_build_footprint_anatomy]] — 71% of pool = ~1GB DWARF test binaries (79% debuginfo); retention policy not Rust is the cause; evict first; −57% profile landed in root .cargo/config.toml.
