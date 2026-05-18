# Compilation Load Baseline — 2026-05-17

> Closing measurement of P1 (cargo-registry-and-compilation-load-reduction sprint).
> Captured 2026-05-18 immediately after T12 (workspace.dependencies unification) landed.

## Scope

Workspace at `/projects/elohim/elohim/`. Measurements taken on Eclipse Che dev container,
CARGO_TARGET_DIR pinned to the family-dev pool slot per the cargo-pool steward.

All builds run with `RUSTFLAGS="" RUSTC_WRAPPER=""` (native builds; sccache excluded from
build-timing measurements for apples-to-apples comparison — sccache stats captured separately).

## Workspace size

- Member crates: **149** (count of Cargo.toml files, excluding `*/target/*`)
- Total Rust LOC: **104,411**
- Workspace tree size (excluding target/): **109 MB**

## Dependency unification state (post-T12)

- `[workspace.dependencies]` entries: **34** (26 external + 8 internal path deps)
- Stragglers remaining: **0** (per T11 audit + T12 migration)
- Transitive duplicate crate names: **19 unique, 0 actionable** (per T13 baseline; see
  `.claude/memory-kit/2026-05-18/transitive-dupes-baseline.md`)

## Build timing

| Measurement | Value | Notes |
|---|---|---|
| Cold build (release, --workspace) | **25.6s** (real) | Empty CARGO_TARGET_DIR; no sccache; Nexus proxy had crates cached from prior builds |
| Warm rebuild (release, no changes) | **0.2s** (real) | Cargo's own fingerprint cache; no sccache |
| Build artifact size | **860 MB** | `du -sh` of CARGO_TARGET_DIR after release build |
| cargo metadata package count | **375** | Workspace + full transitive resolution graph |

**Note on cold build speed:** The 25.6s cold wall-clock is unexpectedly fast because the
Nexus crates.io proxy (T4) already had all crates cached in its local Maven store from
prior builds in this dev container. A true cold build on a fresh Nexus instance would
be slower (dominated by download latency). The figure here reflects compile time only —
the meaningful "download acceleration" benefit of the proxy shows up as reduced variance
across cluster peers, not as reduced wall-clock on a warm-Nexus machine.

## sccache state

sccache is reachable (v0.15.0, S3 backend: `sccache-elohim`) but showed **0 compile
requests** during this measurement session — consistent with the measurement design
(`RUSTC_WRAPPER=""` excluded sccache from the build-timing runs). The sccache null-byte
symptom on cargo-wrapped invocations remains unisolated (see open follow-ups below).

```
Compile requests                      0
Compile requests executed             0
Cache hits                            0
Cache misses                          0
Cache hits rate                       -
Cache location                  s3, name: sccache-elohim, prefix: /
Version (client)                0.15.0
```

## PVC / disk pressure (cargo-pool snapshot)

```
Pool root:    /projects/.cargo-target-pool
Pool size:    5.0G
Volume:       80G used / 38G free / 118G total (68% used, status=ok)

FAMILY               DISK       SLOTS      LAST_TOUCHED
dev                  851.9M     1          -
elohim               4.1G       2          -

Legacy target/ dirs outside pool (native, recoverable): 59.0G
  native        59.0G  /projects/elohim/elohim/elohim-storage/target
  unknown        3.3G  /projects/elohim/elohim/holochain/target
  unknown        3.8G  /projects/elohim/elohim/target
```

**Status: ok (68% used).** Legacy native targets at 59 GB are reclaimable via
`cargo-pool legacy-targets --clean --yes` if pressure rises above 85%.

## Open follow-ups (informational, not in this plan)

- **T9** (publish elohim-epr) deferred on Nexus auth config — Cargo publish requires
  HTTP Basic-auth but Nexus group repo presents itself as a passthrough; the hosted
  repo endpoint needs per-package write credentials. See
  `.claude/memory/feedback_nexus_cargo_publish_basic_auth.md`.
- **T10** (switch consumer to registry dep) blocked behind T9.
- **sccache null-byte symptom** on cargo-wrapped invocations remains unisolated — see
  `.claude/memory/feedback_sccache_cache_corruption_recovery.md`.
- **5 upstream-release monitoring triggers** documented in
  `.claude/memory-kit/2026-05-18/transitive-dupes-baseline.md` (deno_core/ed25519-dalek
  ecosystem splits; re-check when upstreams align on a single RustCrypto major).

## What changed vs the pre-sprint state

| Dimension | Pre-sprint | Post-T12 (this baseline) |
|---|---|---|
| Nexus crates.io proxy | Not wired | Wired (`[source.crates-io]` in `.cargo/config.toml`) |
| `[workspace.dependencies]` entries | ~30 | 34 (added `tokio-test`, `wiremock`, elohim-compute deps) |
| Straggler workspace members | 5 | 0 |
| Transitive dupe count | (no pre-baseline) | 19 unique, 0 actionable (established here) |
| Build wall-clock impact | (baseline) | Proxy + workspace-deps changes don't directly reduce wall-clock; real reductions come from Plan 2 structural refactor (gated on this baseline) |

## How to re-measure

```bash
cd /projects/elohim/elohim

# Cold build
rm -rf /projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev
time RUSTFLAGS="" RUSTC_WRAPPER="" \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo build --workspace --release

# Warm rebuild (run immediately after, no changes)
time RUSTFLAGS="" RUSTC_WRAPPER="" \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev \
  cargo build --workspace --release

# Dependency graph size
RUSTFLAGS="" RUSTC_WRAPPER="" cargo metadata --format-version 1 2>/dev/null | jq '.packages | length'

# Artifact size
du -sh /projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev
```
