---
id: "backlog-peer-driven-dna-repair-update-button"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Peer-driven DNA-lifecycle self-repair — the node reconciles its installed DNA to its assigned bundle, facilitated by a human steward's peer-menu 'Update / Repair' button (not an operator with kubectl)"
slug: "peer-driven-dna-repair-update-button"
written: "2026-06-22"
author: "operator design steer during the alpha CellWithoutGenesis incident (2026-06-22)"
status: "open"
priority: "high"
domain: "D-identity-sovereignty / peer-runtime"
tags: [peer-sovereignty, reconciliation-controller, dna-lifecycle, self-repair, self-healing, conductor, genesis, p2p-design-gate, hub-optional-floor, ui-affordance]
relatedNodeIds:
  - backlog-alpha-conductor-cellwithoutgenesis-floating-happ-tag
  - backlog-agent-peer-binding-cross-signed-proof
cites:
  - genesis/data/timeline/backlog/alpha-conductor-cellwithoutgenesis-floating-happ-tag.md
  - elohim/elohim-storage/src/conductor/process_manager.rs
  - elohim/elohim-storage/src/main.rs
---

# Peer-driven DNA self-repair — the node drives its own genesis recovery, a human steward pushes the button

## The principle (why this exists)
The alpha CellWithoutGenesis incident exposed an **anti-pattern**: recovering a node's
drifted/genesis-less DNA cell required the **operator + kubectl** (PVC wipes, the centralized
genesis pipeline force-reinstalling, env-flag flips). That treats the node as a dumb pod an
operator manages — the opposite of the protocol's peer-sovereign floor ([[project_hub_optional_floor]],
[[feedback_k8s_is_not_the_architecture]]). The **P1 reconciliation-controller** principle
([[project_principle_p1_reconciliation_controller]]) should own the conductor's **DNA lifecycle**,
not just data: a node told "run bundle X" drives its OWN path from "I have a genesis-less / drifted
cell" to "genesised on X" — per its own lineage policy — with **zero operator kubectl**.

## Three agency modes for the SAME repair primitive
The mechanism is one primitive — `ConductorManager::clear_conductor_state` (clear the conductor
data dir → clean boot → `install_fresh` → genesis against the assigned bundle). What differs is
*who triggers it*:

1. **operator + kubectl** — today's anti-pattern. Retire.
2. **the node, autonomously on boot, per its own policy** — **SHIPPED** (2026-06-22, the Fix 2
   revision): `main.rs` boot reconcile-retry loop wraps `start → wait_for_ready →
   ensure_happ_installed`; on a genuine conductor death (child exited) when the node's own
   `GENESIS_SELF_HEAL_IDENTITY` policy says it is re-seedable, it clears the data dir ONCE and
   re-genesises. Gated to the re-seedable policy because it RE-KEYS (the lair keystore is under the
   data dir).
3. **the node's human steward, via an "Update / Repair" button on their peer menu** — **THE VISION,
   open.** The household running its own peer should push a button and have the node repair/update
   itself. The button calls a node-repair trigger; the same primitive runs.

## What the human-facilitated button needs (the open work)
- **A node-repair trigger surface** (storage HTTP / a conductor-admin action the button calls).
  **MUST route through `.claude/skills/p2p-design-gate`** before any route is written — likely a
  Category C operational action (no DHT entry; it mutates local conductor state), but classify it
  properly; answer "what coordinator/signal, what address strategy, what HTTP route LAST" per the
  gate. Auth: only the node's authenticated steward may trigger it (it RE-KEYS / re-genesises).
- **A peer-menu UI affordance** — an "Update / Repair node" control on the peer/node-management
  surface (elohim-app imagodei peer menu and/or doorway-app operator surface), with a clear
  confirm: it re-keys a re-seedable node; for a lineage-bearing node it must MIGRATE, not wipe.
- **The lineage policy / migrate-vs-wipe split** — re-seedable node → re-genesis (re-key OK);
  lineage-bearing node → lineage-preserving migration (key rotation with a cross-signed
  `AgentPeerBinding` proof, [[backlog-agent-peer-binding-cross-signed-proof]] — currently
  blocked/unsigned). The button's behavior is policy-derived, not a blind wipe.
- **Story-first:** write the a2o scenario for "a steward repairs/updates their peer from the menu"
  before implementing (per the repo's story-first default).

## Severity / sequencing
HIGH (it's the sovereign-recovery story, and the incident proved the operator-driven path is
fragile + manual), but NOT the incident's immediate fix — mode 2 (autonomous boot repair) ships
now; the floating-tag pin ([[backlog-alpha-conductor-cellwithoutgenesis-floating-happ-tag]] 1a) closes
recurrence. This seed grows into a proper spec (p2p-design-gate + a2o) on a calmer day.
