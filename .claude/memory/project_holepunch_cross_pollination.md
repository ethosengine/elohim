---
index: false
id: project-holepunch-cross-pollination
name: holepunch-cross-pollination
title: Holepunch cross-pollination (p2p data-plane survey)
description: "Borrow Holepunch/Hypercore transport patterns for the p2p data plane; reject anything truth-shaped — Hypercore proves integrity, only the DHT validates."
metadata:
  node_type: memory
  type: project
  originSessionId: bd61680f-6873-4e7f-8ed3-3a85294dc882
---

[Holepunch](https://github.com/holepunchto) (the Hypercore stack + Pear runtime; Tether-backed) is the closest external mirror of the protocol's **p2p SUBSTRATE** — data plane (§3.10), NAT/transport, confidentiality (§3.13). NOT the doorway/federation seam ([[feedback_p2p_vs_federation_layer_vocabulary]]). Full org-wide, gate-adjudicated, seam-routed survey: `genesis/research/holepunch-p2p-dataplane-cross-pollination-2026-06-24.md` (advances PAST the single-primitive note `steward/node/research/hypercore-holepunch-prior-art.md` + the fediverse-lens [[project_prod_main_lag_vs_alpha_dev]]-adjacent Distributed Press survey). Read the doc for detail; this is the re-loadable residue.

**The one sentence:** *Holepunch trusts the writer's key; Elohim trusts the network's validation — integrity is what a Hypercore proves, validity is what only the DHT can.* It independently rebuilt the entire Elohim data plane — which is EXACTLY why the integrity-vs-validity line matters most here. **Alignment is the seduction, not the safety.** (Hypha analog: care-weighted council vs capital-weighted chain.)

**THE ONE HARD RULE (integrity-boundary invariant):** any Holepunch piece may MOVE/LOCATE bytes (data plane) but never DECIDE whether bytes are TRUE (truth plane). Adopt the transport, never the attestation. Classify Class C (operational) or B/B2 (agent-scoped projection); mint **NO new DHT entry type**. Autobase's order-quorum, HyperDHT's BEP44 records, and Hypercore's multisig all fail this (authorization/integrity, never network-validated) → stay strictly data-plane. Pattern-borrow only; never a JS-runtime port (substrate is Rust: Automerge + libp2p 0.54.1 + iroh 0.92).

**TOP-3 highest-leverage borrows (each targets a LIVE gap, all Class C, no mint):**
1. **Distributed-introducer signaling** (HyperDHT "every node is an introducer") → retires the single dedicated **SBD signal-relay SPOF** (`wss://signal.doorway-alpha`, the one WAN-operational plane = tx5/conductor); harvest any swarm peer's AutoNAT observation as DCUtR rendezvous. Unblocks WAN-NAT Gap A (relay/DCUtR built-but-unwired, bootstrap pinned to svc.cluster.local).
2. **Per-block verified streaming + byte-range fetch** (Hypercore proof()/Hyperblobs) → `blob_fetch::race_fetch` verifies whole-blob sha256 only; add per-block Merkle proof + seek-without-full-pull.
3. **XOR-distance mirror-selection as placement policy** (blind-peering) → fills the DEAD fan-out (`parity_shard_count:0` hardcoded; live "inventory count=3430, blob bytes=0 after 36h"); placement intent is the existing `custody-blob` Mishpat commitment (class A, [[project_rea_compute_commitment_primitive]]), the copy is C.

**DEFER:** operational blind-hosting (encrypt-then-shard PROVEN in `private_replica.rs` tests but blocked on **ed25519→X25519** key-conversion substrate, R6, weave Wave C; `KeyEnvelope` is named-but-undesigned → must pass p2p-design-gate, B2, NOT pre-cleared); UDX (have iroh-QUIC); Bare mobile (Tauri-mobile is the route); Autobase (Automerge already commutative-wired). **REJECT:** keypair-as-identity-apex ([[feedback-identity-sovereignty-ontology-guard]]); Tether token-economy gravity (DHT-is-notary-not-bank); discoveryKey-as-federation-primitive.

**Corrections the adversarial pass caught (don't repeat):** the grounders' "`sync_manager` is None / no first writer" was REFUTED — SyncManager IS constructed (main.rs:1996), write route POST /sync/v1/.../changes exists; the real gap = no frontend Automerge client + doorway intentionally excludes /sync/* from its client route manifest. Hypercore is NOT strictly single-writer (multisig quorum manifests exist) but quorum=authorization≠validation. protomux does NOT preserve message boundaries (secret-stream's framing does). HyperDHT adds a BEP44 sig layer (MORE than bare libp2p Kademlia, still integrity-only). Sibling substrate fellow-traveler; cross-pollination cousin to [[project_hypha_dao_cross_pollination]].
