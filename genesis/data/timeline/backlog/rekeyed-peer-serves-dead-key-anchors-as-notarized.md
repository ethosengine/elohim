---
id: "backlog-rekeyed-peer-serves-dead-key-anchors-as-notarized"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "After a conductor re-key, both the re-keyed peer and its neighbours keep serving the dead incarnation's anchors as trust:notarized — neither re-adoption under the new key nor honest absence"
slug: "rekeyed-peer-serves-dead-key-anchors-as-notarized"
written: "2026-08-21"
author: "claude (staged the alpha conductor-spin class on the local mesh; the spin did not reproduce, this did)"
status: "refined"
priority: "high"
jobs: [elohim-edge]
nodes: [elohim-matthew-alpha, elohim-jessica-alpha, elohim-james-alpha]
relatedNodeIds:
  - "memory:project_ghost_declaration_deadlock_batch3"
  - "memory:feedback_reach_head_replication_distinct_planes"
  - "memory:project_attribution_cut_binding_proof_status"
tags: [conductor, holochain, re-key, provenance, trust, projection, ghost-actions, storage, honesty]
cites:
  - elohim/elohim-storage/src/services/content_service.rs
  - genesis/data/timeline/backlog/alpha-conductor-sys-validation-spin-unfetchable-deps.md
  - genesis/a2o/features/resilience/conductor-validation-spin.feature
---

# A re-keyed peer keeps calling dead-key anchors "notarized" (2026-08-21)

## What was done

On the local household mesh, james authored 5 content nodes on his own storage
(`POST /db/content` at :8092). Each landed NULL-anchored and was then re-authored
through **james's own conductor** by `reanchor_backfill`, reaching
`dhtAnchorHash: uhCkk…`, `trust: notarized`, 5/5 within 45s. Matthew and jessica
each came to hold 4/5 of them under those same anchors.

James was then re-keyed: conductor + storage stopped by exact pid, chain and
keystore deleted, happ re-installed on the kept wasm-cache, new agent key minted
(`uhCAkOZ2b_1Ie8WUDo4v…` → `uhCAkuV2p0tSQm-QoqoR…`), storage restarted on the new
key. **His storage database was kept on purpose** — that asymmetry is the alpha
shape: projection rows describing content the machine still holds, signed by a
chain that no longer exists.

Transcript: `/tmp/elohim-local-mesh/chaos-rekey/act1.rekey.log`, measurement
`/tmp/elohim-local-mesh/chaos-rekey/act1.measure.log`, summaries
`/tmp/elohim-local-mesh/spin/act1-{pre-rekey,measure}.json`. Staged with
`app/elohim-app/scripts/hc-mesh-chaos-rekey.sh --peer james --tag act1`.

## The defect

After the re-key, **all 5 nodes return HTTP 200 from BOTH james (:8092) and
matthew (:8090), carrying james's OLD anchors, still labelled `trust: notarized`**:

| id | anchor served after the re-key | trust |
|---|---|---|
| chaos-rekey-act1-01 | `uhCkkDPLSsQyl7TDAiFAWoMmeZKTqXB5XPmOKyYOV-fDfRqDWuYGp` | notarized |
| chaos-rekey-act1-02 | `uhCkkrdOrf2bxrVNbIq4rGOsAMw95b3C_owLguUWZkd5ZcNmLbvzd` | notarized |
| chaos-rekey-act1-03 | `uhCkkfSscIPEj4TrhrZ7R3Dj26BIicl4xdj70OkvBmS-FwyZrVx-p` | notarized |
| chaos-rekey-act1-04 | `uhCkkQZwjUtYp5QimBkeGMaSbcDx1OIlh7-AorOJeRft-ruvJj2yR` | notarized |
| chaos-rekey-act1-05 | `uhCkk95KC2WFvR2jzk5hxFZPKYBSVIVgIOFkW5E6vqALD7upF4uTo` | notarized |

Those action hashes were authored by an agent key that no longer exists on any
chain. Nothing on the network can produce the actions behind them. `notarized` is
the strongest provenance claim this system makes, and here it is being made
about a signature no living chain can present.

