---
name: Devspace disk-fill recovery procedure
description: Eclipse Che workspace /projects volume is 118G; fills from .angular/node_modules/target plus .claude/worktrees Rust target dirs; cleanup procedure recovered 104G on 2026-05-08
type: project
originSessionId: 49471c84-ee22-4391-ade3-cfcabbe03f0d
---
The Eclipse Che workspace `/projects` volume is 118G. When it hits 100%, the harness silently truncates `/projects/.claude-config/.claude.json` to 0 bytes (Claude refuses to start, "JSON Parse error: Unexpected EOF"). Backups in `/projects/.claude-config/backups/` rotate; restore the most recent.

**Why:** disk fills from two predictable sources:
1. `node_modules/`, `.angular/`, `target/` (Rust/Java) under `/projects/elohim` — recoverable build artifacts
2. `/projects/elohim/.claude/worktrees/agent-*/...` — agent-isolation worktrees, each with their own Rust `target/debug/` (especially `iroh-parallel-stack`, where the parallel-cargo-build pattern explodes disk in <1 hour)

On 2026-05-08 the workspace went from 100% → 12% by clearing both sources, recovering ~104G total (45G from main tree + 59G from worktrees).

**How to apply:** when disk is approaching 85% on /projects:
```bash
# 1. Main-tree build artifacts
find /projects/elohim -type d \( -name node_modules -o -name .angular -o -name target \) -prune -exec rm -rf {} +

# 2. Worktree target dirs (the bigger offender for shifts)
find /projects/elohim/.claude/worktrees -type d -name target -exec rm -rf {} +
```

If the harness already crashed with "Unexpected EOF": restore the most recent backup:
```bash
cp /projects/.claude-config/backups/$(ls -1t /projects/.claude-config/backups/.claude.json.* | head -1) /projects/.claude-config/.claude.json
# Validate before relying: jq empty /projects/.claude-config/.claude.json
```

A SessionStart+Stop hook at `/projects/.claude-config/hooks/disk-pressure.sh` warns at 85% and screams at 95%, with 30-min cooldown to avoid spam.
