---
name: weave-epic-arc
title: The Weave Epic arc
description: "Weave lens seeds 4 subsystems (VSM recursion, tier-capability, compute contracts, replica encryption); COMPOSE-don't-fork — only new DHT entry is KeyEnvelope"
metadata: 
  node_type: memory
  type: project
  originSessionId: 814f0995-0478-402e-89e7-00813f34980d
---

The operational-weave facing lens (`genesis/docs/superpowers/specs/2026-06-19-operational-weave-facing-lens-design.md`, domain D5, the per-cluster capacity *eyes*) seeds an epic of FOUR downstream subsystems. The arc index is `genesis/docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md` (committed 8d51715f9); the Wave-A lens plan is `genesis/docs/superpowers/plans/2026-06-20-operational-weave-lens-plan.md` (35aaa6fbd, 17 task-items, Slice 1 = DB-free `placement_gap_count` proof gate).

**The load-bearing finding (verified, cost a 5-agent P2P-gated workflow to derive): this is COMPOSE-don't-fork. Across all four subsystems the ONLY genuinely-new DHT entry type is #4's `KeyEnvelope`.** Where each composes:
- **#1 Recursive aggregation (VSM councils / Stafford Beer):** `CoverageRollup` is ALREADY built + lib-wired + UNCONSUMED at `elohim/elohim-storage/src/recursion.rs` (`rollup` :216, BLAKE3 :258, `descend()` :273; N=1 tests pass). Council = a `{"kind":"council"}` charter value on the EXISTING imagodei `Collective` entry (`kind` is JSON in `charter:String`, not a typed field) — NOT a new entry type. Composes into `2026-06-14-recursive-architecture-design.md` §2.1.
- **#2 Tier-capability registry:** tiered-quilt §6 already NAMES it `storage-capability` (earned-witnessed). Measured-only = Operational-C fold, zero new tables/entries. Earned = elohim-DNA `attestation:storage-capability` Content SUBTYPE-STRING (the imagodei `Attestation` entry was REMOVED in Stage C.2 — attestations ride elohim `Content` as `attestation:*` now).
- **#3 REA compute contracts:** the EXISTING Mishpat `Commitment` `delegates-compute` action (`delegates-compute.schema.json` ships) + a new `compute-fulfilled` action on the EXISTING EconomicEvent; reward = existing `appreciation`. Deterministic compute ⇒ output CID IS the proof (recompute → CID match, no new crypto). cid=entry_hash (see [[project_mishpat_commitment_cid_is_entry_hash]]).
- **#4 Private-replica encryption:** blob plane is PLAINTEXT today (`BlobStore::store` → `fs::write`); reuse the `sealed_against_self.rs` dryoc X25519 seal. Only NEW entry type = `KeyEnvelope` (per-READER sealed DEK; custodians get no key). Live path BLOCKED on the conductor-leak fix + an X25519 reader-key substrate (Holochain keys are ed25519).

Forks are documented as per-`/plan` decision points (operator chose to scope, not resolve). Binding constraints honored throughout: DHT-is-notary (capacity/rollup = gossip+projection, never a DHT entry), `agent_cid` is the sole join key (the unsigned `AgentPeerBinding` must NOT be used for economic attribution yet). Also corrected a stale claim in [[resilience-snapshot-humans-junction]]: provide rows ARE seeded outside test_util now (`seed-provide-rows.ts`, CI-wired).