A re-keyed peer owes one of two honest answers. Either **re-adoption** — "I have
this again, under my new key", which means a new anchor authored by the new
agent — or **honest absence** — "I cannot prove this any more", a 404/410 or a
downgraded trust tier. This is neither. It is the confident-wrong third answer,
and it is worse than the spin that was being staged when it surfaced, because a
spin is at least loud.

## The storage plane half-notices, and cannot act

It is not that nothing detects the divergence:

- james's `/p2p/status` reported `projectionReconcile.divergentAnchor: 1` and
  `converged: false` during the measurement window, recovering to `0` / `true`
  by the end.
- matthew held `divergentAnchor: 1` across the whole window; jessica went 1 → 0.
- `reanchor_backfill` logged **"no NULL-anchor content (nothing to heal)"**.

That last line is the mechanism. The heal path is keyed on a NULL anchor, and
these rows are not NULL — they hold a *dead* anchor. A dead anchor is
indistinguishable from a live one by the only test the backfill applies
(present/absent), so the rows are skipped as healthy forever. The divergence
counter sees something; the repair path is looking for a different shape.

## What to decide

1. **Liveness must be part of the anchor test.** `reanchor_backfill` selects on
   NULL. It needs a second class: an anchor whose author is not a living agent on
   this conductor, or whose action the conductor cannot resolve. That class should
   re-author under the current key (re-adoption) rather than be skipped.
2. **`trust` must be able to say "I cannot prove this".** Today the tier is
   written at projection time and never revisited. A row whose anchor cannot be
   resolved should degrade out of `notarized` — the honest-absence answer — rather
   than keep asserting the strongest claim available.
3. **Decide what neighbours owe.** Matthew served the dead anchor too. Whether a
   peer re-serves another peer's now-unprovable anchor, or drops to its own
   independent evidence, is a governance call about what a household member may
   assert on another's behalf, not only a bug fix.
4. **Note the relationship to the spin.** This is the same fossil population as
   `alpha-conductor-sys-validation-spin-unfetchable-deps` viewed from the storage
   plane instead of the conductor plane. A cure for the conductor's retry loop
   (bounded backoff + abandon) does not touch this: the conductor would stop
   asking, and storage would go on calling the fossil notarized, quietly.

## How to reproduce

```bash
bash app/elohim-app/scripts/hc-mesh-chaos-rekey.sh --peer james --tag <tag>
# then, for each authored id, on BOTH :8092 and :8090:
curl -s -H "Authorization: Bearer mesh-admin-dev-key" \
  http://localhost:8092/db/content/<id> | jq '{dhtAnchorHash, trust}'
```

The finish line: every id answers with either an anchor authored by the peer's
CURRENT agent key, or a status/trust tier that admits it cannot be proven.

## 2026-08-22 (wave3 triage): the fossil population also survives the boot-time membership reconcile, and sits in custody rows

Two more storage-plane faces of the same dead-key residue, measured on the household mesh
after the 2026-08-22 01:19 james re-key (`chaos-rekey a2o-70809`):

1. **`humans.agent_pub_key` fossil not converged by the boot pass.**
   `features/dataplane/resilience-identity-coherence.feature` "No household-member human on
   alpha-A carries a fossil agentPubKey" red: `human-james-son` on matthew's peer still
   carried the PRE-re-key key `uhCAkkAYh…` (live key `uhCAkXieZ…`). Matthew's storage
   booted 03:44:57 — AFTER the re-key — so the boot-time membership-truth key-supersede
   pass (`services/membership_identity_reconcile.rs`) had its window on exactly the
   "unambiguous lone resolvable fossil" case it exists for, and left it. Either the pass's
   fossil test has the same present/absent blindness as `reanchor_backfill` (a dead key is
   not NULL), or the membership truth it reconciles against itself still names the dead key.

2. **Custody-blob commitments accumulate fossil providers.** The `chaos-ladder` blob's
   custody rows on matthew list FOUR providers: jessica + james-old (`uhCAkkAYh…`, authored
   00:54 pre-re-key) + james-new (`uhCAkXieZ…`, re-authored 02:19) + matthew. The dead
   incarnation stays on the provider record indefinitely — a recovery drill consulting
   custody credits sees a promise from an agent that no longer exists. Same
   decide-point as §"What to decide" item 1: provider liveness needs to be part of the
   custody-credit read (or a supersede pass must re-home the row), not only the anchor read.
