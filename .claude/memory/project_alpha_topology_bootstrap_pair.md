---
id: project-alpha-topology-bootstrap-pair
name: Alpha cluster topology — 7 active peers, cast directive, bootstrap pair
title: Alpha cluster — 7 active peers
description: 7-peer alpha fabric (4 shem + 3 household) per the 2026-07-02 cast directive; adam+matthew bootstrap pair; deployments.json suspended flag IS the roster — check before debugging peers.
type: project
originSessionId: 872c2e1c-02fe-453a-93b3-e69dac1e54e3
cites:
  - genesis/orchestrator/data/deployments.json
modified: 2026-08-08T23:58:05.288Z
---
Alpha is a **7-active-peer** test fabric where the k8s node split represents household boundaries. The roster is governed by the `suspended` flag in `deployments.json` under the **operator scope directive 2026-07-02** (minimal coordination-ladder cast, recorded in every suspended human's `$suspendedComment`): one instance per tier — intimacy/family (matthew/jessica/james Dowell household), community/local (gertrude elder household + susan neighbor household), regional (on-prem region vs shem region {adam, gertrude, susan, eve}), global (adam federation anchor).

| Peer | k8s node | Archetype | Role |
|---|---|---|---|
| adam | shem | home-nuc | genesis peer / federation anchor; bulk seed receiver (paired w/ matthew) |
| eve | shem | home-nuc | second always-on hub steward (Eden Valley household) |
| gertrude | shem | home-nuc | elder household; reciprocal-recovery counterparty to Dowells |
| susan | shem | recycled-laptop | Seattle household (Matthew's sister); tri-region backup chain leg — **only recycled-laptop on shem** |
| matthew | ethosengine | family hub | other half of bootstrap pair; bulk seed receiver |
| jessica | (household node) | recycled-laptop | Dowell household |
| james | (household node) | chromebook-floor | Dowell household |

Suspended (replicas 0, stale image, still declared remote): pete, terrance, frank, caleb, daniel, emma, nancy — un-suspend when shem relief lands or an epic scenario `@requires` a wider cast.

**Bootstrap pair = adam + matthew across the node-split** — the two bulk-upload seeder targets. **Any "N peers" claim in docs/handoffs should be checked against the suspended flags, not folklore** — the 6-peer framing circulated for ~5 weeks (2026-07-02→2026-08-08) while susan was active the whole time; edge deploys had been reporting "7/7 peers Ready" correctly.

**Update 2026-06-03 — adam's genesis role is substrate-gated at CONSUMPTION, not a static flag.** adam is remote-only (pinned to shem). When shem is down, `genesis/Jenkinsfile runContentSeedStage` HOLDS adam from the genesis set and matthew carries ingest alone; `adam.genesisPeer:true` stays correct-when-shem-is-up because reconciliation happens where the value is READ. See [[project_substrate_scope_runtime_arm]], [[feedback_shem_down_peers_are_held_not_failed]].

**How to apply:** When debugging alpha symptoms ("where is this byte / why isn't this peer reachable"), consult this roster first — and remember "N/N peers Ready" in the edge deploy gate is a `:8090` liveness probe, NOT mesh participation (susan sat at zero kitsune2 gossip attempts while Ready, found 2026-08-08).
