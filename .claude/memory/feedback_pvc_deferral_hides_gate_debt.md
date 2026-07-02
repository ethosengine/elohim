---
name: feedback_pvc_deferral_hides_gate_debt
title: PVC-deferral hides gate debt
description: Chronic 85%+ disk pressure defers HEAVY_GATES, so dev "green" = deferred not passed; triage integration reds by origin/dev byte-identity, not as regressions.
metadata: 
  node_type: memory
  type: feedback
  originSessionId: fb007ca7-2e24-46d8-bded-e3284050d0e1
---

When the volume sits above the 85% hard ceiling for long stretches (the container
overlay's structural baseline — `/projects` itself is tiny, the pressure is the
Docker/nix/buildkit root, NOT reclaimable by `cargo-pool enforce`), the `.husky/pre-push`
hook **DEFERS the HEAVY_GATES** (`elohim-storage epr-storage doorway steward-node
elohim-compute elohim-epr sweettest-check`). So `clippy -D warnings` and `cargo test`
**never actually run on dev pushes** — and debt accumulates UNCAUGHT for weeks.

The 2026-06-20→21 two-sprint integration surfaced three classes at once, all byte-identical
on origin/dev (proven: empty branch diffs / 0-commits-on-branch): 2 clippy errors
(redundant_guards, unnecessary_sort_by), 3 `CreateContentInput` test constructors missing
`dht_anchor_hash` (added by the resilience sprint, test sites never updated), and 2 storage
integration tests asserting on `observe_kind`'s `#[cfg(test)]` field (always `[]` cross-crate —
needs `#[cfg(debug_assertions)]`). The frontend floor is the same story (ng-build buffer
polyfill from `@holochain/client` 0.20, eslint-plugin-sonarjs S6759 crash).

**Why:** the gate that would catch debt is exactly the gate that defers under the pressure
that's always present — so "green on dev" means "deferred," not "passed." This is the silent
counterpart to [[feedback_sprint_dod_includes_prepush_gates]] (run the touched-tree gates per
task) and explains why an integration push hits a wall of "pre-existing" red.

**How to apply:** as integrator, EXPECT to surface accumulated debt the moment you actually
run the gates (bump `volume_hard_pct` per [[project_cargo_disk_guard_override]], `/tmp` target
dirs + `RUSTC_WRAPPER=""` to dodge the fingerprint-ENOENT/sccache traps). Triage each red by
**byte-identity against origin/dev** — `git diff origin/dev..HEAD -- <file>` empty / `git show
origin/dev:<file>` identical → pre-existing, NOT your regression. Fix the mechanical ones
(clippy, missing fields) to clear the gate; hand structural/infra reds (cfg-design calls,
polyfills, tooling crashes) to the shakeout shift with root causes. The elohim-app gate is NOT
in HEAVY_GATES → never defers → a pre-existing-red frontend blocks ANY frontend-touching dev
push, so `--no-verify` is the only mechanism (that's why the operator push-lease carries
`hook_bypass:true`). The real fix is disk/CI capacity, not per-push heroics.
