---
id: "backlog-stranded-branches-after-2026-10-18-power-loss"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Stranded branches after shem's 2026-10-18 power loss: four landed, one held for the operator, and seven still waiting on a landing, a ruling or their owner"
slug: "stranded-branches-after-2026-10-18-power-loss"
written: "2026-10-19"
author: "2026-10-19 overnight shift (finish integration-2026-10-18)"
status: "backlog"
priority: "medium"
jobs: []
relatedNodeIds:
  - "habit:dataplane-convergence"
cites:
  - genesis/docs/superpowers/sprints/2026-10-18-marathon-retrospective.md
  - genesis/data/timeline/backlog/hosted-provisioning-stop-damage-held-for-operator.md
tags: [integration, branches, triage, operator-decision, power-loss]
---

# Stranded branches after the 2026-10-18 power loss

The 2026-10-18 integration pass compared every stranded branch and worktree with dev and gave each a verdict. Its write-up is the marathon retrospective's "Integration pass" section. This entry tracks what is still open after the 2026-10-19 overnight shift.

## Landed (household receipt on each one's own source)

- Wave 6 and the doorway operator authorization fix: `b1e42ad82` (2026-10-19 01:11Z).
- The retired-pin readmit backoff: `d37d467e4`.
- The SSR self-heal for abandoned lookups, plus the `c9f9f8ea0` SSR phase: `c41e075a8`.
- The plane-violation write guard: `7d0e3089e`.

## Held for an operator decision

- `fix/hosted-provisioning-stop-damage`: see [hosted-provisioning-stop-damage-held-for-operator](epr:hosted-provisioning-stop-damage-held-for-operator).

## Still open

| branch | verdict | next act | whose |
|---|---|---|---|
| `perf/call-floor-2` `a9c9d764c` (ux-walk / click-probe) | still needed | cherry-pick, run the gate, land | any shift |
| `feat/brit-daily-driver-epr` | still needed; 1 conflict (epr-cli `lib.rs`) | rebase, gate epr-cli, land | any shift |
| `fix/provide-revoke-arm` | parked by dev ruling 617874935 (the commitment's identity is undecided) | the ruling | operator |
| `ci/edge-gate-only` (PR #215) | open | merge, which rolls the fleet | operator |
| `ci/storage-probe-count` (PR #216) | its extras are not needed | close | operator |
| `infra/devfile-shem` | `devfile-shem.yaml` is absent on dev; a cluster operation | decide | operator |
| `rescue/power-loss-2026-10-16-main-tree` `0ee5dfde8` | about 150 unique files: the seam-guidance native module, epr-cli view/integrate, the portable app lifecycle, local views (new listeners, which need a security review), the bridges workspace, and the absolutes guard | owner review per cluster; keep the branch | owner |
| `wt-sw-preload` (`feat/chrome-trust-edge`) | 150 uncommitted edits: content-chrome layer 2, mid-flight; touches a DNA zome and the doorway CSP | owner review; do not clean | owner |

## How to land one

Use a fresh worktree under `.worktrees/`. Prepare it with:

- `git submodule update --init elohim/rakia`
- `pnpm install --frozen-lockfile`
- the workspace dists: storage-client and its dependents, `elohim-elements/*` and elohim-core

Give the worktree's gate its own `CARGO_TARGET_POOL_FAMILY`. When two gates shared one family, a cargo run that started mid-gate deleted the other gate's fingerprint directories, twice.

Before running any lane, ask `genesis/orchestrator/scripts/t2-receipt.sh` which household stories the diff's serving paths need. Run it against an empty reports directory and it names every one. Then push the explicit sha.
