---
id: "backlog-hc07-lane-d-doorway-client-family"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Holochain 0.7 — Lane D: doorway-service onto the 0.9.0 / 0.7.0 client finals"
slug: "hc07-lane-d-doorway-client-family"
written: "2026-09-02"
author: "holochain 0.7 upgrade guide (Lane D)"
status: "open"
priority: "high"
tags: [holochain-0.7, doorway, dependency-pins, codex-claimable, lane-d]
cites:
  - genesis/docs/superpowers/plans/2026-09-02-holochain-0-7-upgrade-guide.md
---

# Lane D — doorway-service client family (claimable by any agent; no session context assumed)

Part of the Holochain 0.7 upgrade guide (see cite; read its **§Global Constraints**, **§Version
Table** and **§0.7 code-migration patterns** first — they govern). **Write-set: `doorway/doorway-service/**` only.**
Other lanes own everything else; do not touch them.

## Context

- Holochain 0.7.0 shipped 2026-07-30. The whole substrate moves in one atomic family batch on branch
  `upgrade/holochain-0.7`; this lane is one disjoint slice of it.
- `doorway/doorway-service/Cargo.toml:57-75` pins the UNRELEASED 0.7 dev line
  (`holochain_client =0.9.0-dev.24`, `holo_hash =0.7.0-dev.9`, `holochain_zome_types =0.7.0-dev.15`,
  `holochain_types`/`holochain_websocket`/`holochain_conductor_api =0.7.0-dev.23`,
  `holochain_serialized_bytes =0.0.57`) and `:251` path-patches a vendored hsb 0.0.56.
- The 0.7 finals: `holochain_client =0.9.0`, `holo_hash =0.7.0`, `holochain_zome_types =0.7.0`,
  `holochain_types =0.7.0`, `holochain_websocket =0.7.0`, `holochain_conductor_api =0.7.0`,
  `holochain_serialized_bytes =0.0.57` (crates.io, no path patch).
- Known API moves: `CapAccess` → `CapAccessType` (`src/conductor/typed_admin.rs:19` import, `:248`
  `CapAccess::Assigned { secret, assignees }`; a `// 0.7 migration:` breadcrumb sits at `:17`);
  `dump_network_stats` now returns the unified transport-stats type (consumer in `src/routes/health.rs`);
  `AppStatus` is a union (`Enabled` / nested `Disabled(reason)`); `Action` is `{ header, data }`
  (common fields via `action.author()` etc.; variants are `ActionData::*`).
- **NOT in this lane:** removing the tx5 signal server (`src/signal/**` and its routes). It is retired
  in Lane G after the fleet is green on 0.7.

## Steps

1. Pins: apply the finals above; delete the `[patch.crates-io] holochain_serialized_bytes = { path = "../../vendor/holochain_serialized_bytes-0.0.56" }`
   line at `:251`; replace the "WHY dev.23 IS THE FLOOR" comment block with one line ("0.7.0 finals,
   matching the 0.7.0 conductor"). `cargo update -p holochain_client -p holochain_types -p holochain_zome_types -p holochain_websocket -p holochain_conductor_api -p holo_hash`.
2. Build, compile-driven: set `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/doorway__doorway-service/dev`
   (never bare `target/`), then
   `RUSTFLAGS="" flock -w 7200 /projects/.cargo-target-pool/upgrade-0.7.lock cargo build --release 2>&1 | grep -E '^error' -A6 | head -60; echo EXIT=$?`.
   Fix each error per the guide's pattern table. Mechanical adaptation only — if a removed field has
   two replacement paths, STOP and report rather than choose.
3. Bootstrap wire check: unpack `kitsune2_bootstrap_srv` 0.4.1 and 0.5.0
   (`curl -sfL https://static.crates.io/crates/kitsune2_bootstrap_srv/kitsune2_bootstrap_srv-<v>.crate | tar xz`)
   into a scratch dir, `diff -ru` their `src/`, read every route/handler change, and write the verdict
   as a comment block at the top of `src/bootstrap/k2.rs`: `CONFIRMED-COMPATIBLE` with the changed
   routes listed, or the exact handler that must change (and change it). The relay is served by
   separate `iroh-relay` deployments, not by doorway — the integrated-relay feature is out of scope.
4. Gates (all must print EXIT=0):
   `RUSTFLAGS="" flock -w 7200 /projects/.cargo-target-pool/upgrade-0.7.lock cargo test --lib --bins 2>&1 | tail -20; echo EXIT=$?` ·
   `cargo clippy -- -D warnings 2>&1 | tail -5; echo EXIT=$?` · `cargo fmt --check; echo EXIT=$?` ·
   `just gate doorway-service`.
5. Sweeps over `doorway/doorway-service`:
   `grep -rnE '\.(action|content)\.(author|timestamp|action_seq|prev_action)\b' src` (expect 0 unless
   migrated to `.header.`), `grep -A1 '^name = "hd[ik]"$' Cargo.lock` (at most one version each).

## DoD

All gates EXIT=0 with output pasted in the report; one path-limited commit on a work branch:
`git commit -m "feat(doorway): holochain client family 0.9.0/0.7.0 finals; CapAccessType; bootstrap wire verified against kitsune2 0.5.0" -- doorway/doorway-service`.
**Commit-only; never push** — the integrating session reviews and folds it into `upgrade/holochain-0.7`.
If the 0.7.0 family fails to resolve against another doorway dependency, STOP, revert, and report the
exact conflict — a documented conflict is a valid outcome; a silent partial bump is not.
