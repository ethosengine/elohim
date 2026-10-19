---
id: "backlog-steward-key-chain-fork-warrant-exile"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A chain fork exiles an agent key forever — adam and eve on alpha, james on the household after a power loss; cause, observability, prevention and the way back, worked in order"
slug: "steward-key-chain-fork-warrant-exile"
written: "2026-10-10"
revised: "2026-10-19"
author: "steward-exile-probe shift, 2026-10-10 fleet read; revised by the 2026-10-18 integration pass (household case)"
status: "open"
priority: "high"
jobs: [elohim-edge]
relatedNodeIds:
  - "backlog-keyrotation-mint-path-witness-backed"
  - "backlog-dataplane-reanchor-dead-remaining-rekeyed-peer"
  - "backlog-rekeyed-peer-serves-dead-key-anchors-as-notarized"
  - "habit:dataplane-convergence"
cites:
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
  - genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md
  - elohim/holochain-conductor/docs/design/source_chain_restore.md
  - genesis/data/timeline/backlog/keyrotation-mint-path-witness-backed.md
  - genesis/a2o/features/resilience/chain-fork-lost-tail.feature
tags: [fleet, household, conductor, warrant, chain-fork, kitsune2, dataplane-convergence, alpha, power-loss, durability, risk, decision, open-question, lesson]
shift_objective: |
  Work steward-key-chain-fork-warrant-exile in its stated phase order. Phase 0 needs no decision:
  read the fork's seq from the preserved household copy (C1a), and add the household exile
  detector plus a lane failure line that names exile (C2b). Bring the operator one decision (D1:
  re-key with lineage, or operator unblock) with the evidence in this item. Do not start C4 or C5
  before D1 is recorded. Done = each concern you touched shows its probe result and a dated line.
---

**The class.** A chain fork is two actions by one author at one sequence number. Holochain cannot
tell a forger from an honest node that lost its last write, so both get the same response. Every
authority that validates the `ChainFork` warrant blocks the key from now to `Timestamp::max()`
(`integrate_dht_ops_workflow.rs:104`). kitsune2 0.5.0 then refuses the blocked agent's info at the
peer store (`mem_peer_store.rs`), so no access decision ever exists for its url, and a url with no
decision counts as blocked (`core_space.rs is_any_agent_at_url_blocked`). No admin call lifts any
of this. Under this pin, a warranted key cannot converge by waiting or by restarting.

## Instances

**Alpha, 2026-10-10 (open).** Chain-fork warrants against adam's agent key `uhCAkCUzKl3…` and eve's
`uhCAkhsVVjkuw1D8…`, authored by jessica, james, gertrude, eve and matthew. Matthew discarded
4,285,797 of the 4,285,796 messages adam sent him. Adam's `hc_conductor_workflow_integrated_ops_total`
is 0. Loki's four-day retention holds six distinct fork pairs (adam four, eve two), new ones on three
consecutive days, and one each after the 2026-10-09 conductor clear. That pattern argues against a
one-time event. Evidence and queries: `dataplane-convergence` DELTA 2026-10-10a and 10-10b.

**Household, 2026-10-18 (a controlled case; ended by recast).**
- The conductors' last log line is `2026-10-18T00:37:37.755Z`, written under `database is locked`
  (517/5) contention. That moment is the power loss.
- The next line is the cold start at 22:11:51Z. Nothing ran in between.
- At 22:17:28–34Z, five `Authoring warrant for chain fork by uhCAkGAKYdJgX8pd…` lines name james's
  agent. They cover ONE action pair, `uhCkkDAHEc70LAuv…` and `uhCkk0Kwwp1cRnML…`.
- No warrant line exists anywhere earlier in the log, which goes back to 2026-10-06.
- The block was confined to one DNA: lamad (`uhC0kUQPSe…`).
  - Matthew's conductor dropped 221,069 of the 601,060 messages james sent. Jessica's dropped 221,042.
  - The other four DNAs blocked about 0.
  - `hc client call list-agents --dna <lamad>` on matthew and jessica showed no entry for james.
  - Matthew's peer meta showed 0 lamad gossip rounds with james, against 43–49 in every other DNA.
