---
index: false
name: project_cargo_disk_guard_override
title: Cargo disk-guard override
description: "At the 85% disk hard-ceiling the PreToolUse hook DENIES heavy cargo; FORCE_HEAVY_GATES does not bypass it — free non-pool space or bump volume_hard_pct."
metadata: 
  node_type: memory
  type: project
  originSessionId: e8b573fe-62bd-4248-bb8b-d7d407266dca
---

`.claude/hooks/cargo-disk-guard.py` DENIES heavy cargo (`build`/`test`/`clippy`/…) when `/projects` df ≥ `volume_hard_pct` (85%, from `genesis/agentic/pool-policy.json`). **`FORCE_HEAVY_GATES=1` does NOT override this** — that env var only overrides the *pre-push* gate deferral, not the PreToolUse deny (the hook never reads it; verified 2026-06-19). The hook also has no other env bypass.

**The pool is tiny (~442M); the 729G/911G that pegs the volume at 85% is non-pool data**, so `cargo-pool enforce --yes` reclaims almost nothing (it only freed a 1.1G stray in-tree `target/`). The real unblock levers, in order:
1. Free multi-GB of **non-pool** volume space (operator-owned — other worktrees/data/projects on the shared PVC).
2. Bump `volume_hard_pct` in `pool-policy.json` (operator decision) — estimate the build vs the **real PVC cap** first (`cargo-pool estimate`; a single elohim-storage test build is ~6G observed, ~10-15G cold; the "~27G" estimate includes doorway/steward cold fallbacks you aren't building). Set the ceiling to clear current usage but hard-stop well below the 911G physical cap, build, then **revert**.

Two build traps that recur together on this stack (already in repo memory): the `/projects` pool slot throws **fingerprint ENOENT** (`failed to write …/.fingerprint/…/invoked.timestamp: No such file or directory`) on cold builds → use a **`/tmp` target dir**; and **sccache** (`RUSTC_WRAPPER=sccache`) intermittently returns **null bytes** on the clippy-driver `--print` rustc-stdin probe (`unknown start of token: \u{0}`) → re-run with `RUSTC_WRAPPER=""`. Native cargo here needs `RUSTFLAGS=""` (the ambient WASM getrandom flag breaks linking).
