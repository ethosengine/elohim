---
id: "backlog-household-keeps-no-copy-of-its-installed-bundle"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The household records which cells it installed but keeps no copy of the bundle, so a cold resume after a restart cannot reproduce them"
slug: "household-keeps-no-copy-of-its-installed-bundle"
written: "2026-10-31"
author: "claude-opus-5-5"
status: "open"
priority: "high"
area: "household mesh / hc-mesh.sh / restart recovery"
domain: "D5"
tags: [household, mesh, power-loss, restart, happ, receipt]
---

# The household keeps no copy of its installed bundle

After the 2026-10-30 power loss the Dowell household was fully stopped. `just mesh start`
refused `hApp bundle identity`: the bundle path it defaults to (the worktree's
`elohim/holochain/dna/elohim/workdir/elohim.happ`) was absent, and no bundle on the device
reproduced `installed-cells.json` (written 2026-10-29T23:25Z):

| Bundle tried | Result |
|---|---|
| main checkout `workdir/elohim.happ` (built 10-21) | lamad differs |
| `genesis/local-dev/jenkins-native-bdd/*.happ` (10-21, 10-22) | lamad, or all three, differ |
| rebuilt in `.worktrees/big-train` with each DNA's `just build` | imagodei matches; lamad and infrastructure differ (infrastructure's source unchanged since before the install) |
| Jenkins `elohim-holochain/dev` #1520 artifact (CI flags) | all three differ |

The bundle the household was installed from was most likely under `/tmp`, which a Che restart
erases. The steward ruled `MESH_ALLOW_BUNDLE_SKEW=1` for train 2's receipt, so the household
resumed on its old cells with a bundle on disk that does not match them.

| Row | What | State |
|---|---|---|
| 1 | Whenever the mesh installs cells, copy the exact bundle into `<MESH_DIR>/installed-bundle.happ` beside `installed-cells.json`, and default a resume's `MESH_HAPP_PATH` to that copy when the worktree bundle does not reproduce the cells | open |
| 2 | The refusal line names that copy as its first cure, ahead of `MESH_RESET=1` | open |
| 3 | Note in CLAUDE.md's persistent-workspace section that a household's installed bundle must not live only under `/tmp` | open |

## Done when

- A household stopped by a restart resumes with `just mesh start` and no bundle override.
