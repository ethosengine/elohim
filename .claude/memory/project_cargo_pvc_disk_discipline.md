---
name: project_cargo_pvc_disk_discipline
title: Cargo/PVC disk + native build-env discipline (umbrella)
description: "Cargo disk + build-env: act at 85% PVC, cargo-pool reclaim, disk-guard hook; CARGO_TARGET_DIR per workspace, no nextest here, sweettest needs RUSTFLAGS=\"\"."
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
- [[feedback_cargo_target_dir_for_native_builds]] — Native (non-WASM) cargo builds need CARGO_TARGET_DIR at the pool slot per workspace; forgotten legacy target/ dirs balloon to ~30G. WASM workspaces stay default.
- [[project_container_cargo_environment_quirks]] — No nextest here (plain cargo test, unpiped exit codes); /projects target dirs ENOENT → use /tmp; WASM-warmed slot corrupts native builds; explicit cd per gate.
- [[project_sweettest_native_build_env]] — Sweettest needs RUSTFLAGS="" (WASM getrandom flag breaks native link), BINDGEN_EXTRA_CLANG_ARGS for clang-21; `just pack` (not build) refreshes the .dna bundle.

**2026-08-25 — one slot, two cargo invocations = E0460 doctest red (not a code failure).** `just gate
elohim-storage` and a concurrent `cargo build --features "p2p p2p-iroh"` in the SAME pool slot serialize
on the build-dir lock, but the gate's DOCTEST step then fails `error[E0460]: found possibly newer version
of crate elohim_cache_core` because the feature-different build rewrote rlibs between the test build and
rustdoc. Unit/integration suites had already passed. Re-run the gate alone on a quiet slot (it went green
first try). Also: the gate's `cargo test` re-links `debug/elohim-storage` with DEFAULT features, so
always re-run the feature-full build AFTER the gate before `just mesh start` (start refuses otherwise).