- Consequences:
  - `epr-app-deliverability` failed at "every household peer answers with the same server pointer"
    (`james=<200>`).
  - The pre-push serving receipt cannot pass in that state, and a landing (wave 6) was blocked.
  - james's storage looked like restart churn: projection-reconcile pending grew from 12 to 49 and
    `stuckSweeps` climbed from 9 to 35. A 30-minute settle wait changed nothing.
- On operator instruction, the household was recast (`MESH_RESET=1`) at 23:55Z.
- A copy of james's conductor databases (with `db.key`) and every conductor and storage log was
  preserved first, at `genesis/local-dev/integration-2026-10-18/forensics-james-fork/`. It is
  gitignored and lives on this workspace's PVC only.
- Campaign journal: that directory's `STATE.md`.

## Concerns, in the order that unblocks the most

**Replication drill (proof surface for every concern below):**
`genesis/a2o/features/resilience/chain-fork-lost-tail.feature`, tagged `@concern:chain-fork-recovery`.
- It runs on a fresh `just mesh join-peer` fixture, never on an incumbent.
- It reproduces the lost tail deterministically: snapshot the conductor databases, author and
  publish one action, restore the snapshot, then author again.
- Scenario 1 is the failure regression. It should pass on today's pin; that pass is the measured
  failure.
- The other scenarios are each concern's cure proof: C2b exile named, C2a warrant seq, C3b head
  check, C3a durability, and C4/C5 lineage re-key.
- All of it is `@wip` until the steps exist. A concern closes when its scenario passes on the
  household.

| # | Concern | Kind | Needs | State |
|---|---|---|---|---|
| C1a | Household fork cause | read | the preserved copy | consistent with WAL loss; seq unread |
| C1b | Alpha fork cause (recurring) | read | Loki ≤3 h windows, or C2a | unread |
| C2a | Fork seq + warrant surface in the conductor | conductor-fork code | next conductor roll | open |
| C2b | Household exile detector + lane line that names exile | storage / hc-mesh code | nothing | open |
| C3 | Prevention: no publish before durable; head check after an unclean stop | conductor config / fork code | C1a confirms; perf-io measure | open |
| D1 | The way back: re-key with lineage vs operator unblock | **operator decision** | this item's evidence | **undecided since 2026-10-10** |
| C4 | Recovery implementation | DNA + storage code | D1, KeyRotation mint path | blocked on D1 |
| C5 | Household re-key-one-peer arm (no full recast) | hc-mesh code | D1, C4 | blocked on D1 |
| C6 | Matthew's key on alpha | read | Loki | unverified |
| C7 | Return adam and eve on alpha; seam 7 → `--gate` | operator act | D1, C4 | blocked |

### C1a — household cause

- **Assertion:** the forked pair is james's last action before 00:37:37Z, recorded in his peers'
  copies and absent from his own chain, plus a different action at the same seq authored after
  22:11:51Z.
- **Probe:** open the preserved `dht-uhC0kUQPSe…db` with SQLCipher and its `db.key`. This is an
  offline copy, so no live conductor is involved. Read james's authored rows at the seq that holds
  either hash, with their timestamps.
  - Do not use `hc client call dump-state` for this. It closed the websocket even when paged.
- **Current state:** consistent with the mechanism but not proven. The conductor opens every
  database at `synchronous=Normal` in WAL mode: `DbSyncLevel::Normal` is the default in
  `holochain_data/src/lib.rs:69`, and no repo config sets `db_sync_level`. SQLite documents that a
  WAL commit at NORMAL may roll back after a power loss or OS crash. A process kill does not lose it.

### C1b — alpha cause

A pod OOM-kill is a process kill, which NORMAL survives. So if C1a confirms WAL loss, that does
**not** explain alpha's daily forks unless a node or host lost power. Keep the 10-10 candidates
open:
- a dataset restored behind its own published chain
- one batch of stale ops replayed by a third peer (the two pairs at 2026-10-09T23:52:10Z reached
  jessica 0.5 ms apart)

Read the forked peer's own log in the ten minutes before each first sighting, in windows of three
hours or less. A four-day query probably OOM-killed loki-0.

### C2a — make the fork observable (conductor fork)

