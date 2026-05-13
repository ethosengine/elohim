---
name: Cargo dep probes — resolution success ≠ compilation success
description: A `cargo metadata` / `cargo generate-lockfile` probe only proves SemVer constraints reconcile. Pre-release crates can resolve cleanly but fail to compile (stale imports, removed paths). Always run `cargo build` against the candidate pin before declaring it viable.
type: feedback
originSessionId: 6e799b4d-d829-41f0-9d59-b60202c57923
---
During iroh parallel-stack Phase 1.1 (2026-05-07), three iterations landed in the wrong place:

1. **Iter 1:** probed iroh-blobs 0.100 only, hit sha2 rc.5 conflict, pivoted to standalone iroh + custom protocol.
2. **Iter 2:** walked the iroh-blobs version range with `cargo generate-lockfile`, found 0.98 resolves cleanly. Wrote 339-line plan rewrite. **Did not compile.**
3. **Iter 3 (compile):** iroh-blobs 0.95–0.98 RESOLVE but FAIL TO COMPILE because they pull `ed25519-dalek 3.0.0-pre.1` → `curve25519-dalek 5.0.0-pre.1`, whose published source imports `digest::crypto_common::BlockSizeUser` — a path that no longer exists in current `digest`. iroh-base pins these pre-releases exactly, so no resolver-level escape. Real soak boundary turned out to be iroh-blobs 0.94 (Sep 2025), the highest version using stable ed25519-dalek 2.2 + curve25519-dalek 4.1.

**Why pre-releases hide this:** SemVer resolution doesn't validate that published source files still match the imports they declare. A pre-release crate can be published, then its transitive deps move forward (e.g., `digest` removes a re-export), and the pre-release's `lib.rs` becomes stale-but-still-resolvable.

**How to apply:** when probing Cargo dep candidates against existing workspace constraints:
1. `cargo generate-lockfile` proves SemVer reconciliation only.
2. `cargo build` (or at minimum `cargo check`) is the real gate. Run it on the candidate pin before committing the plan.
3. If pre-releases appear in `cargo tree` (any version with `-pre.*`, `-rc.*`, `-alpha.*`), treat resolution success with extra suspicion.
4. The "highest soak boundary" is often a few minor versions below the current latest — don't pin to bleeding edge just because resolution succeeds.
5. Plan-time pinning rationale should record both probes: "resolves AND compiles cleanly" not just "resolves cleanly."

**Bonus gotcha during the same session:** `cargo update -p <pkg>` propagates to transitive bumps even when the lockfile would otherwise hold pinned non-iroh packages stable. After a probe-induced cargo update, `git diff Cargo.lock` may show unrelated bumps (in this case `holochain_types 0.7.0-dev.5 → dev.22`) that break the build. Revert Cargo.lock to HEAD before re-running build to isolate the actual iroh impact.
