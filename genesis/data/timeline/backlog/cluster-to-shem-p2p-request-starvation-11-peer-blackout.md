---
id: "backlog-cluster-to-shem-p2p-request-starvation-11-peer-blackout"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Cluster→shem P2P request starvation: 11 of 13 mesh peers connected-but-unanswering (sync+shard timeouts); pull=false and adam-class verdicts are downstream"
slug: "cluster-to-shem-p2p-request-starvation-11-peer-blackout"
written: "2026-07-02"
author: "shift-genesis-verdicts-green"
shift_objective: "genesis-verdicts-green (2026-07-02 overnight) — B/D classification: BLOCKED-BY-ENV, evidence captured here"
status: "backlog"
priority: "high"
requires_env: "shem"
themes: [dataplane, libp2p, shem, reachability, acquisition, observability]
relatedNodeIds:
  - "genesis/manifests/habits.yaml"
  - "genesis/data/timeline/backlog/wan-nat-federation-dataplane-discovery-gap-2026-06-23.md"
tags: [p2p, shem, blocked-by-env, habits:sync-scale-honesty]
---

# Cluster→shem P2P request starvation — the 11-peer blackout

Grounded overnight 2026-07-02 (shift `genesis-verdicts-green`, Loki + edge #1135
forensics by ci-investigator; window 2026-07-01T23:49→2026-07-02T11:49):

**Shape.** matthew completes P2P request-response with exactly 2 of its 13 mesh
peers — jessica and james, the only other on-prem in-cluster pods. Against the
remaining **11** (adam, terrance, frank, gertrude, susan, daniel, emma, eve,
nancy + the two never-Ready: pete, caleb) BOTH sync (`/elohim/storage-sync`) and
shard protocols fail with `error: Timeout`, outbound AND inbound, ~97–123
events/peer per 2h — a uniform roster-wide cycle, not a flaky subset. james
shows the identical 11-peer signature (rules out a matthew-local defect). Peers
stay swarm-CONNECTED (`/health peerCount=13`) — connections accept, request
handlers never answer: starvation, not partition.

**One family, one root-class.** All 11 are shem-scheduled ("every non-family
persona is pinned to remote" — 2026-05-27 placement directive, quoted from the
nancy manifest render in edge #1135). The same shem axis carries: pete+caleb
rollout timeouts (0 pods at revision, edge #1135), adam's conductor admin-WS
flapping (genesis #1231 probe ❌) and custody-convergence miss, and the
zero-Loki invisibility of every shem pod (promtail DaemonSet scrapes on-prem
nodes only — task #7; NOT a Loki misconfig). Recent relief attempt already on
dev: 97182d06b capped shem edgenodeCpuLimit 2000m→1000m.

**Downstream verdicts this explains (do not grind code loops on them):**
- `projection.matthew.streams — pull=false`: acquisition rollup total>0,
  fetched<total — stuck pins' providers are in the unreachable 11. Deterministic
  consequence, not an acquisition-logic bug.
- `propagation.custody-convergence missing on: adam` + `[probe] adam:4444`.
- Task #9's "timeout trio" (NguL/GPmV/Rnj3) = 3 representatives of the 11.

**Ceiling (operator):** shem node capacity/scheduling relief; promtail/Alloy
coverage of the shem node (restores observability for 11 peers at once);
whether request-handler starvation under CPU caps deserves a QoS floor
(protocol handlers vs conductor contention).

**Code-side candidates (post-relief, habit `sync-scale-honesty`):** per-peer
timeout backoff/circuit so 11×~50/h request storms don't burn cycles;
`/p2p/status` pull rollup should log its total/fetched at INFO on change (the
rollup is currently untraced — this investigation could not read it from logs);
peerId→identity mapping surfaced at deploy time (the 11 could not be named from
observability alone).

**Falsifier:** if a shem-side capacity relief lands and the 11-peer timeout
signature persists at unchanged rates, the starvation read is wrong — re-open
as a protocol-level defect (request-response handler wiring on shem builds).
