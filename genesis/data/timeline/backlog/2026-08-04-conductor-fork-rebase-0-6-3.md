---
id: "backlog-conductor-fork-rebase-0-6-3"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Rebase the holochain-conductor fork 0.6.0 → upstream 0.6.3 keeping tx5 compiled — carry dd12826, drop f85c2a7, reapply jemalloc pair"
slug: "conductor-fork-rebase-0-6-3"
written: "2026-08-04"
author: "holochain-iroh convergence campaign (Wave 1 Lane B)"
status: "resolved-pending-operator-push"
priority: "high"
tags: [conductor, holochain-fork, rebase, kitsune2, tx5, wave-1, codex-claimable]
cites:
  - genesis/docs/superpowers/plans/2026-08-04-holochain-iroh-convergence-upgrade-campaign.md
  - genesis/docs/content/elohim-protocol/history/2026-06-17-conductor-leak-upstream-research-tx5-pin-verdict.md
---

# Conductor fork rebase 0.6.0 → 0.6.3 (Wave 1, Lane B — claimable by any agent)

Task B1 of the convergence campaign plan (see cites — the plan's Global Constraints
section governs). Write-set: `elohim/holochain-conductor/**` (git submodule) only —
disjoint from Lanes A (elohim-storage), C (doorway), D (read-only).

## Context you need (no session context assumed)

- Submodule at `/projects/elohim/elohim/holochain-conductor`; remotes: `origin`=ethosengine/holochain, `upstream`=holochain/holochain. Current branch `elohim-0.6`, HEAD `dd12826`, based on upstream tag `holochain-0.6.0` (= commit `a6d4e80`).
- Rebase target: upstream tag `holochain-0.6.3` (= commit `448a36ef`, 2026-07-15).
- Our 5 fork commits, oldest→newest, and their expected rebase behavior:
  - `f85c2a7` — cherry-pick of upstream `6923effd` (validation-receipt fix, landed upstream in 0.6.1). MUST drop out empty; if it conflicts, verify upstream content already contains the fix, then `git rebase --skip`.
  - `7cc927e` — `[patch.crates-io]` pinning the tx5 0.8.1 family to `ethosengine/tx5` branch `elohim-0.8.1-zombie-fix`. Keep; if 0.6.3 moved the tx5 family past 0.8.1, STOP and escalate (do not guess a new pin).
  - `d0f505f` + `b477ca7` — jemalloc `jemalloc-prof` / `jemalloc` features (`crates/holochain/Cargo.toml`, `main.rs`). Additive; expected clean.
  - `dd12826` — change-check in `store_slice_hash` (`crates/holochain_p2p/src/op_store.rs`) + test. Confirmed 2026-08-04 the upstream function is byte-identical at 0.6.3 → clean reapply expected. This patch is load-bearing (adam write-lock-starvation incident 2026-07-20) and MUST survive.
- 0.6.1 made iroh the DEFAULT conductor transport. THIS TASK DOES NOT FLIP TRANSPORT: ensure the tx5 feature (0.6.3 name: check `crates/holochain_p2p/Cargo.toml` at the tag, expected `transport-tx5-backend-go-pion`) stays compiled and our runtime config stays tx5-shaped. The iroh flip is Wave 2, operator-gated.

## Steps

1. `git fetch upstream --tags` (if the network is blocked in your environment, stop and report — do not vendor tarballs).
2. `git checkout -b elohim-0.6.3 elohim-0.6 && git rebase --onto holochain-0.6.3 a6d4e80`.
3. Verify `git log --oneline holochain-0.6.3..HEAD` shows exactly 4 commits (f85c2a7 gone).
4. Conductor-config delta check (report only, monorepo template edits belong to the orchestrator review): diff config structs 0.6.0→0.6.3 for the new required `incoming_request_concurrency_limit` and removed `dpki:` section; record the upstream default value with file:line.
5. Gate (native build, `RUSTFLAGS=""`, set `CARGO_TARGET_DIR` per the session preflight pool-slot listing — never bare `target/`):
   - `cargo build --release 2>&1 | tail -5; echo EXIT=$?`
   - `cargo test -p holochain_p2p 2>&1 | tail -20; echo EXIT=$?` (dd12826's test lives here)

## DoD

4-commit branch `elohim-0.6.3` in the submodule, both gates EXIT=0 with output pasted, the config-delta report written, and NO monorepo gitlink change (the orchestrating session bumps the gitlink after composition review). Commit-only; never push to the monorepo.

## Outcome (2026-08-04)

Rebase completed on submodule branch `elohim-0.6.3` @ `da823fc6a` (4 commits, `f85c2a7` dropped correctly). Review caught 2 composition breaks, fixed in monorepo `4b163f707` + che `97916b6` (tx5 feature rename to `transport-tx5-backend-go-pion` + `--no-default-features`; required `relay_url` + denied unknown `NetworkConfig` keys). Gates evidenced green (release build, `holochain_p2p` 51/0). Remaining: operator pushes the branch, flips the che Jenkins params (same edit already staged locally), then bumps the gitlink.
