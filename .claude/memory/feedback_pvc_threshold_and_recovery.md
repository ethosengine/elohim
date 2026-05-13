---
name: PVC threshold — act at 85%+, cargo-pool legacy-targets is the first reclamation
description: Workspace PVC is 118G. Above 85% used, the operator considers the system "close to toast." First-party reclamation: cargo-pool legacy-targets --clean --yes (recovers ~30-35G of duplicate target/ dirs outside the shared pool — safe, build artifacts only).
type: feedback
originSessionId: 60007cbf-4a59-4bce-9be7-6e57d1568cf6
---
The workspace PVC is **118G**. The operator's threshold language: "you hit 118G and you close to toast." Above 85% used, take action — don't just observe.

**First-party reclamation (in priority order):**

1. **`cargo-pool legacy-targets --clean --yes`** — removes target/ dirs OUTSIDE the shared cargo-target-pool. These are duplicates from cargo runs that didn't redirect CARGO_TARGET_DIR. Typical recovery: 25-35G. Safe — only removes "native" classification (WASM/typesrc/unknown are kept). Cargo rebuilds on demand if needed.

2. **`cargo-pool prune --stale-incrementals --yes`** — GCs old incremental hash dirs inside the pool. Smaller recovery, lower risk.

3. **`cargo-pool node-modules --clean --yes`** — removes node_modules where lockfile is newer than install. Recovery varies.

4. **`cargo-pool prune family <name>`** (interactive) — nuclear option for a whole family. Don't use without confirmation; another sprint may own the family.

**Don't touch:**
- Other families' worktrees inside the pool unless coordinated. Concurrent sprints (e.g., the tiered-storage sprint co-existing with EPR delivery) may own families that look idle.
- `.angular` caches without checking `cargo-pool angular-cache` first.

**Pre-dispatch check before any cargo-running agent:**
- `df -h /projects | tail -1` — must show < 80% used.
- `bash genesis/agentic/bin/cargo-pool status | head -3` — confirm status=ok, not status=warn.
- If above 80%: clean BEFORE dispatching, not while the agent is running.

**Why first-party cleanup is operator-authorized in autonomous mode:**
The `cargo-pool` tool was authored explicitly for this scenario. Session-start hooks recommend the cleanup proactively when legacy-targets are detected. In overnight / auto-mode flows where the operator is unreachable, running the documented safe-cleanup is preferred over crashing the substrate.

**What to NEVER do without confirmation:**
- `rm -rf` anything by hand (use cargo-pool tools)
- Delete worktrees you didn't create (parallel sprints may own them)
- Touch the legacy-targets cleanup if it'd reclaim < 5G (not worth the action)
