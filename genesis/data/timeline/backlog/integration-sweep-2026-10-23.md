---
id: "backlog-integration-sweep-2026-10-23"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Integration sweep 2026-10-23: the main workspace's stranded work landed on dev or published under refs/held, with a verdict for every branch"
slug: "integration-sweep-2026-10-23"
written: "2026-10-23"
author: "2026-10-23 integration sweep (steward of record: human:matthew)"
status: "backlog"
priority: "high"
jobs: []
relatedNodeIds:
  - "habit:dataplane-convergence"
cites:
  - genesis/data/timeline/backlog/stranded-branches-after-2026-10-18-power-loss.md
  - genesis/data/timeline/backlog/hosted-provisioning-stop-damage-held-for-operator.md
tags: [integration, branches, triage, operator-decision, workspace-coordination]
---

# Integration sweep, 2026-10-23

The main workspace had accumulated work that never reached dev: a dirty main tree, 28 branches
with patches dev lacked, and about 50 worktrees. The steward directed that everything except
Codex's in-progress work be evaluated, landed on dev and cleaned up, with CI/CD suppressed
(`[skip ci]`) and the local pre-push gate bypassed. These pushes did not run the full
24-project pre-push selection; the gates named below are the only ones run.

## Landed on dev

| what | commit | gates run |
|---|---|---|
| Rust style guide, RS-GOV-04 checker, hook, policy, habit, a2o story, plus the main tree's other pending work | `41642c01a` | rust-style, orchestrator, placement, eprfs |
| bridge gates watch the rust-style crate epr-cli now links | `092924525` | orchestrator |
| canon-lift projection carries `rust-style-strict` | `9e1b14d6b` | eprfs `canon_lift_parity` |
| `infra/devfile-shem`: the shem workspace devfile | merge on the sweep branch | none (a devfile) |
| `perf/call-floor-2` `a9c9d764c`: persona walks, click-probe, ux-walk skill | cherry-pick | none |
| `feat/brit-daily-driver-epr`: `epr integrate`, standalone-root gate; plus RS-GOV-04 paydown in `flow/governor.rs` | cherry-pick | eprfs |

The a2o blind-reader floor was varied for `genesis/a2o/features/devflow/rust-style.feature`
under covenant; one fresh read is still owed.

## Published, not merged: fetch with `git fetch origin 'refs/held/*:refs/held/*'`

`refs/held/*` is outside the branch namespace, so Jenkins does not index or build it.

| ref | why it is held |
|---|---|
| `refs/held/land/hosted-provisioning-1019` (contains `fix/hosted-provisioning-stop-damage` and `proof/recovery-gate`) | Once deployed, doorways read an empty intent log as `wiped` and refuse registrations until `POST /admin/provisioning/recover` runs on each alpha doorway. A `[build:edge]` push from any workspace would deploy it. Conflicts in `provisioner.rs` and `typed_admin.rs`. |
| `refs/held/fix/provide-revoke-arm` | Reviewed unsafe twice; parked by ruling `617874935` until a renewed commitment's identity is decided. |
| `refs/held/rescue/power-loss-2026-10-16-main-tree` | 183 files dev lacks entirely (epr-cli views, portable app lifecycle in storage-client-ts and schemas, local-view listeners that need a security review, `bridges/browser`, `bridges/workspace`, the native seam-guidance module and its package, the absolutes guard). The other 445 files are older copies of files dev has since changed. It needs a per-cluster landing, not a merge. |
| `refs/held/feat/chrome-trust-edge` (`47ba8da76`) | Content-chrome layer 2, 191 files that were uncommitted in `wt-sw-preload`. Touches a DNA zome and the doorway CSP. |
| `ci/edge-gate-only` (PR #215) | Conflicts in the orchestrator and edge Jenkinsfiles that the native-worker CI work has since rewritten; resolving it untested risks every pipeline. |

## Superseded: dev already holds the change in edited form

The rest of `perf/call-floor-2`, `chore/vite-8-alignment` (dev is on Vite 8),
`worktree-agent-a37d55a795b144f1d`, `land/inv-conv-note` (its package rides the rescue
cluster), `ci/storage-probe-count` (closed), and `wt-habits-proj`.

## Left alone: Codex in progress

`feat/autonomous-delivery-20261022` and its landing and run-b worktrees,
`review/autonomous-{capture,privilege}-integration-20261022`,
`fix/autonomous-{gate,capture}-prerequisite-20261022`, `feat/autonomous-attribution-20261022`,
the `feat/habit-progress-*` family, `feat/jenkins-native-bdd-land`, and
`codex/placement-interfaces-20261021`.