`make_fork_warrant_op_inner` (`sys_validation_workflow.rs:1712`) receives `seq` but logs only the
hashes. Log the seq. Also expose integrated warrants (warrantee, kind, seq, when) through the admin
surface that storage already reads for `/db/p2p/conductor-diagnostics`. That turns C1a and C1b
from inference into a read. A pin move rolls the fleet, so ship this together with C3's fork part
in **one** intended conductor roll.

### C2b — name exile where it bites (no decision needed)

Assertion: a household lane or status read never presents an exiled peer as "slow". Today
`scripts/ci/substrate-seam-smoke.sh` seam 7 `peer-exile` covers the fleet only. Add the same read
to `just mesh status` and preflight: blocked incoming ÷ received per connection, per DNA, from
`/db/p2p/conductor-diagnostics`. Then make the deliverability pointer step's failure message name
an exiled peer instead of `james=<200>`. The household case cost a 30-minute settle wait that the
first diagnostics read would have prevented.

### C3 — prevention

Two layers, both needed:
1. **Durable before published.** Options:
   - Set `db_sync_level: Full` in the household and fleet conductor configs. This is a config-only
     change. Measure its perf-io cost on the household first.
   - Or a fork patch that syncs the authoring transaction before the publish workflow can read it.
   - Choose by measured cost. The assertion is the same either way: an action never reaches an
     authority before it is durable on its author's disk.
2. **Head check after an unclean stop.** If the conductor did not shut down cleanly, it compares its
   own chain head with the network's (`get_agent_activity_multi`, in the pin since a933aa040) before
   authoring anything. If the network is ahead, it re-imports the missing actions instead of
   re-authoring over them. Upstream's restore design (`docs/design/source_chain_restore.md`) states
   that its Step 2 writes are idempotent on a partially populated database, which is exactly this
   case. Build on that workflow rather than a parallel one.

### D1 — the way back (operator decision; record the ruling here)

- **Re-key with lineage.** The agent closes the forked chain and continues under a new key that a
  `KeyRotation` binds to the same identity head (`identity-head-key-lineage` design). The human
  keeps their identity, the old key stays warranted, and nothing is unblocked.
- **Operator unblock.** Clear the block on every authority that integrated the warrant. That is DB
  surgery per conductor, since no admin call exists, and it accepts a chain that the network has
  proven forked.
- **Evidence:** upstream's own model treats a locally validated `ChainFork` as terminal. Restore
  moves the app to `AppStatus::Unrecoverable(ChainFork)` and offers no unblock
  (`source_chain_restore.md`, failure table). Unblock would therefore diverge from upstream, and
  every authority would have to act in step.
- **Recommendation:** re-key with lineage. Until D1 is recorded, every future fork ends in a recast
  (household) or a permanent exile (fleet).

### C4 — recovery implementation (after D1)

If D1 picks re-key with lineage, the critical path is `keyrotation-mint-path-witness-backed`: no
coordinator function mints a valid witness-backed `KeyRotation` yet. A re-key also leaves storage
residue that must heal under the new key. Two existing items cover it:
- `dataplane-reanchor-dead-remaining-rekeyed-peer` (dead anchors that never clear)
- `rekeyed-peer-serves-dead-key-anchors-as-notarized`

Assertion: after a re-key, the peer converges in every DNA, and no row anchored to the dead key is
served as notarized.

### C5 — household arm (after C4)

`just mesh rekey <peer> [role]` re-keys one peer's cell through C4's path, preserving every other
identity. It refuses unless that peer is warranted. Assertion: a forked household peer recovers
without `MESH_RESET=1`. Receipt: deliverability passes on the same household after the arm.

### C6 / C7 — alpha

C6: re-read `Warrant op is valid`, grouped by warrantee, in a three-hour window. If matthew's key is
warranted, the node the quiesce gate reads is in this class. C7: once C4 exists, the operator
returns adam and eve under lineage keys; then flip seam 7 into `--gate`.

## Lessons (2026-10-18)

- When one household peer lags after a hard stop, read these in order before waiting:
  1. blocked incoming per DNA (`/db/p2p/conductor-diagnostics`)
  2. `list-agents --dna` on the other peers
  3. `grep -F 'Authoring warrant' conductors/.sandbox_run_log`
- The forensic copy must be taken **before** a recast. The recast destroys the only chain that can
  date the fork.
