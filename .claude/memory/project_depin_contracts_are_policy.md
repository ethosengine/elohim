---
name: DePIN contracts are policy (DHT); libp2p is mechanism
description: Stewardship contracts between family/remote-family peers are DHT-notarized (REA commitments); libp2p dataplane operates within those contract bounds
type: project
originSessionId: 17546f03-3ee8-4704-bdf9-18d0d64baf9b
---
DePIN-style contracts for stewarded compute and storage between family and remote-family peers are DHT-notarized. The libp2p dataplane (shard distribution, verification, reconstruction) operates **within** the bounds those contracts establish.

**Why:** Contracts are integrity-load-bearing (who promised what storage/compute to whom, under what SLA, for what duration) — the DHT is the right home for them. Distribution mechanics (which shard to which peer, when to verify, how to reconstruct) are high-frequency operational work — libp2p is the right home for them. Policy/mechanism separation keeps DHT noise down (per `project_dht_vs_libp2p_scoping`) while preserving protocol-integrity claims.

**How to apply:**
- Contracts live as REA-style commitments on DHT (use existing entry types — `rea_commitments` projection already exists in elohim-storage).
- Distribution/verification/reconstruction loops READ contract state to know their bounds (storage budget, diversity requirements, stewardship agreement scope). They never create new contract entities.
- Fulfillment attestations (rung-e "I am holding shard X per commitment Y") are B2 on the existing imagodei Attestation entry type — they reference contracts, don't replace them.
- When designing dataplane features, ask: "what contract bounds this operation?" Never propose a DHT entity for operational state that a contract already governs.
