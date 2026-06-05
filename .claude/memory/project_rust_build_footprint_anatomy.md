---
name: rust-build-footprint-anatomy
description: "Verified anatomy of the 321G cargo pool (2026-06-04 ultracode research) — 71% is DWARF-laden ~1GB test binaries; retention policy, not Rust, is the dominant cause; ranked levers for maintainers and framework consumers"
metadata: 
  node_type: memory
  type: project
  originSessionId: 29e291fd-09c2-42e4-b132-bffd8a584b73
---

2026-06-04 measured + adversarially verified (workflow wf_fbed4ec7, 18 claims re-measured, readelf-confirmed):

**Anatomy of the 321G pool:**
- 71% (228G, 367 files) = ~1GB ELF test/bin executables of elohim-storage, each ~79% DWARF (verified: 925MB bin = 731MB `.debug_*` vs 91MB `.text`). NOT rlibs, NOT incremental (21G), NOT node_modules (1.3G).
- Multiplication, overlapping axes: storage workspace compiled in 5 families (251G in 3 dev slots = 78% of pool) × stale superseded build hashes within each slot (dev 13 / elohim 8 / sprint 6 / shift 3 one-GB binaries, never GC'd). One clean storage build ≈ 15-20G → 321G ≈ 19× policy amplification.
- **Key reframe (critic-verified): only ONE live git worktree existed, yet 6 pool families persisted.** The dominant multiplier is the pool steward's retention policy (never evicts merged families), not workspace design or "Rust is big."
- NO native crate defines any `[profile.*]` → cargo default `debug=2` full DWARF everywhere. CARGO_INCREMENTAL unset (default on). sccache already active (Garage S3) — saves time, not disk.

**Why:** Disk-pressure decisions and framework-packaging strategy were being made from folklore ("worktree targets and node_modules are the usual offenders" — measured false: 0G and 1.3G).

**How to apply:**
1. Immediate pressure → eviction outranks everything: auto-evict merged families + stale-hash GC (sprint prune reclaimed 92.9G same day).
2. Durable per-slot fix → **LANDED 2026-06-04** in root `.cargo/config.toml` (single point — config.toml reaches all native workspaces+worktrees; the standalone crates can't share a workspace profile due to RUSTFLAGS divergence + links=sqlite3 conflict). **VALIDATED** on doorway-service cold `--all-targets`: debug=2 → 9.50G; line-tables-only → 5.44G (−43%, builds faster); +deps debug=false → 4.07G (**−57%**, the committed config). The earlier 60-75% estimate was optimistic. Temporary full-debug escape: `CARGO_PROFILE_DEV_DEBUG=2`.
3. Pool is ext4: cross-family same-hash artifacts are true duplicate inodes; cargo artifacts are write-once → a guarded `jdupes -L` hardlink pass is safe and orthogonal to all other levers.
4. Structural: consolidate integration-test targets (each tests/*.rs file = one statically-linked ~1GB binary).
5. Framework consumers: ship compiled .dna/.happ + conductor/doorway binaries (holonix model) = framework disk ≈ 0; ship lean profiles in templates (Bevy model). A zome-only consumer never builds the native storage test suite — but their WASM path (in-tree holochain target = 14G) is UNPROFILED; open item.
6. Cargo offers no help: 1.88 GC covers CARGO_HOME only; target-dir GC unimplemented (rust-lang/cargo#13136). ccache's bounded 5GiB LRU is the C-world analog cargo lacks — eviction must be our tooling.

Open items: ~387G of the 736G volume sits outside /projects (unaccounted — second front?); line-tables-only reclaim unvalidated; WASM consumer path unprofiled.

Related: [[devspace-disk-cleanup-procedure]], [[cargo-target-dir-for-native-builds]], [[multi-agent-pvc-pacing]]
