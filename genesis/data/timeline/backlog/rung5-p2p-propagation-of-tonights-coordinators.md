---
id: "backlog-rung5-p2p-propagation-of-tonights-coordinators"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Next shift Objective — propagate tonight's coordinator changes (content_store compute_task + correction externs; mishpat signed-author links) to the alpha canary over p2p (rung 5) from the workspace T3 peer, single-role candidates cut FOR james, and read the attestation back — not attempted 2026-09-07 because the household mesh held ports 8090/8888 all night and the budget closed"
slug: "rung5-p2p-propagation-of-tonights-coordinators"
written: "2026-09-07"
author: "overnight shift 2026-09-07"
status: "open"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:runtime-upgrade-propagation"
tags: [rung-5, p2p, release, coordinator, next-shift]
---

**Why it was not done tonight:** the mandate asked to "propagate an update over p2p native"; the recipe exists (memory `project_workspace_to_fleet_first_crossing_2026_09_06`: `just mesh stop`, T3 peer via `hc-start.sh` join-alpha with the fork bin, single-role candidate via `coordinator-candidate.ts --role lamad`, `--applies-to-from-adoption <doorway>/db/p2p/adoption?peer=james`, publish, canary applies ~60 s, attestation +86 s). The household mesh was needed until 10:5xZ for the correction stations, the compute authority leg and the fixtures-clone legs, and the T3 peer contends for 8090/8888; the edge deploy from tonight's push was also rolling (never run the fleet WRITE while an edge deploy rolls). **Shape for the next shift:** after the edge deploy quiesces, cut a lamad single-role candidate from dev, publish from the T3 peer, confirm james applies by election (`coordinator hot-swap applied … drifted=1 applied=1`), then a rollback-shaped release to bring james back to baseline; receipts under `genesis/a2o/reports/workspace-release/2026-09-07/`. Real delta available: the coordinator bytes from tonight are already on the fleet via the pipeline, so use `--marker` or wait for the next coordinator change.
